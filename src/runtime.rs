use crate::{
    bootstrap::Application,
    transport::{BoundedListener, ConnectionInfo},
};
use axum::Router;
use std::future::IntoFuture;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
pub async fn serve(
    listener: TcpListener,
    router: Router,
    app: Application,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let shutdown_app = app.clone();
    let server = axum::serve(
        BoundedListener {
            listener,
            shutdown: app.shutdown.clone(),
        },
        router.into_make_service_with_connect_info::<ConnectionInfo>(),
    )
    .with_graceful_shutdown(async move {
        shutdown.cancelled().await;
        shutdown_app.begin_shutdown()
    })
    .into_future();
    server.await?;
    app.drain().await;
    Ok(())
}
pub async fn shutdown_signal() -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let mut termination =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {result=tokio::signal::ctrl_c()=>result?,_=termination.recv()=>{}}
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await?;
    Ok(())
}
