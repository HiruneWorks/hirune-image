mod support;
use application::images::Limits;
use std::sync::Arc;
use support::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Duration, Instant},
};

#[tokio::test]
async fn admission_precedes_upload_health_survives_and_disconnect_releases() {
    let server = Server::start(Limits {
        deadline_ms: 2000,
        idle_ms: 1000,
        ..Limits::default()
    })
    .await;
    let mut socket = TcpStream::connect(server.url.trim_start_matches("http://"))
        .await
        .unwrap();
    socket.write_all(b"POST /v1/images/process HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=waiting\r\nContent-Length: 10000\r\n\r\n--waiting\r\n").await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        while server.app.permits.available_permits() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        server
            .client
            .get(format!("{}/health/live", server.url))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        server
            .client
            .get(format!("{}/health/ready", server.url))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    let busy = server
        .process(png(1, 1), &recipe("lossless", "preserve", &[(1, 1)]))
        .await;
    assert_eq!(busy.status(), 503);
    assert_eq!(busy.headers()["retry-after"], "1");
    drop(socket);
    server.released().await;
}
#[tokio::test]
async fn upload_idle_timeout_is_typed_and_cleans_temp() {
    let server = Server::start(Limits {
        deadline_ms: 2000,
        idle_ms: 150,
        ..Limits::default()
    })
    .await;
    let mut socket = TcpStream::connect(server.url.trim_start_matches("http://"))
        .await
        .unwrap();
    socket.write_all(b"POST /v1/images/process HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=waiting\r\nContent-Length: 10000\r\n\r\n--waiting\r\n").await.unwrap();
    let mut bytes = vec![0; 8192];
    let count = tokio::time::timeout(Duration::from_secs(2), socket.read(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    let response = String::from_utf8_lossy(&bytes[..count]);
    assert!(response.starts_with("HTTP/1.1 504"));
    assert!(response.contains("processing_timeout"));
    server.released().await;
}
fn controlled_worker() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("worker.rs");
    std::fs::write(&source,r#"fn main(){let args:Vec<_>=std::env::args().collect();let path=std::path::Path::new(&args[2]);std::fs::write(path.join("child.pid"),std::process::id().to_string()).unwrap();loop{std::thread::sleep(std::time::Duration::from_secs(1));}}"#).unwrap();
    let path = directory.path().join(if cfg!(windows) {
        "controlled.exe"
    } else {
        "controlled"
    });
    assert!(
        std::process::Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
    directory
}
#[tokio::test]
async fn processing_deadline_kills_reaps_then_releases() {
    let worker = controlled_worker();
    let mut server = Server::start(Limits {
        deadline_ms: 400,
        idle_ms: 300,
        ..Limits::default()
    })
    .await;
    let mut settings = (*server.app.settings).clone();
    settings.worker = worker.path().join(if cfg!(windows) {
        "controlled.exe"
    } else {
        "controlled"
    });
    let mut app = hirune_image::bootstrap::bootstrap((*server.app.settings).clone())
        .await
        .unwrap();
    app.settings = Arc::new(settings);
    let temp = tempfile::tempdir().unwrap();
    let mut settings = (*app.settings).clone();
    settings.temp_root = temp.path().to_path_buf();
    app.settings = Arc::new(settings);
    server = Server::from_app(app, temp).await;
    let started = Instant::now();
    let response = server
        .process(png(2, 2), &recipe("lossless", "preserve", &[(2, 2)]))
        .await;
    assert_eq!(response.status(), 504);
    assert!(started.elapsed() < Duration::from_secs(3));
    server.released().await;
}

async fn controlled_server(deadline_ms: u64) -> (Server, tempfile::TempDir) {
    let worker = controlled_worker();
    let base = Server::start(Limits {
        deadline_ms,
        idle_ms: deadline_ms,
        ..Limits::default()
    })
    .await;
    let mut app = hirune_image::bootstrap::bootstrap((*base.app.settings).clone())
        .await
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut settings = (*app.settings).clone();
    settings.temp_root = temp.path().to_path_buf();
    settings.worker = worker.path().join(if cfg!(windows) {
        "controlled.exe"
    } else {
        "controlled"
    });
    app.settings = Arc::new(settings);
    (Server::from_app(app, temp).await, worker)
}
async fn wait_for_child(server: &Server) -> u32 {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            for entry in std::fs::read_dir(server.temp.path()).unwrap().flatten() {
                if let Ok(pid) = std::fs::read_to_string(entry.path().join("child.pid")) {
                    return pid.parse().unwrap();
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn disconnect_during_native_work_cancels_the_child() {
    let (server, _worker) = controlled_server(5000).await;
    let body = raw_multipart(&png(2, 2), &recipe("lossless", "preserve", &[(2, 2)]));
    let mut socket = TcpStream::connect(server.url.trim_start_matches("http://"))
        .await
        .unwrap();
    socket.write_all(format!("POST /v1/images/process HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=strict\r\nContent-Length: {}\r\n\r\n",body.len()).as_bytes()).await.unwrap();
    socket.write_all(&body).await.unwrap();
    let pid = wait_for_child(&server).await;
    drop(socket);
    server.released().await;
    assert_child_gone(pid);
}
#[tokio::test]
async fn shutdown_stops_readiness_and_reaps_active_work() {
    let (server, _worker) = controlled_server(5000).await;
    let client = server.client.clone();
    let url = format!("{}/v1/images/process", server.url);
    let body = raw_multipart(&png(2, 2), &recipe("lossless", "preserve", &[(2, 2)]));
    let request = tokio::spawn(async move {
        client
            .post(url)
            .header("content-type", "multipart/form-data; boundary=strict")
            .body(body)
            .send()
            .await
    });
    let pid = wait_for_child(&server).await;
    server.app.begin_shutdown();
    assert!(!server.app.is_ready());
    server.released().await;
    assert_child_gone(pid);
    let _ = request.await;
}
fn assert_child_gone(pid: u32) {
    #[cfg(target_os = "linux")]
    assert!(
        !std::path::Path::new(&format!("/proc/{pid}")).exists(),
        "child still present or zombie after release"
    );
    #[cfg(not(target_os = "linux"))]
    let _ = pid;
}
#[tokio::test]
async fn slow_tcp_consumer_is_closed_before_admission_is_reused() {
    let server = Server::start(Limits {
        deadline_ms: 8000,
        idle_ms: 300,
        ..Limits::default()
    })
    .await;
    let mut seed = 1u32;
    let image = image::RgbaImage::from_fn(1600, 1600, |_, _| {
        let mut p = [0; 4];
        for c in &mut p[..3] {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *c = (seed >> 24) as u8;
        }
        p[3] = 255;
        image::Rgba(p)
    });
    let mut source = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut source, image::ImageFormat::Png)
        .unwrap();
    let body = raw_multipart(
        &source.into_inner(),
        &recipe("lossless", "preserve", &[(4096, 4096)]),
    );
    let mut socket = TcpStream::connect(server.url.trim_start_matches("http://"))
        .await
        .unwrap();
    socket2::SockRef::from(&socket)
        .set_recv_buffer_size(1024)
        .unwrap();
    socket.write_all(format!("POST /v1/images/process HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=strict\r\nContent-Length: {}\r\n\r\n",body.len()).as_bytes()).await.unwrap();
    socket.write_all(&body).await.unwrap();
    let mut header = Vec::new();
    let mut byte = [0; 1];
    loop {
        socket.read_exact(&mut byte).await.unwrap();
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header.starts_with("HTTP/1.1 200"), "{header}");
    let declared = header
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse::<usize>().unwrap())
        })
        .unwrap();
    assert_eq!(server.app.permits.available_permits(), 0);
    server.released().await;
    let mut delivered = Vec::new();
    let read = tokio::time::timeout(Duration::from_secs(4), socket.read_to_end(&mut delivered))
        .await
        .expect("transport remained open");
    if let Err(error) = read {
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
        ));
    }
    assert!(
        delivered.len() < declared,
        "slow consumer received a complete response unexpectedly"
    );
}
