#![deny(unsafe_op_in_unsafe_fn)]
mod animation;
mod container;
#[cfg(test)]
mod container_tests;
mod heic_dimensions;
mod jpeg_preview;
mod normalize;
mod pipeline;
mod png_color;
mod png_container;
mod resize;
pub mod sandbox;
mod source;
mod source_apng;
mod source_avif;
mod source_heic;
mod source_jxl;
mod source_png;
mod source_static;
mod source_svg;
mod timeline;
mod webp;
pub use normalize::normalize;
pub use pipeline::{process_job, verify_output};
pub use resize::resize_inside;
pub use source::{FrameLayout, decode_source};
pub use webp::encode;
pub use webp::{decode_frames, probe, processor_info};
