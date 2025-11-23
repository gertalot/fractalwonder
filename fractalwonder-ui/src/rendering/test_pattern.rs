/// Tick spacing parameters for the ruler test pattern.
/// All values derived from major_spacing.
#[derive(Debug, Clone, PartialEq)]
pub struct TickParams {
    /// Major tick interval (e.g., 1.0 when viewport width ~4)
    pub major_spacing: f64,
    /// Medium tick interval (major / 2)
    pub medium_spacing: f64,
    /// Minor tick interval (major / 10)
    pub minor_spacing: f64,
    /// Threshold for detecting major ticks (major / 50)
    pub major_threshold: f64,
    /// Threshold for detecting medium ticks (major / 75)
    pub medium_threshold: f64,
    /// Threshold for detecting minor ticks (major / 100)
    pub minor_threshold: f64,
    /// Threshold for detecting axis lines (major / 100)
    pub axis_threshold: f64,
}

/// Calculate tick parameters from viewport width.
///
/// Uses log10 to find appropriate scale, then derives all parameters
/// from a single major_spacing value.
pub fn calculate_tick_params(viewport_width_f64: f64) -> TickParams {
    let log_width = viewport_width_f64.log10();
    let major_exp = (log_width - 0.5).floor() as i32;
    let major_spacing = 10.0_f64.powi(major_exp);

    TickParams {
        major_spacing,
        medium_spacing: major_spacing / 2.0,
        minor_spacing: major_spacing / 10.0,
        major_threshold: major_spacing / 50.0,
        medium_threshold: major_spacing / 75.0,
        minor_threshold: major_spacing / 100.0,
        axis_threshold: major_spacing / 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_spacing_at_width_4() {
        let params = calculate_tick_params(4.0);
        assert!((params.major_spacing - 1.0).abs() < 0.001);
        assert!((params.medium_spacing - 0.5).abs() < 0.001);
        assert!((params.minor_spacing - 0.1).abs() < 0.001);
    }

    #[test]
    fn tick_spacing_at_width_0_04() {
        let params = calculate_tick_params(0.04);
        assert!((params.major_spacing - 0.01).abs() < 0.0001);
    }

    #[test]
    fn tick_spacing_at_width_40() {
        let params = calculate_tick_params(40.0);
        assert!((params.major_spacing - 10.0).abs() < 0.001);
    }

    #[test]
    fn tick_spacing_at_width_400() {
        let params = calculate_tick_params(400.0);
        assert!((params.major_spacing - 100.0).abs() < 0.001);
    }

    #[test]
    fn tick_thresholds_proportional_to_spacing() {
        let params = calculate_tick_params(4.0);
        assert!((params.major_threshold - params.major_spacing / 50.0).abs() < 0.0001);
        assert!((params.axis_threshold - params.major_spacing / 100.0).abs() < 0.0001);
    }
}
