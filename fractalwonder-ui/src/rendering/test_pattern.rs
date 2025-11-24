/// Colors for the test pattern (RGBA) - used by tests
const BACKGROUND_LIGHT: [u8; 4] = [245, 245, 245, 255]; // Light grey
const BACKGROUND_DARK: [u8; 4] = [255, 255, 255, 255]; // White
const AXIS_COLOR: [u8; 4] = [100, 100, 100, 255]; // Dark grey
const MAJOR_TICK_COLOR: [u8; 4] = [50, 50, 50, 255]; // Darker grey
const MEDIUM_TICK_COLOR: [u8; 4] = [80, 80, 80, 255];
const MINOR_TICK_COLOR: [u8; 4] = [120, 120, 120, 255];
const ORIGIN_COLOR: [u8; 4] = [255, 0, 0, 255]; // Red

/// Calculate distance to nearest multiple of interval.
/// Returns a value in [0, interval/2].
fn distance_to_nearest_multiple(value: f64, interval: f64) -> f64 {
    let remainder = value.rem_euclid(interval);
    remainder.min(interval - remainder)
}

/// Determine if a point is on a "light" or "dark" checkerboard cell.
/// Cells are aligned to major tick grid.
fn is_light_cell(fx: f64, fy: f64, major_spacing: f64) -> bool {
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
    /// Length of major tick marks perpendicular to axis (major / 8)
    pub major_tick_length: f64,
    /// Length of medium tick marks perpendicular to axis (major / 12)
    pub medium_tick_length: f64,
    /// Length of minor tick marks perpendicular to axis (major / 20)
    pub minor_tick_length: f64,
}

/// Calculate tick parameters from viewport width's log2 value.
///
/// Uses log2 to find appropriate scale, then derives all parameters
/// from a single major_spacing value. Works at any zoom level including
/// extreme depths where width would underflow f64.
///
/// # Arguments
/// * `log2_width` - The log2 of the viewport width (from BigFloat::log2_approx())
pub fn calculate_tick_params_from_log2(log2_width: f64) -> TickParams {
    use std::f64::consts::LOG2_10;

    // Convert log2 to log10: log10(x) = log2(x) / log2(10)
    let log10_width = log2_width / LOG2_10;
    let major_exp = (log10_width - 0.5).floor() as i32;

    // major_spacing = 10^major_exp, but we compute it from log2 to avoid overflow/underflow
    // 10^exp = 2^(exp * log2(10))
    let major_log2 = major_exp as f64 * LOG2_10;
    let major_spacing = 2.0_f64.powf(major_log2);

    TickParams {
        major_spacing,
        medium_spacing: major_spacing / 2.0,
        minor_spacing: major_spacing / 10.0,
        major_threshold: major_spacing / 50.0,
        medium_threshold: major_spacing / 75.0,
        minor_threshold: major_spacing / 100.0,
        axis_threshold: major_spacing / 100.0,
        major_tick_length: major_spacing / 8.0,
        medium_tick_length: major_spacing / 12.0,
        minor_tick_length: major_spacing / 20.0,
    }
}

/// Calculate tick parameters from viewport width (f64 version for tests).
///
/// Uses log10 to find appropriate scale, then derives all parameters
/// from a single major_spacing value.
pub fn calculate_tick_params(viewport_width_f64: f64) -> TickParams {
    calculate_tick_params_from_log2(viewport_width_f64.log2())
}

