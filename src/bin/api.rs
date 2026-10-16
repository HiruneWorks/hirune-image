use hirune_image::{
    bootstrap::bootstrap, config::Settings, http::build_router, observability, runtime,
};
use tokio_util::sync::CancellationToken;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::initialize()?;
    let settings = Settings::from_environment()?;
    let app = bootstrap(settings).await?;
    let listener = tokio::net::TcpListener::bind(app.settings.bind).await?;
    tracing::info!(address=%listener.local_addr()?,"API listening");
    let shutdown = CancellationToken::new();
    let watch = tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            let _ = runtime::shutdown_signal().await;
            shutdown.cancel();
        }
    });
    let result = runtime::serve(listener, build_router(app.clone()), app, shutdown).await;
    watch.abort();
    result
}
