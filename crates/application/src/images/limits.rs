use super::ImageFacts;
use crate::error::{ProcessingError, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_WEBP_DIMENSION: u32 = 16_383;
pub const MAX_WEBP_FILE_BYTES: u64 = (1u64 << 32) - 2;
pub const MAX_FRAME_DURATION_MS: u32 = 0x00ff_ffff;
pub const MAX_CYCLE_DURATION_MS: u32 = i32::MAX as u32;
const CONTROL_BASE_BYTES: u64 = 64 << 10;

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CodecLimits {
    pub webp_dimension: u32,
    pub webp_file_bytes: u64,
    pub frame_duration_ms: u32,
    pub cycle_duration_ms: u32,
}
impl Default for CodecLimits {
    fn default() -> Self {
        Self {
            webp_dimension: MAX_WEBP_DIMENSION,
            webp_file_bytes: MAX_WEBP_FILE_BYTES,
            frame_duration_ms: MAX_FRAME_DURATION_MS,
            cycle_duration_ms: MAX_CYCLE_DURATION_MS,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
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
    pub child_cpu_seconds: u64,
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
            child_cpu_seconds: 60,
        }
    }
}
impl Limits {
    pub fn validate(&self) -> Result<()> {
        let allocation_limit = isize::MAX as u64;
        let byte_budgets = [
            self.source_bytes,
            self.request_bytes,
            self.recipe_bytes,
            self.decoded_bytes,
            self.working_bytes,
            self.spool_bytes,
            self.output_bytes,
            self.child_memory_bytes,
        ];
        let minimum_request = self
            .source_bytes
            .checked_add(self.recipe_bytes)
            .and_then(|bytes| bytes.checked_add(1024))
            .ok_or(ProcessingError::InvalidRequest)?;
        if byte_budgets
            .iter()
            .any(|&bytes| bytes == 0 || bytes > allocation_limit)
            || self.request_bytes < minimum_request
            || self.input_dimension == 0
            || self.canvas_pixels == 0
            || self.frames == 0
            || self.total_frame_pixels == 0
            || self.duration_ms == 0
            || self.duration_ms > MAX_CYCLE_DURATION_MS
            || self.outputs == 0
            || self.output_dimension == 0
            || self.output_dimension > MAX_WEBP_DIMENSION
            || self.concurrency == 0
            || self.concurrency > u32::MAX as usize
            || self.deadline_ms == 0
            || self.idle_ms == 0
            || self.idle_ms > self.deadline_ms
            || self.child_cpu_seconds == 0
            || self.child_cpu_seconds >= allocation_limit
            || std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(self.deadline_ms))
                .is_none()
        {
            return Err(ProcessingError::InvalidRequest);
        }
        self.job_metadata_bytes()?;
        self.result_metadata_bytes(self.outputs)?;
        Ok(())
    }
    pub fn job_metadata_bytes(&self) -> Result<u64> {
        self.recipe_bytes
            .checked_add(CONTROL_BASE_BYTES)
            .filter(|&bytes| bytes <= isize::MAX as u64)
            .ok_or(ProcessingError::InvalidRequest)
    }
    pub fn result_metadata_bytes(&self, outputs: usize) -> Result<u64> {
        u64::try_from(outputs)
            .ok()
            .and_then(|count| count.checked_mul(1024))
            .and_then(|bytes| bytes.checked_add(CONTROL_BASE_BYTES))
            .filter(|&bytes| bytes <= isize::MAX as u64)
            .ok_or(ProcessingError::InvalidRequest)
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
