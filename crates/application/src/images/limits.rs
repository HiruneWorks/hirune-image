use super::ImageFacts;
use crate::error::{ProcessingError, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub source_bytes: u64,
    pub request_bytes: u64,
    pub recipe_bytes: u64,
    pub input_dimension: u32,
    pub canvas_pixels: u64,
    pub frames: u32,
    pub total_frame_pixels: u64,
    pub decoded_bytes: u64,
    pub working_bytes: u64,
    pub spool_bytes: u64,
    pub duration_ms: u32,
    pub outputs: usize,
    pub output_bytes: u64,
    pub output_dimension: u32,
    pub concurrency: usize,
    pub deadline_ms: u64,
    pub idle_ms: u64,
    pub child_memory_bytes: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            source_bytes: 32 << 20,
            request_bytes: 33 << 20,
            recipe_bytes: 16 << 10,
            input_dimension: 8192,
            canvas_pixels: 40_000_000,
            frames: 120,
            total_frame_pixels: 100_000_000,
            decoded_bytes: 256 << 20,
            working_bytes: 256 << 20,
            spool_bytes: 256 << 20,
            duration_ms: 60_000,
            outputs: 4,
            output_bytes: 32 << 20,
            output_dimension: 4096,
            concurrency: 1,
            deadline_ms: 60_000,
            idle_ms: 10_000,
            child_memory_bytes: 768 << 20,
        }
    }
}
impl Limits {
    pub fn validate(&self) -> Result<()> {
        let m = Self::default();
        if self.source_bytes == 0
            || self.source_bytes > m.source_bytes
            || self.request_bytes
                < self
                    .source_bytes
                    .saturating_add(self.recipe_bytes)
                    .saturating_add(1024)
            || self.request_bytes > m.request_bytes
            || self.recipe_bytes == 0
            || self.recipe_bytes > m.recipe_bytes
            || self.input_dimension == 0
            || self.input_dimension > m.input_dimension
            || self.canvas_pixels == 0
            || self.canvas_pixels > m.canvas_pixels
            || self.frames == 0
            || self.frames > m.frames
            || self.total_frame_pixels == 0
            || self.total_frame_pixels > m.total_frame_pixels
            || self.decoded_bytes == 0
            || self.decoded_bytes > m.decoded_bytes
            || self.working_bytes == 0
            || self.working_bytes > m.working_bytes
            || self.spool_bytes == 0
            || self.spool_bytes > m.spool_bytes
            || self.duration_ms == 0
            || self.duration_ms > m.duration_ms
            || self.outputs == 0
            || self.outputs > 4
            || self.output_bytes == 0
            || self.output_bytes > m.output_bytes
            || self.output_dimension == 0
            || self.output_dimension > m.output_dimension
            || self.concurrency == 0
            || self.concurrency > 4
            || self.deadline_ms == 0
            || self.deadline_ms > m.deadline_ms
            || self.idle_ms == 0
            || self.idle_ms > self.deadline_ms
            || self.child_memory_bytes < 256 << 20
            || self.child_memory_bytes > m.child_memory_bytes
        {
            return Err(ProcessingError::InvalidRequest);
        }
        Ok(())
    }
    pub fn canvas(&self, width: u32, height: u32) -> Result<u64> {
        let pixels = u64::from(width)
            .checked_mul(u64::from(height))
            .ok_or(ProcessingError::ImageLimitsExceeded)?;
        if width == 0
            || height == 0
            || width > self.input_dimension
            || height > self.input_dimension
            || pixels > self.canvas_pixels
        {
            return Err(ProcessingError::ImageLimitsExceeded);
        }
        Ok(pixels)
    }
    pub fn preflight(&self, facts: &ImageFacts) -> Result<()> {
        let pixels = self.canvas(facts.width, facts.height)?;
        let total = pixels
            .checked_mul(u64::from(facts.frame_count))
            .ok_or(ProcessingError::ImageLimitsExceeded)?;
        let bytes = total
            .checked_mul(4)
            .ok_or(ProcessingError::ImageLimitsExceeded)?;
        if facts.frame_count == 0
            || facts.frame_count > self.frames
            || total > self.total_frame_pixels
            || bytes > self.decoded_bytes
            || bytes > self.spool_bytes
            || facts.duration_ms > self.duration_ms
            || pixels
                .checked_mul(20)
                .ok_or(ProcessingError::ImageLimitsExceeded)?
                > self.working_bytes
        {
            return Err(ProcessingError::ImageLimitsExceeded);
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct Budget {
    pub frames: u32,
    pub pixels: u64,
    pub bytes: u64,
    pub duration_ms: u32,
}
impl Budget {
    pub fn frame(&mut self, width: u32, height: u32, delay: u32, limits: &Limits) -> Result<()> {
        let error = ProcessingError::ImageLimitsExceeded;
        let pixels = limits.canvas(width, height)?;
        self.frames = self.frames.checked_add(1).ok_or(error)?;
        self.pixels = self.pixels.checked_add(pixels).ok_or(error)?;
        self.bytes = self
            .bytes
            .checked_add(pixels.checked_mul(4).ok_or(error)?)
            .ok_or(error)?;
        self.duration_ms = self.duration_ms.checked_add(delay).ok_or(error)?;
        if self.frames > limits.frames
            || self.pixels > limits.total_frame_pixels
            || self.bytes > limits.decoded_bytes
            || self.bytes > limits.spool_bytes
            || self.duration_ms > limits.duration_ms
        {
            return Err(error);
        }
        Ok(())
    }
}
