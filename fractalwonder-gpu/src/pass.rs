// fractalwonder-gpu/src/pass.rs

/// Adam7 progressive rendering pass (1-7).
///
/// Replaces the old resolution-based Pass system. Each pass computes a subset
/// of pixels at full resolution, with each pass doubling the pixel count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adam7Pass(u8);

impl Adam7Pass {
    /// Create a new Adam7Pass. Panics if step is not 0-7.
    /// Step 0 means "compute all pixels" (no Adam7 interlacing).
    /// Steps 1-7 are the Adam7 interlacing passes.
    pub fn new(step: u8) -> Self {
        assert!(step <= 7, "Adam7 step must be 0-7, got {step}");
        Self(step)
    }

    /// Create an Adam7Pass that computes all pixels (no interlacing).
    pub fn all_pixels() -> Self {
        Self(0)
    }

    /// Returns all 7 passes in order.
    pub fn all() -> [Adam7Pass; 7] {
        [1, 2, 3, 4, 5, 6, 7].map(Adam7Pass)
    }

    /// Returns the step number (1-7).
    pub fn step(&self) -> u8 {
        self.0
    }

    /// Returns true if this is the final pass (step 7).
    pub fn is_final(&self) -> bool {
        self.0 == 7
    }

    /// Cumulative pixel percentage after this pass completes.
    pub fn cumulative_percent(&self) -> f32 {
        match self.0 {
            1 => 1.5625,
            2 => 3.125,
            3 => 6.25,
            4 => 12.5,
            5 => 25.0,
            6 => 50.0,
            7 => 100.0,
            _ => 0.0,
        }
    }

    /// Pixels computed in this pass as a fraction (for progress display).
    pub fn pass_fraction(&self) -> f32 {
        match self.0 {
            1 => 1.0 / 64.0,
            2 => 1.0 / 64.0,
            3 => 2.0 / 64.0,
            4 => 4.0 / 64.0,
            5 => 8.0 / 64.0,
            6 => 16.0 / 64.0,
            7 => 32.0 / 64.0,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_passes() {
        let passes = Adam7Pass::all();
        assert_eq!(passes.len(), 7);
        assert_eq!(passes[0].step(), 1);
        assert_eq!(passes[6].step(), 7);
    }

    #[test]
    fn test_is_final() {
        assert!(!Adam7Pass::new(1).is_final());
        assert!(!Adam7Pass::new(6).is_final());
        assert!(Adam7Pass::new(7).is_final());
    }

    #[test]
    fn test_cumulative_percent() {
        assert!((Adam7Pass::new(1).cumulative_percent() - 1.5625).abs() < 0.001);
        assert!((Adam7Pass::new(7).cumulative_percent() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_fractions_sum_to_one() {
        let total: f32 = Adam7Pass::all().iter().map(|p| p.pass_fraction()).sum();
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_step_zero_computes_all() {
        let pass = Adam7Pass::new(0);
        assert_eq!(pass.step(), 0);
        assert!(!pass.is_final()); // step 0 is not a "final" pass
    }

    #[test]
    fn test_all_pixels_helper() {
        let pass = Adam7Pass::all_pixels();
        assert_eq!(pass.step(), 0);
    }

    #[test]
    #[should_panic(expected = "Adam7 step must be 0-7")]
    fn test_invalid_step_eight() {
        Adam7Pass::new(8);
    }
}
