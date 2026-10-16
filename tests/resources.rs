#![cfg(target_os = "linux")]
mod support;
use application::images::Limits;
use std::{sync::Arc, time::Duration};
use support::*;

#[tokio::test]
async fn actual_native_memory_and_cpu_fences_reap_and_recover() {
    let base = Server::start(Limits {
        child_memory_bytes: 128 << 20,
        child_cpu_seconds: 1,
        deadline_ms: 8000,
        idle_ms: 8000,
        ..Limits::default()
    })
    .await;
    let mut app = base.app.clone();
    let mut settings = (*app.settings).clone();
    let temp = tempfile::tempdir().unwrap();
    settings.temp_root = temp.path().to_path_buf();
    settings.worker = env!("CARGO_BIN_EXE_resource_test_worker").into();
    app.settings = Arc::new(settings);
    let server = Server::from_app(app, temp).await;
    for mode in ["memory", "cpu"] {
        let job_recipe = recipe("lossless", "preserve", &[(2, 2)]).replace("variant_0", mode);
        let client = server.client.clone();
        let url = format!("{}/v1/images/process", server.url);
        let body = raw_multipart(&png(2, 2), &job_recipe);
        let request = tokio::spawn(async move {
            client
                .post(url)
                .header("content-type", "multipart/form-data; boundary=strict")
                .body(body)
                .send()
                .await
                .unwrap()
        });
        let pid: u32 = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                for entry in std::fs::read_dir(server.temp.path()).unwrap().flatten() {
                    if let Ok(pid) = std::fs::read_to_string(entry.path().join("child.pid")) {
                        return pid.parse().unwrap();
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        let response = request.await.unwrap();
        assert_eq!(response.status(), 503);
        let error: serde_json::Value = response.json().await.unwrap();
        assert_eq!(error["error"]["code"], "processor_unavailable");
        server.released().await;
        assert!(
            !std::path::Path::new(&format!("/proc/{pid}")).exists(),
            "child still present or zombie after admission release"
        );
        let recipe = recipe("lossless", "preserve", &[(2, 2)]);
        let source = png(2, 2);
        verify(
            server.process(source.clone(), &recipe).await,
            &source,
            &recipe,
        )
        .await;
        server.released().await;
    }
}
