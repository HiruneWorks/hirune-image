use crate::config::Settings;
use application::images::ProcessorInfo;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{process::Command, sync::Semaphore};
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub struct Application {
    pub settings: Arc<Settings>,
    pub permits: Arc<Semaphore>,
    pub processor: ProcessorInfo,
    pub shutdown: CancellationToken,
    ready: Arc<AtomicBool>,
}
pub async fn bootstrap(settings: Settings) -> anyhow::Result<Application> {
    settings.limits.validate()?;
    anyhow::ensure!(
        settings.limits.concurrency <= Semaphore::MAX_PERMITS,
        "concurrency exceeds semaphore capacity"
    );
    anyhow::ensure!(
        settings.worker.is_file(),
        "image worker executable is missing"
    );
    std::fs::create_dir_all(&settings.temp_root)?;
    tempfile::tempfile_in(&settings.temp_root)?;
    let mut command = Command::new(&settings.worker);
    command
        .arg("--probe")
        .arg("--limits")
        .arg(serde_json::to_string(&settings.limits)?)
        .env_clear()
        .kill_on_drop(true);
    let output =
        tokio::time::timeout(std::time::Duration::from_secs(10), command.output()).await??;
    anyhow::ensure!(
        output.status.success() && output.stdout.len() <= 4096,
        "codec probe failed"
    );
    let processor = serde_json::from_slice(&output.stdout)?;
    Ok(Application {
        permits: Arc::new(Semaphore::new(settings.limits.concurrency)),
        settings: Arc::new(settings),
        processor,
        shutdown: CancellationToken::new(),
        ready: Arc::new(AtomicBool::new(true)),
    })
}
impl Application {
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
    pub fn begin_shutdown(&self) {
        self.ready.store(false, Ordering::Release);
        self.shutdown.cancel();
    }
    pub async fn drain(&self) {
        if let Ok(permit) = self
            .permits
            .clone()
            .acquire_many_owned(
                u32::try_from(self.settings.limits.concurrency).expect("validated admission count"),
            )
            .await
        {
            drop(permit)
        }
    }
}
