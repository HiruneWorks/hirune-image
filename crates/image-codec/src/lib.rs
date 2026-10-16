#![deny(unsafe_op_in_unsafe_fn)]
mod container;
#[cfg(test)]
mod container_tests;
mod normalize;
mod pipeline;
mod resize;
pub mod sandbox;
mod source;
mod webp;
pub use normalize::normalize;
pub use pipeline::{process_job, verify_output};
pub use resize::resize_inside;
pub use source::decode_source;
pub use webp::encode;
pub use webp::{decode_frames, probe, processor_info};
