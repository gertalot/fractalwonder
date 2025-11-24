// fractalwonder-compute/src/test_image.rs

use crate::Renderer;
use fractalwonder_core::{BigFloat, TestImageData, Viewport};

/// Tick spacing parameters derived from viewport size.
/// All thresholds and lengths are proportional to major_spacing.
#[derive(Debug, Clone)]
struct TickParams {
    major_spacing: f64,
    medium_spacing: f64,
    minor_spacing: f64,
    major_threshold: f64,
    medium_threshold: f64,
    minor_threshold: f64,
    axis_threshold: f64,
    origin_threshold: f64,
    major_tick_length: f64,
    medium_tick_length: f64,
    minor_tick_length: f64,
}

/// Calculate tick parameters from viewport height's log2 value.
///
/// Computes tick spacing in fractal space, then normalizes to viewport-relative
/// coordinates. Works at any zoom level including extreme depths.
fn calculate_tick_params(log2_height: f64) -> TickParams {
    use std::f64::consts::LOG2_10;

    // Convert log2 to log10: log10(x) = log2(x) / log2(10)
    let log10_height = log2_height / LOG2_10;

    // Calculate fractal-space tick spacing to get ~4-5 major divisions
    // major_spacing should be 10^exp where exp makes ~4 divisions visible
    let major_exp = (log10_height - 0.5).floor() as i32;

    // major_spacing in fractal space = 10^major_exp
    let major_log2 = major_exp as f64 * LOG2_10;
    let fractal_spacing = 2.0_f64.powf(major_log2);

    // Convert fractal-space spacing to normalized viewport space
    // viewport.height in fractal space maps to 1.0 in normalized space
    let viewport_height = 2.0_f64.powf(log2_height);
    let major_spacing = fractal_spacing / viewport_height;

    // All other parameters are proportional and also in normalized space
    TickParams {
        major_spacing,
        medium_spacing: major_spacing / 2.0,
        minor_spacing: major_spacing / 10.0,
        major_threshold: major_spacing / 50.0,
        medium_threshold: major_spacing / 75.0,
        minor_threshold: major_spacing / 100.0,
        axis_threshold: major_spacing / 100.0,
        origin_threshold: major_spacing / 5.0,
        major_tick_length: major_spacing / 5.0,
        medium_tick_length: major_spacing / 7.5,
        minor_tick_length: major_spacing / 12.5,
    }
}

/// Renderer for the test image pattern using normalized viewport coordinates.
pub struct TestImageRenderer;

impl Renderer for TestImageRenderer {
    type Data = TestImageData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<TestImageData> {
        let (width, height) = canvas_size;
        let precision = viewport.precision_bits();

        // Pre-compute origin offset in normalized viewport coordinates.
        // Normalize by height to preserve aspect ratio.
        let zero = BigFloat::zero(precision);
        let origin_norm_x = zero.sub(&viewport.center.0).div(&viewport.height).to_f64();
        let origin_norm_y = zero.sub(&viewport.center.1).div(&viewport.height).to_f64();

        // Compute adaptive tick spacing based on viewport height
        let log2_height = viewport.height.log2_approx();
        let tick_params = calculate_tick_params(log2_height);

        (0..height)
            .flat_map(|py| {
                let tick_params = tick_params.clone();
                (0..width).map(move |px| {
                    // Pixel to normalized coords
                    // Map [0, width) -> [-0.5*aspect, 0.5*aspect] and [0, height) -> [-0.5, 0.5]
                    let aspect = width as f64 / height as f64;
                    let norm_x = ((px as f64 / width as f64) - 0.5) * aspect;
                    let norm_y = (py as f64 / height as f64) - 0.5;

                    compute_test_image_data(
                        norm_x,
                        norm_y,
                        origin_norm_x,
                        origin_norm_y,
                        &tick_params,
                    )
                })
            })
            .collect()
    }
}

/// Compute TestImageData for a single pixel using normalized coordinates.
fn compute_test_image_data(
    norm_x: f64,
    norm_y: f64,
    origin_norm_x: f64,
    origin_norm_y: f64,
    params: &TickParams,
) -> TestImageData {
    // Position relative to absolute origin (0,0)
    let fx = norm_x - origin_norm_x;
    let fy = norm_y - origin_norm_y;

    let origin_visible = origin_norm_x.abs() < 1.0 && origin_norm_y.abs() < 1.0;
    let x_axis_visible = origin_norm_y.abs() < 1.0;
    let y_axis_visible = origin_norm_x.abs() < 1.0;

    let dist_to_origin = (fx * fx + fy * fy).sqrt();
    let dist_to_x_axis = fy.abs();
    let dist_to_y_axis = fx.abs();

    let dist_to_major_x = distance_to_nearest_multiple(fx, params.major_spacing);
    let dist_to_medium_x = distance_to_nearest_multiple(fx, params.medium_spacing);
    let dist_to_minor_x = distance_to_nearest_multiple(fx, params.minor_spacing);

    let dist_to_major_y = distance_to_nearest_multiple(fy, params.major_spacing);
    let dist_to_medium_y = distance_to_nearest_multiple(fy, params.medium_spacing);
    let dist_to_minor_y = distance_to_nearest_multiple(fy, params.minor_spacing);

    TestImageData {
        is_on_origin: origin_visible && dist_to_origin < params.origin_threshold,
        is_on_x_axis: x_axis_visible && dist_to_x_axis < params.axis_threshold,
        is_on_y_axis: y_axis_visible && dist_to_y_axis < params.axis_threshold,
        is_on_major_tick_x: x_axis_visible
            && dist_to_major_x < params.major_threshold
            && dist_to_x_axis < params.major_tick_length,
        is_on_medium_tick_x: x_axis_visible
            && dist_to_medium_x < params.medium_threshold
            && dist_to_x_axis < params.medium_tick_length,
        is_on_minor_tick_x: x_axis_visible
            && dist_to_minor_x < params.minor_threshold
            && dist_to_x_axis < params.minor_tick_length,
        is_on_major_tick_y: y_axis_visible
            && dist_to_major_y < params.major_threshold
            && dist_to_y_axis < params.major_tick_length,
        is_on_medium_tick_y: y_axis_visible
            && dist_to_medium_y < params.medium_threshold
            && dist_to_y_axis < params.medium_tick_length,
        is_on_minor_tick_y: y_axis_visible
            && dist_to_minor_y < params.minor_threshold
            && dist_to_y_axis < params.minor_tick_length,
        is_light_cell: is_light_cell(fx, fy, params.major_spacing),
    }
}

/// Calculate distance to nearest multiple of interval.
fn distance_to_nearest_multiple(value: f64, interval: f64) -> f64 {
    let remainder = value.rem_euclid(interval);
    remainder.min(interval - remainder)
}

/// Determine if a point is on a "light" or "dark" checkerboard cell.
fn is_light_cell(fx: f64, fy: f64, major_spacing: f64) -> bool {
    let cell_x = (fx / major_spacing).floor() as i64;
    let cell_y = (fy / major_spacing).floor() as i64;
    (cell_x + cell_y) % 2 == 0
}
