use application::error::ProcessingError;
use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorEnvelope {
    pub error: ErrorDescription,
}
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorDescription {
    pub code: ProcessingError,
    pub message: String,
    pub request_id: String,
}
pub struct ApiError {
    pub error: ProcessingError,
    pub request_id: Uuid,
}
pub fn status(error: ProcessingError) -> StatusCode {
    use ProcessingError::*;
    match error {
        InvalidRequest | InvalidRecipe => StatusCode::BAD_REQUEST,
        RequestTooLarge | ImageLimitsExceeded | OutputLimitsExceeded => {
            StatusCode::PAYLOAD_TOO_LARGE
        }
        UnsupportedFormat => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        InvalidImage | UnsupportedColorProfile | AnimationNotAllowed | UnrepresentableAnimation => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        ProcessorOverloaded | ProcessorUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        ProcessingTimeout => StatusCode::GATEWAY_TIMEOUT,
        InternalError => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::info!(request_id=%self.request_id,error_code=?self.error,status=status(self.error).as_u16(),"request rejected");
        let mut response = (
            status(self.error),
            Json(ErrorEnvelope {
                error: ErrorDescription {
                    code: self.error,
                    message: self.error.to_string(),
                    request_id: self.request_id.to_string(),
                },
            }),
        )
            .into_response();
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            "no-store".parse().expect("static header"),
        );
        response.headers_mut().insert(
            "x-request-id",
            self.request_id.to_string().parse().expect("UUID header"),
        );
        if matches!(
            self.error,
            ProcessingError::ProcessorOverloaded | ProcessingError::ProcessorUnavailable
        ) {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, "1".parse().expect("static header"));
        }
        response
    }
}
