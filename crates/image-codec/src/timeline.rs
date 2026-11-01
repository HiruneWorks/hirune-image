use application::{
    error::{ProcessingError as Error, Result},
    images::{MAX_CYCLE_DURATION_MS, MAX_FRAME_DURATION_MS},
};

#[derive(Default)]
pub struct Timeline {
    elapsed_ms: f64,
    compensation: f64,
    rounded_ms: u32,
}
impl Timeline {
    pub fn push(&mut self, numerator: u64, denominator: u64) -> Result<u32> {
        if numerator == 0 || denominator == 0 {
            return Err(Error::UnrepresentableAnimation);
        }
        // Compensated cumulative rounding limits cycle drift instead of rounding each frame independently.
        let increment = numerator as f64 / denominator as f64 * 1000.0;
        let adjusted = increment - self.compensation;
        let next = self.elapsed_ms + adjusted;
        self.compensation = (next - self.elapsed_ms) - adjusted;
        self.elapsed_ms = next;
        if !next.is_finite() || next > f64::from(MAX_CYCLE_DURATION_MS) {
            return Err(Error::ImageLimitsExceeded);
        }
        let rounded = next.round() as u32;
        let delay = rounded
            .checked_sub(self.rounded_ms)
            .ok_or(Error::UnrepresentableAnimation)?;
        if delay == 0 || delay > MAX_FRAME_DURATION_MS {
            return Err(Error::UnrepresentableAnimation);
        }
        self.rounded_ms = rounded;
        Ok(delay)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_timing_keeps_cycle_length_without_dropping_frames() {
        let mut clock = Timeline::default();
        let delays: Vec<_> = (0..60).map(|_| clock.push(1, 60).unwrap()).collect();
        assert_eq!(delays.iter().sum::<u32>(), 1000);
        assert!(delays.iter().all(|d| *d == 16 || *d == 17));
        assert_eq!(clock.push(0, 1), Err(Error::UnrepresentableAnimation));
    }
}
