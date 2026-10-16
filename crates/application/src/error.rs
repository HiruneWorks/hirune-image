use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq, thiserror::Error,
)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingError {
    #[error("The request is invalid.")]
    InvalidRequest,
    #[error("The recipe is invalid.")]
    InvalidRecipe,
    #[error("The request exceeds the byte limit.")]
    RequestTooLarge,
    #[error("The image exceeds processing limits.")]
    ImageLimitsExceeded,
    #[error("The outputs exceed processing limits.")]
    OutputLimitsExceeded,
    #[error("The image format is unsupported.")]
    UnsupportedFormat,
    #[error("The image is invalid.")]
    InvalidImage,
    #[error("The color profile is unsupported.")]
    UnsupportedColorProfile,
    #[error("Animation is not allowed.")]
    AnimationNotAllowed,
    #[error("The animation cannot be represented.")]
    UnrepresentableAnimation,
    #[error("The processor is busy.")]
    ProcessorOverloaded,
    #[error("The processor is unavailable.")]
    ProcessorUnavailable,
    #[error("Processing timed out.")]
    ProcessingTimeout,
    #[error("An internal error occurred.")]
    InternalError,
}
pub type Result<T> = std::result::Result<T, ProcessingError>;
