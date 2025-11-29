/// Progress information for ongoing renders.
///
/// Used by both tiled CPU rendering (steps = tiles) and GPU rendering (steps = passes).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderProgress {
    pub completed_steps: u32,
    pub total_steps: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}

impl RenderProgress {
    /// Create new progress tracker.
    pub fn new(total_steps: u32) -> Self {
        Self {
            completed_steps: 0,
            total_steps,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }

    /// Calculate completion percentage (0.0 to 100.0).
    pub fn percentage(&self) -> f32 {
        if self.total_steps == 0 {
            0.0
        } else {
            (self.completed_steps as f32 / self.total_steps as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_progress_starts_at_zero() {
        let progress = RenderProgress::new(100);
        assert_eq!(progress.completed_steps, 0);
        assert_eq!(progress.total_steps, 100);
        assert!(!progress.is_complete);
    }

    #[test]
    fn percentage_calculation() {
        let mut progress = RenderProgress::new(100);
        progress.completed_steps = 50;
        assert!((progress.percentage() - 50.0).abs() < 0.001);
    }

    #[test]
    fn percentage_zero_steps() {
        let progress = RenderProgress::new(0);
        assert!((progress.percentage() - 0.0).abs() < 0.001);
    }

    #[test]
    fn percentage_complete() {
        let mut progress = RenderProgress::new(64);
        progress.completed_steps = 64;
        assert!((progress.percentage() - 100.0).abs() < 0.001);
    }
}
