use crate::{
    bootstrap::Application,
    execution::{self, JobLease},
    http::{delivery, error::ApiError, upload},
    transport::ConnectionInfo,
};
use application::error::ProcessingError as Error;
use axum::{
    extract::{ConnectInfo, Request, State},
    response::Response,
};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
#[tracing::instrument(skip_all,fields(request_id=tracing::field::Empty))]
pub async fn process_image(
    State(app): State<Application>,
    request: Request,
) -> Result<Response, ApiError> {
    let request_id = Uuid::now_v7();
    tracing::Span::current().record("request_id", request_id.to_string());
    let failure = |error| ApiError { error, request_id };
    let connection = request
        .extensions()
        .get::<ConnectInfo<ConnectionInfo>>()
        .map(|info| info.0.connection.clone());
    let disconnected = connection
        .as_ref()
        .map_or_else(CancellationToken::new, |c| c.closed());
    if !app.is_ready() {
        return Err(failure(Error::ProcessorUnavailable));
    }
    let permit = app
        .permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| failure(Error::ProcessorOverloaded))?;
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(app.settings.limits.deadline_ms))
        .ok_or_else(|| failure(Error::InternalError))?;
    let directory = tempfile::Builder::new()
        .prefix("hirune-image-")
        .tempdir_in(&app.settings.temp_root)
        .map_err(|_| failure(Error::ProcessorUnavailable))?;
    let lease = JobLease { directory, permit };
    let upload = tokio::select! {
        result=tokio::time::timeout_at(deadline,upload::read_upload(request,&app,deadline))=>result.map_err(|_|failure(Error::ProcessingTimeout))?.map_err(failure)?,
        _=app.shutdown.cancelled()=>return Err(failure(Error::ProcessorUnavailable)),
    };
    let prepared = execution::run_job(
        app.clone(),
        lease,
        upload.job,
        upload.source,
        deadline,
        disconnected,
    )
    .await
    .map_err(failure)?;
    delivery::response(
        prepared,
        request_id,
        deadline,
        app.settings.limits.idle_ms,
        app.shutdown,
        connection,
    )
    .map_err(failure)
}
