pub mod capabilities;
pub mod images;
use crate::bootstrap::Application;
use axum::{Router, routing::get};
pub fn routes() -> Router<Application> {
    Router::new()
        .route("/capabilities", get(capabilities::capabilities))
        .merge(images::routes())
}
