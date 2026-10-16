use crate::bootstrap::Application;
use axum::{Router, routing::post};
pub fn routes() -> Router<Application> {
    Router::new().route("/images/process", post(super::process::process_image))
}
