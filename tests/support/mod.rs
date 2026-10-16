#![allow(dead_code)]
use application::images::{Limits, Manifest};
use bytes::Bytes;
use futures_util::stream;
use hirune_image::{
    bootstrap::{Application, bootstrap},
    config::Settings,
    http::build_router,
    runtime,
};
use reqwest::{
    Client,
    multipart::{Form, Part},
};
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

pub struct Server {
    pub url: String,
    pub app: Application,
    pub temp: TempDir,
    pub client: Client,
    pub shutdown: CancellationToken,
}
impl Server {
    pub async fn start(limits: Limits) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let app = bootstrap(Settings {
            bind: "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            limits,
            worker: PathBuf::from(env!("CARGO_BIN_EXE_image_worker")),
            temp_root: temp.path().to_path_buf(),
        })
        .await
        .unwrap();
        Self::from_app(app, temp).await
    }
    pub async fn from_app(app: Application, temp: TempDir) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let shutdown = CancellationToken::new();
        tokio::spawn(runtime::serve(
            listener,
            build_router(app.clone()),
            app.clone(),
            shutdown.clone(),
        ));
        Self {
            url,
            app,
            temp,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            shutdown,
        }
    }
    pub async fn process(&self, bytes: Vec<u8>, recipe: &str) -> reqwest::Response {
        self.client
            .post(format!("{}/v1/images/process", self.url))
            .multipart(
                Form::new()
                    .part(
                        "file",
                        Part::bytes(bytes)
                            .file_name("../../untrusted.png")
                            .mime_str("application/octet-stream")
                            .unwrap(),
                    )
                    .part(
                        "recipe",
                        Part::bytes(recipe.as_bytes().to_vec())
                            .mime_str("application/json")
                            .unwrap(),
                    ),
            )
            .send()
            .await
            .unwrap()
    }
    pub async fn released(&self) {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if self.app.permits.available_permits() == self.app.settings.limits.concurrency {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("admission was not released");
        assert_eq!(
            std::fs::read_dir(self.temp.path()).unwrap().count(),
            0,
            "temporary files leaked"
        );
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown.cancel();
        self.app.begin_shutdown();
    }
}
pub fn recipe(mode: &str, policy: &str, sizes: &[(u32, u32)]) -> String {
    serde_json::json!({"contract_version":1,"animation_policy":policy,"outputs":sizes.iter().enumerate().map(|(i,(w,h))|{
        let mut encoding=serde_json::json!({"format":"webp","mode":mode,"effort":0});
        if mode=="lossy"{encoding["quality"]=serde_json::json!(85);}
        serde_json::json!({"id":format!("variant_{i}"),"resize":{"fit":"inside","max_width":w,"max_height":h},"encoding":encoding})
    }).collect::<Vec<_>>()} ).to_string()
}
pub fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_fn(width, height, |x, y| {
        image::Rgba([
            (x % 251) as u8,
            (y % 251) as u8,
            93,
            if x == 0 { 0 } else { 255 },
        ])
    });
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    output.into_inner()
}
pub async fn verify(
    response: reqwest::Response,
    source: &[u8],
    recipe: &str,
) -> (Manifest, Vec<Vec<u8>>) {
    assert_eq!(
        response.status(),
        200,
        "{}",
        response.text().await.unwrap_or_default()
    );
    let content_type = response.headers()["content-type"]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(content_type.starts_with("multipart/related;"));
    let boundary = content_type
        .split("boundary=")
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let bytes = response.bytes().await.unwrap();
    let mut multipart = multer::Multipart::new(
        stream::once(async { Ok::<Bytes, std::io::Error>(bytes) }),
        boundary,
    );
    let first = multipart.next_field().await.unwrap().unwrap();
    assert_eq!(first.headers()["content-id"], "<manifest>");
    let manifest: Manifest = serde_json::from_slice(&first.bytes().await.unwrap()).unwrap();
    assert_eq!(manifest.source_sha256, application::images::sha256(source));
    assert_eq!(
        manifest.recipe_sha256,
        application::images::sha256(recipe.as_bytes())
    );
    let mut outputs = Vec::new();
    for expected in &manifest.outputs {
        let part = multipart.next_field().await.unwrap().unwrap();
        assert_eq!(
            part.headers()["content-id"],
            format!("<output-{}>", expected.id)
        );
        assert_eq!(part.headers()["content-type"], "image/webp");
        assert_eq!(
            part.headers()["content-length"]
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            expected.byte_size
        );
        let bytes = part.bytes().await.unwrap();
        assert_eq!(bytes.len() as u64, expected.byte_size);
        assert_eq!(application::images::sha256(&bytes), expected.sha256);
        assert_eq!(
            image_codec::verify_output(&bytes, &Limits::default()).unwrap(),
            expected.image
        );
        outputs.push(bytes.to_vec());
    }
    assert!(multipart.next_field().await.unwrap().is_none());
    (manifest, outputs)
}
pub fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

pub fn raw_multipart(source: &[u8], recipe: &str) -> Vec<u8> {
    let mut body = b"--strict\r\nContent-Disposition: form-data; name=\"file\"\r\n\r\n".to_vec();
    body.extend_from_slice(source);
    body.extend_from_slice(b"\r\n--strict\r\nContent-Disposition: form-data; name=\"recipe\"\r\nContent-Type: application/json\r\n\r\n");
    body.extend_from_slice(recipe.as_bytes());
    body.extend_from_slice(b"\r\n--strict--\r\n");
    body
}
