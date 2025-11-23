/// Colors for the test pattern (RGBA)
pub const BACKGROUND_LIGHT: [u8; 4] = [245, 245, 245, 255]; // Light grey
pub const BACKGROUND_DARK: [u8; 4] = [255, 255, 255, 255]; // White
pub const AXIS_COLOR: [u8; 4] = [100, 100, 100, 255]; // Dark grey
pub const MAJOR_TICK_COLOR: [u8; 4] = [50, 50, 50, 255]; // Darker grey
pub const MEDIUM_TICK_COLOR: [u8; 4] = [80, 80, 80, 255];
pub const MINOR_TICK_COLOR: [u8; 4] = [120, 120, 120, 255];
pub const ORIGIN_COLOR: [u8; 4] = [255, 0, 0, 255]; // Red

/// Calculate distance to nearest multiple of interval.
/// Returns a value in [0, interval/2].
pub fn distance_to_nearest_multiple(value: f64, interval: f64) -> f64 {
    let remainder = value.rem_euclid(interval);
    remainder.min(interval - remainder)
}

/// Determine if a point is on a "light" or "dark" checkerboard cell.
/// Cells are aligned to major tick grid.
pub fn is_light_cell(fx: f64, fy: f64, major_spacing: f64) -> bool {
    let cell_x = (fx / major_spacing).floor() as i64;
    let cell_y = (fy / major_spacing).floor() as i64;
    (cell_x + cell_y) % 2 == 0
}

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

/// Compute the RGBA color for a pixel at fractal coordinates (fx, fy).
///
/// Renders:
/// 1. Checkerboard background aligned to major tick grid
/// 2. Axis lines at x=0 and y=0
/// 3. Tick marks at major/medium/minor intervals
/// 4. Origin marker at (0,0)
pub fn test_pattern_color(fx: f64, fy: f64, params: &TickParams) -> [u8; 4] {
    // 1. Check for origin marker (highest priority)
    let dist_to_origin = (fx * fx + fy * fy).sqrt();
    if dist_to_origin < params.major_threshold * 2.0 {
        return ORIGIN_COLOR;
    }

    // 2. Check for horizontal axis (y ~ 0)
    let dist_to_x_axis = fy.abs();
    if dist_to_x_axis < params.axis_threshold {
        // Check for tick marks along x-axis
        let dist_to_major = distance_to_nearest_multiple(fx, params.major_spacing);
        if dist_to_major < params.major_threshold {
            return MAJOR_TICK_COLOR;
        }
        let dist_to_medium = distance_to_nearest_multiple(fx, params.medium_spacing);
        if dist_to_medium < params.medium_threshold {
            return MEDIUM_TICK_COLOR;
        }
        let dist_to_minor = distance_to_nearest_multiple(fx, params.minor_spacing);
        if dist_to_minor < params.minor_threshold {
            return MINOR_TICK_COLOR;
        }
        return AXIS_COLOR;
    }

    // 3. Check for vertical axis (x ~ 0)
    let dist_to_y_axis = fx.abs();
    if dist_to_y_axis < params.axis_threshold {
        // Check for tick marks along y-axis
        let dist_to_major = distance_to_nearest_multiple(fy, params.major_spacing);
        if dist_to_major < params.major_threshold {
            return MAJOR_TICK_COLOR;
        }
        let dist_to_medium = distance_to_nearest_multiple(fy, params.medium_spacing);
        if dist_to_medium < params.medium_threshold {
            return MEDIUM_TICK_COLOR;
        }
        let dist_to_minor = distance_to_nearest_multiple(fy, params.minor_spacing);
        if dist_to_minor < params.minor_threshold {
            return MINOR_TICK_COLOR;
        }
        return AXIS_COLOR;
    }

    // 4. Checkerboard background
    if is_light_cell(fx, fy, params.major_spacing) {
        BACKGROUND_LIGHT
    } else {
        BACKGROUND_DARK
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

    #[test]
    fn distance_to_nearest_multiple_at_boundary() {
        assert!((distance_to_nearest_multiple(1.0, 1.0) - 0.0).abs() < 0.0001);
        assert!((distance_to_nearest_multiple(0.0, 1.0) - 0.0).abs() < 0.0001);
        assert!((distance_to_nearest_multiple(2.5, 1.0) - 0.5).abs() < 0.0001);
    }

    #[test]
    fn distance_to_nearest_multiple_negative_values() {
        assert!((distance_to_nearest_multiple(-1.0, 1.0) - 0.0).abs() < 0.0001);
        assert!((distance_to_nearest_multiple(-0.3, 1.0) - 0.3).abs() < 0.0001);
    }

    #[test]
    fn checkerboard_alternates_at_integer_boundaries() {
        // With major_spacing=1.0, cells at (0.5, 0.5) and (1.5, 0.5) should differ
        assert!(is_light_cell(0.5, 0.5, 1.0));
        assert!(!is_light_cell(1.5, 0.5, 1.0));
        assert!(!is_light_cell(0.5, 1.5, 1.0));
        assert!(is_light_cell(1.5, 1.5, 1.0));
    }

    #[test]
    fn checkerboard_works_with_negative_coords() {
        assert!(is_light_cell(-0.5, -0.5, 1.0));
        assert!(!is_light_cell(-1.5, -0.5, 1.0));
    }

    #[test]
    fn test_pattern_axis_detected_near_zero() {
        let params = calculate_tick_params(4.0);
        // Point very close to y=0 axis should be axis color (or tick color)
        let color = test_pattern_color(0.5, 0.001, &params);
        // Should NOT be background color
        assert_ne!(color, BACKGROUND_LIGHT);
        assert_ne!(color, BACKGROUND_DARK);
    }

    #[test]
    fn test_pattern_origin_is_red() {
        let params = calculate_tick_params(4.0);
        let color = test_pattern_color(0.0, 0.0, &params);
        assert_eq!(color, ORIGIN_COLOR);
    }

    #[test]
    fn test_pattern_background_alternates() {
        let params = calculate_tick_params(4.0);
        // Points away from axes should be background
        let c1 = test_pattern_color(0.5, 0.5, &params);
        let c2 = test_pattern_color(1.5, 0.5, &params);
        // Should be different (checkerboard)
        assert_ne!(c1, c2);
        // Both should be background colors
        assert!(c1 == BACKGROUND_LIGHT || c1 == BACKGROUND_DARK);
        assert!(c2 == BACKGROUND_LIGHT || c2 == BACKGROUND_DARK);
    }
}
