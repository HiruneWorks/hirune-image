use crate::bootstrap::Application;
use application::{
    error::{ProcessingError as Error, Result},
    images::{JobInput, JobResult, Manifest, inside_dimensions, sha256},
};
use bytes::Bytes;
use std::process::Stdio;
use tempfile::TempDir;
use tokio::{
    process::{Child, Command},
    sync::{OwnedSemaphorePermit, oneshot},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
pub struct JobLease {
    pub directory: TempDir,
    pub permit: OwnedSemaphorePermit,
}
pub struct PreparedResponse {
    pub manifest: Manifest,
    pub outputs: Vec<Bytes>,
    pub lease: JobLease,
}
struct CancelOnDrop(CancellationToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
pub async fn run_job(
    app: Application,
    lease: JobLease,
    job: JobInput,
    source: Bytes,
    deadline: Instant,
    disconnected: CancellationToken,
) -> Result<PreparedResponse> {
    let cancellation = CancellationToken::new();
    let guard = CancelOnDrop(cancellation.clone());
    let (sender, receiver) = oneshot::channel();
    let shutdown = app.shutdown.clone();
    let span = tracing::Span::current();
    tokio::spawn(
        async move {
            let result = supervise(&app, lease, &job, source, deadline, &cancellation).await;
            let _ = sender.send(result);
        }
        .instrument(span),
    );
    // A timed-out file operation stays owned by the supervisor until it completes.
    // Returning the HTTP error does not return its permit early.
    let result = tokio::select! {
        result=receiver=>result.map_err(|_|Error::ProcessorUnavailable)?,
        _=tokio::time::sleep_until(deadline)=>Err(Error::ProcessingTimeout),
        _=shutdown.cancelled()=>Err(Error::ProcessorUnavailable),
        _=disconnected.cancelled()=>Err(Error::ProcessorUnavailable),
    };
    drop(guard);
    result
}
fn check_cancel(deadline: Instant, cancel: &CancellationToken, app: &Application) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(Error::ProcessingTimeout);
    }
    if cancel.is_cancelled() || app.shutdown.is_cancelled() {
        return Err(Error::ProcessorUnavailable);
    }
    Ok(())
}
async fn kill_and_reap(child: &mut Child) {
    let _ = child.start_kill();
    loop {
        match child.wait().await {
            Ok(_) => return,
            Err(error) => {
                tracing::error!(kind=?error.kind(),"child reap failed; retaining admission");
                tokio::time::sleep(Duration::from_millis(20)).await;
                let _ = child.start_kill();
            }
        }
    }
}
async fn supervise(
    app: &Application,
    lease: JobLease,
    job: &JobInput,
    source: Bytes,
    deadline: Instant,
    cancel: &CancellationToken,
) -> Result<PreparedResponse> {
    let directory = lease.directory.path();
    let source_size = source.len() as u64;
    check_cancel(deadline, cancel, app)?;
    tokio::fs::write(directory.join("source"), source)
        .await
        .map_err(|_| Error::InternalError)?;
    check_cancel(deadline, cancel, app)?;
    let metadata = serde_json::to_vec(job).map_err(|_| Error::InternalError)?;
    if metadata.len() as u64 > job.limits.job_metadata_bytes()? {
        return Err(Error::InvalidRequest);
    }
    tokio::fs::write(directory.join("job.json"), metadata)
        .await
        .map_err(|_| Error::InternalError)?;
    check_cancel(deadline, cancel, app)?;
    let mut command = Command::new(&app.settings.worker);
    command
        .arg("--job")
        .arg(directory)
        .arg("--limits")
        .arg(serde_json::to_string(&job.limits).map_err(|_| Error::InternalError)?)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|_| Error::ProcessorUnavailable)?;
    let child_id = child.id().unwrap_or(0);
    tracing::info!(child_id, "image child started");
    let status = tokio::select! {
        status=child.wait()=>match status{Ok(status)=>status,Err(_)=>{kill_and_reap(&mut child).await;return Err(Error::ProcessorUnavailable)}},
        _=tokio::time::sleep_until(deadline)=>{kill_and_reap(&mut child).await;tracing::info!(child_id,reason="deadline","image child reaped");return Err(Error::ProcessingTimeout)}
        _=cancel.cancelled()=>{kill_and_reap(&mut child).await;tracing::info!(child_id,reason="cancel","image child reaped");return Err(Error::ProcessorUnavailable)}
        _=app.shutdown.cancelled()=>{kill_and_reap(&mut child).await;return Err(Error::ProcessorUnavailable)}
    };
    tracing::info!(child_id, success = status.success(), "image child reaped");
    if !status.success() {
        return Err(Error::ProcessorUnavailable);
    }
    check_cancel(deadline, cancel, app)?;
    let result_path = directory.join("result.json");
    if tokio::fs::metadata(&result_path)
        .await
        .map_err(|_| Error::ProcessorUnavailable)?
        .len()
        > job.limits.result_metadata_bytes(job.recipe.outputs.len())?
    {
        return Err(Error::InternalError);
    }
    let result: JobResult = serde_json::from_slice(
        &tokio::fs::read(result_path)
            .await
            .map_err(|_| Error::ProcessorUnavailable)?,
    )
    .map_err(|_| Error::InternalError)?;
    check_cancel(deadline, cancel, app)?;
    let manifest = match result {
        JobResult::Success { manifest } => *manifest,
        JobResult::Failure { error } => return Err(error),
    };
    if manifest.contract_version != 1
        || manifest.processor != app.processor
        || manifest.source_sha256 != job.source_sha256
        || manifest.recipe_sha256 != job.recipe_sha256
        || manifest.outputs.len() != job.recipe.outputs.len()
    {
        return Err(Error::InternalError);
    }
    job.limits
        .preflight(&manifest.source.image)
        .map_err(|_| Error::InternalError)?;
    if manifest.source.byte_size != source_size
        || manifest.source.byte_size > job.limits.source_bytes
        || !(1..=8).contains(&manifest.source.orientation)
    {
        return Err(Error::InternalError);
    }
    let mut outputs = Vec::new();
    let mut total = 0u64;
    for (output, recipe) in manifest.outputs.iter().zip(&job.recipe.outputs) {
        check_cancel(deadline, cancel, app)?;
        let (sw, sh) = if manifest.source.orientation >= 5 {
            (manifest.source.image.height, manifest.source.image.width)
        } else {
            (manifest.source.image.width, manifest.source.image.height)
        };
        if output.id != recipe.id
            || output.media_type != application::images::MediaType::Webp
            || (output.image.width, output.image.height)
                != inside_dimensions(sw, sh, recipe.max_width, recipe.max_height)
        {
            return Err(Error::InternalError);
        }
        let path = directory.join(format!("output-{}.webp", recipe.id));
        let size = tokio::fs::metadata(&path)
            .await
            .map_err(|_| Error::InternalError)?
            .len();
        total = total.checked_add(size).ok_or(Error::OutputLimitsExceeded)?;
        if size != output.byte_size || total > job.limits.output_bytes {
            return Err(Error::InternalError);
        }
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|_| Error::InternalError)?;
        if sha256(&bytes) != output.sha256 {
            return Err(Error::InternalError);
        }
        outputs.push(Bytes::from(bytes));
    }
    check_cancel(deadline, cancel, app)?;
    Ok(PreparedResponse {
        manifest,
        outputs,
        lease,
    })
}
