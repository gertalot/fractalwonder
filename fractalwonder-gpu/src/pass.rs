// fractalwonder-gpu/src/pass.rs

/// Defines the 4 progressive rendering passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// 1/16 resolution, 1/16 iterations
    Preview16,
    /// 1/8 resolution, 1/8 iterations
    Preview8,
    /// 1/4 resolution, 1/4 iterations
    Preview4,
    /// Full resolution, full iterations
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
        ((canvas_w + s - 1) / s, (canvas_h + s - 1) / s)
    }

    /// Scales max iterations for this pass (floor of 100).
    pub fn scale_iterations(&self, max_iter: u32) -> u32 {
        (max_iter / self.scale()).max(100)
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
    fn test_scale_iterations() {
        assert_eq!(Pass::Preview16.scale_iterations(16000), 1000);
        assert_eq!(Pass::Preview16.scale_iterations(1600), 100);
        assert_eq!(Pass::Preview16.scale_iterations(160), 100); // Floor of 100
        assert_eq!(Pass::Full.scale_iterations(16000), 16000);
    }

    #[test]
    fn test_is_final() {
        assert!(!Pass::Preview16.is_final());
        assert!(!Pass::Preview8.is_final());
        assert!(!Pass::Preview4.is_final());
        assert!(Pass::Full.is_final());
    }
}