/// Compute the RGBA color for a pixel using NORMALIZED coordinates.
///
/// This version works at any zoom level, including extreme depths where
/// absolute fractal coordinates would underflow f64.
///
/// # Arguments
/// * `norm_x` - Normalized x coordinate in [-0.5, 0.5] (0 = viewport center)
/// * `norm_y` - Normalized y coordinate in [-0.5, 0.5] (0 = viewport center)
/// * `origin_norm_x` - Normalized x of the origin (0,0) relative to viewport center
/// * `origin_norm_y` - Normalized y of the origin (0,0) relative to viewport center
///
/// The origin offsets tell us where (0,0) is in normalized viewport space.
/// At extreme zoom far from origin, these may be very large (origin is far away).
#[allow(dead_code)] // Kept for reference/potential future use
fn test_pattern_color_normalized(
    norm_x: f64,
    norm_y: f64,
    origin_norm_x: f64,
    origin_norm_y: f64,
) -> [u8; 4] {
    // Fixed spacing in normalized coordinates (viewport-relative)
    // Major ticks at 0.2 intervals = 5 major divisions across viewport
    const MAJOR_SPACING: f64 = 0.2;
    const MEDIUM_SPACING: f64 = 0.1;
    const MINOR_SPACING: f64 = 0.02;

    const MAJOR_THRESHOLD: f64 = 0.004;
    const MEDIUM_THRESHOLD: f64 = 0.003;
    const MINOR_THRESHOLD: f64 = 0.002;

    const AXIS_THRESHOLD: f64 = 0.003;
    const ORIGIN_THRESHOLD: f64 = 0.02;

    const MAJOR_TICK_LENGTH: f64 = 0.04;
    const MEDIUM_TICK_LENGTH: f64 = 0.03;
    const MINOR_TICK_LENGTH: f64 = 0.02;

    // Position relative to absolute origin (0,0)
    // fx = norm_x - origin_norm_x gives position in viewport-normalized units
    // where fx=0 means we're at the true origin
    let fx = norm_x - origin_norm_x;
    let fy = norm_y - origin_norm_y;

    // 1. Check for origin marker (highest priority) - only if origin is visible
    if origin_norm_x.abs() < 1.0 && origin_norm_y.abs() < 1.0 {
        let dist_to_origin = (fx * fx + fy * fy).sqrt();
        if dist_to_origin < ORIGIN_THRESHOLD {
            return ORIGIN_COLOR;
        }
    }

    let dist_to_x_axis = fy.abs();
    let dist_to_y_axis = fx.abs();

    // 2. Check for tick marks extending from x-axis (only if x-axis is visible)
    if origin_norm_y.abs() < 1.0 {
        // Use fx (origin-relative) for tick positions so grid stays anchored to origin
        let dist_to_major_x = distance_to_nearest_multiple(fx, MAJOR_SPACING);
        if dist_to_major_x < MAJOR_THRESHOLD && dist_to_x_axis < MAJOR_TICK_LENGTH {
            return MAJOR_TICK_COLOR;
        }
        let dist_to_medium_x = distance_to_nearest_multiple(fx, MEDIUM_SPACING);
        if dist_to_medium_x < MEDIUM_THRESHOLD && dist_to_x_axis < MEDIUM_TICK_LENGTH {
            return MEDIUM_TICK_COLOR;
        }
        let dist_to_minor_x = distance_to_nearest_multiple(fx, MINOR_SPACING);
        if dist_to_minor_x < MINOR_THRESHOLD && dist_to_x_axis < MINOR_TICK_LENGTH {
            return MINOR_TICK_COLOR;
        }

        // Check for horizontal axis line (y ~ 0)
        if dist_to_x_axis < AXIS_THRESHOLD {
            return AXIS_COLOR;
        }
    }

    // 3. Check for tick marks extending from y-axis (only if y-axis is visible)
    if origin_norm_x.abs() < 1.0 {
        // Use fy (origin-relative) for tick positions so grid stays anchored to origin
        let dist_to_major_y = distance_to_nearest_multiple(fy, MAJOR_SPACING);
        if dist_to_major_y < MAJOR_THRESHOLD && dist_to_y_axis < MAJOR_TICK_LENGTH {
            return MAJOR_TICK_COLOR;
        }
        let dist_to_medium_y = distance_to_nearest_multiple(fy, MEDIUM_SPACING);
        if dist_to_medium_y < MEDIUM_THRESHOLD && dist_to_y_axis < MEDIUM_TICK_LENGTH {
            return MEDIUM_TICK_COLOR;
        }
        let dist_to_minor_y = distance_to_nearest_multiple(fy, MINOR_SPACING);
        if dist_to_minor_y < MINOR_THRESHOLD && dist_to_y_axis < MINOR_TICK_LENGTH {
            return MINOR_TICK_COLOR;
        }

        // Check for vertical axis line (x ~ 0)
        if dist_to_y_axis < AXIS_THRESHOLD {
            return AXIS_COLOR;
        }
    }

    // 4. Checkerboard background (use fx, fy so grid stays anchored to origin)
    if is_light_cell(fx, fy, MAJOR_SPACING) {
        BACKGROUND_LIGHT
    } else {
        BACKGROUND_DARK
    }
}

/// Compute the RGBA color for a pixel at fractal coordinates (fx, fy).
///
/// Renders:
/// 1. Checkerboard background aligned to major tick grid
/// 2. Axis lines at x=0 and y=0
/// 3. Tick marks at major/medium/minor intervals (extending perpendicular to axis)
/// 4. Origin marker at (0,0)
#[cfg(test)]
fn test_pattern_color(fx: f64, fy: f64, params: &TickParams) -> [u8; 4] {
    // 1. Check for origin marker (highest priority)
    let dist_to_origin = (fx * fx + fy * fy).sqrt();
    if dist_to_origin < params.major_threshold * 2.0 {
        return ORIGIN_COLOR;
    }

    let dist_to_x_axis = fy.abs();
    let dist_to_y_axis = fx.abs();

    // 2. Check for tick marks extending from x-axis (vertical ticks at x positions)
    // Ticks extend perpendicular to axis: check if within tick length of axis
    let dist_to_major_x = distance_to_nearest_multiple(fx, params.major_spacing);
    if dist_to_major_x < params.major_threshold && dist_to_x_axis < params.major_tick_length {
        return MAJOR_TICK_COLOR;
    }
    let dist_to_medium_x = distance_to_nearest_multiple(fx, params.medium_spacing);
    if dist_to_medium_x < params.medium_threshold && dist_to_x_axis < params.medium_tick_length {
        return MEDIUM_TICK_COLOR;
    }
    let dist_to_minor_x = distance_to_nearest_multiple(fx, params.minor_spacing);
    if dist_to_minor_x < params.minor_threshold && dist_to_x_axis < params.minor_tick_length {
        return MINOR_TICK_COLOR;
    }

    // 3. Check for tick marks extending from y-axis (horizontal ticks at y positions)
    let dist_to_major_y = distance_to_nearest_multiple(fy, params.major_spacing);
    if dist_to_major_y < params.major_threshold && dist_to_y_axis < params.major_tick_length {
        return MAJOR_TICK_COLOR;
    }
    let dist_to_medium_y = distance_to_nearest_multiple(fy, params.medium_spacing);
    if dist_to_medium_y < params.medium_threshold && dist_to_y_axis < params.medium_tick_length {
        return MEDIUM_TICK_COLOR;
    }
    let dist_to_minor_y = distance_to_nearest_multiple(fy, params.minor_spacing);
    if dist_to_minor_y < params.minor_threshold && dist_to_y_axis < params.minor_tick_length {
        return MINOR_TICK_COLOR;
    }

    // 4. Check for horizontal axis line (y ~ 0)
    if dist_to_x_axis < params.axis_threshold {
        return AXIS_COLOR;
    }

    // 5. Check for vertical axis line (x ~ 0)
    if dist_to_y_axis < params.axis_threshold {
        return AXIS_COLOR;
    }

    // 6. Checkerboard background
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
