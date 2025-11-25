/// Progress information for ongoing renders.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderProgress {
    pub completed_tiles: u32,
    pub total_tiles: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}

impl RenderProgress {
    /// Create new progress tracker.
    pub fn new(total_tiles: u32) -> Self {
        Self {
            completed_tiles: 0,
            total_tiles,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }

    /// Calculate completion percentage (0.0 to 100.0).
    pub fn percentage(&self) -> f32 {
        if self.total_tiles == 0 {
            0.0
        } else {
            (self.completed_tiles as f32 / self.total_tiles as f32) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_progress_starts_at_zero() {
        let progress = RenderProgress::new(100);
        assert_eq!(progress.completed_tiles, 0);
        assert_eq!(progress.total_tiles, 100);
        assert!(!progress.is_complete);
    }

    #[test]
    fn percentage_calculation() {
        let mut progress = RenderProgress::new(100);
        progress.completed_tiles = 50;
        assert!((progress.percentage() - 50.0).abs() < 0.001);
    }

    #[test]
    fn percentage_zero_tiles() {
        let progress = RenderProgress::new(0);
        assert!((progress.percentage() - 0.0).abs() < 0.001);
    }

    #[test]
    fn percentage_complete() {
        let mut progress = RenderProgress::new(64);
        progress.completed_tiles = 64;
        assert!((progress.percentage() - 100.0).abs() < 0.001);
    }
}
