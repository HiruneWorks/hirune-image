use crate::bootstrap::Application;
use application::images::{CodecLimits, Limits, ProcessorInfo};
use axum::{Json, extract::State};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct Capabilities {
    pub contract_version: u32,
    pub processor: ProcessorInfo,
    pub limits: Limits,
    pub codec_limits: CodecLimits,
    pub input_formats: [&'static str; 14],
    pub animated_input_formats: [&'static str; 5],
    pub output_format: &'static str,
    pub encoding_modes: [&'static str; 2],
    pub animation_policies: [&'static str; 1],
    pub resize_fit: &'static str,
    pub allow_upscale: bool,
    pub zero_delay_policy: &'static str,
    pub timing_policy: &'static str,
    pub color_policy: &'static str,
    pub native_resource_fence: &'static str,
}
pub async fn capabilities(State(app): State<Application>) -> Json<Capabilities> {
    Json(Capabilities {
        contract_version: 1,
        processor: app.processor,
        limits: app.settings.limits.clone(),
        codec_limits: CodecLimits::default(),
        input_formats: [
            "jpeg", "png", "apng", "gif", "webp", "avif", "heic", "bmp", "tiff", "jxl", "svg",
            "ico", "qoi", "pnm",
        ],
        output_format: "webp",
        animated_input_formats: ["gif", "apng", "webp", "avif", "jxl"],
        encoding_modes: ["lossy", "lossless"],
        animation_policies: ["preserve"],
        resize_fit: "inside",
        allow_upscale: false,
        zero_delay_policy: "reject_animated_zero_delay",
        timing_policy: "nearest_cumulative_millisecond; reject_unrepresentable_frames",
        color_policy: "icc_and_cicp_to_srgb8; unprofiled_srgb; reject_hdr",
        native_resource_fence: if cfg!(target_os = "linux") {
            "rlimit_as_and_cpu"
        } else {
            "deadline_only; memory_acceptance_requires_linux"
        },
    })
}
