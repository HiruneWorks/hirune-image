mod delivery;
pub mod error;
mod upload;
pub mod v1;
use crate::bootstrap::Application;
use axum::{Router, extract::State, http::StatusCode, routing::get};
pub fn build_router(app: Application) -> Router {
    Router::new()
        .route("/health/live", get(|| async { StatusCode::NO_CONTENT }))
        .route("/health/ready", get(ready))
        .route(
            "/openapi.json",
            get(|| async { axum::Json(crate::contracts::openapi_document()) }),
        )
        .nest("/v1", v1::routes())
        .with_state(app)
}
async fn ready(State(app): State<Application>) -> StatusCode {
    if app.is_ready() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
