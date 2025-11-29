// fractalwonder-gpu/src/pass.rs

/// Defines the 4 progressive rendering passes.
///
/// Each pass renders at reduced RESOLUTION (fewer pixels) but uses FULL iterations.
/// The speedup comes from fewer pixels, not fewer iterations - reducing iterations
/// would cause incorrect results at deep zooms where escape happens late.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// 1/16 resolution
    Preview16,
    /// 1/8 resolution
    Preview8,
    /// 1/4 resolution
    Preview4,
    /// Full resolution
    Full,
}

impl Pass {
    /// Returns all passes in order.
    pub fn all() -> [Pass; 4] {
        [Pass::Preview16, Pass::Preview8, Pass::Preview4, Pass::Full]
    }

    /// Returns the scale factor (16, 8, 4, or 1).
    pub fn scale(&self) -> u32 {
        match self {
            Pass::Preview16 => 16,
            Pass::Preview8 => 8,
            Pass::Preview4 => 4,
            Pass::Full => 1,
        }
    }

    /// Computes pass dimensions from canvas dimensions.
    pub fn dimensions(&self, canvas_w: u32, canvas_h: u32) -> (u32, u32) {
        let s = self.scale();
        (canvas_w.div_ceil(s), canvas_h.div_ceil(s))
    }

    /// Returns max iterations for this pass.
    ///
    /// All passes use full iterations - the speedup comes from fewer pixels,
    /// not fewer iterations. Reducing iterations causes incorrect results at
    /// deep zooms where escape happens at high iteration counts.
    pub fn max_iterations(&self, max_iter: u32) -> u32 {
        max_iter
    }

    /// Returns true if this is the final (full resolution) pass.
    pub fn is_final(&self) -> bool {
        matches!(self, Pass::Full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensions() {
        assert_eq!(Pass::Preview16.dimensions(3840, 2160), (240, 135));
        assert_eq!(Pass::Preview8.dimensions(3840, 2160), (480, 270));
        assert_eq!(Pass::Preview4.dimensions(3840, 2160), (960, 540));
        assert_eq!(Pass::Full.dimensions(3840, 2160), (3840, 2160));
    }

    #[test]
    fn test_dimensions_rounding() {
        // 1000 / 16 = 62.5, should round up to 63
        assert_eq!(Pass::Preview16.dimensions(1000, 1000), (63, 63));
    }

    #[test]
    fn test_max_iterations() {
        // All passes use full iterations
        assert_eq!(Pass::Preview16.max_iterations(16000), 16000);
        assert_eq!(Pass::Preview8.max_iterations(16000), 16000);
        assert_eq!(Pass::Preview4.max_iterations(16000), 16000);
        assert_eq!(Pass::Full.max_iterations(16000), 16000);
    }

    #[test]
    fn test_is_final() {
        assert!(!Pass::Preview16.is_final());
        assert!(!Pass::Preview8.is_final());
        assert!(!Pass::Preview4.is_final());
        assert!(Pass::Full.is_final());
    }
}
