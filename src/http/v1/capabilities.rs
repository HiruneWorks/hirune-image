use crate::bootstrap::Application;
use application::images::{Limits, ProcessorInfo};
use axum::{Json, extract::State};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct Capabilities {
    pub contract_version: u32,
    pub processor: ProcessorInfo,
    pub limits: Limits,
    pub input_formats: [&'static str; 4],
    pub output_format: &'static str,
    pub encoding_modes: [&'static str; 2],
    pub animation_policies: [&'static str; 3],
    pub resize_fit: &'static str,
    pub allow_upscale: bool,
    pub zero_delay_policy: &'static str,
    pub color_policy: &'static str,
    pub native_resource_fence: &'static str,
}
pub async fn capabilities(State(app): State<Application>) -> Json<Capabilities> {
    Json(Capabilities {
        contract_version: 1,
        processor: app.processor,
        limits: app.settings.limits.clone(),
        input_formats: ["jpeg", "png", "gif", "webp"],
        output_format: "webp",
        encoding_modes: ["lossy", "lossless"],
        animation_policies: ["preserve", "first_frame", "reject"],
        resize_fit: "inside",
        allow_upscale: false,
        zero_delay_policy: "reject_animated_zero_delay",
        color_policy: "rgb_icc_to_srgb; unprofiled_srgb; reject_custom_png_color_and_cmyk",
        native_resource_fence: if cfg!(target_os = "linux") {
            "rlimit_as_and_cpu"
        } else {
            "deadline_only; memory_acceptance_requires_linux"
        },
    })
}
