// fractalwonder-compute/src/test_image.rs

use crate::Renderer;
use fractalwonder_core::{BigFloat, TestImageData, Viewport};

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

        (0..height)
            .flat_map(|py| {
                (0..width).map(move |px| {
                    // Pixel to normalized coords
                    // Map [0, width) -> [-0.5*aspect, 0.5*aspect] and [0, height) -> [-0.5, 0.5]
                    let aspect = width as f64 / height as f64;
                    let norm_x = ((px as f64 / width as f64) - 0.5) * aspect;
                    let norm_y = (py as f64 / height as f64) - 0.5;

                    compute_test_image_data(norm_x, norm_y, origin_norm_x, origin_norm_y)
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
) -> TestImageData {
    // Fixed spacing in normalized coordinates (viewport-relative)
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
    let fx = norm_x - origin_norm_x;
    let fy = norm_y - origin_norm_y;

    let origin_visible = origin_norm_x.abs() < 1.0 && origin_norm_y.abs() < 1.0;
    let x_axis_visible = origin_norm_y.abs() < 1.0;
    let y_axis_visible = origin_norm_x.abs() < 1.0;

    let dist_to_origin = (fx * fx + fy * fy).sqrt();
    let dist_to_x_axis = fy.abs();
    let dist_to_y_axis = fx.abs();

    let dist_to_major_x = distance_to_nearest_multiple(fx, MAJOR_SPACING);
    let dist_to_medium_x = distance_to_nearest_multiple(fx, MEDIUM_SPACING);
    let dist_to_minor_x = distance_to_nearest_multiple(fx, MINOR_SPACING);

    let dist_to_major_y = distance_to_nearest_multiple(fy, MAJOR_SPACING);
    let dist_to_medium_y = distance_to_nearest_multiple(fy, MEDIUM_SPACING);
    let dist_to_minor_y = distance_to_nearest_multiple(fy, MINOR_SPACING);

    TestImageData {
        is_on_origin: origin_visible && dist_to_origin < ORIGIN_THRESHOLD,
        is_on_x_axis: x_axis_visible && dist_to_x_axis < AXIS_THRESHOLD,
        is_on_y_axis: y_axis_visible && dist_to_y_axis < AXIS_THRESHOLD,
        is_on_major_tick_x: x_axis_visible
            && dist_to_major_x < MAJOR_THRESHOLD
            && dist_to_x_axis < MAJOR_TICK_LENGTH,
        is_on_medium_tick_x: x_axis_visible
            && dist_to_medium_x < MEDIUM_THRESHOLD
            && dist_to_x_axis < MEDIUM_TICK_LENGTH,
        is_on_minor_tick_x: x_axis_visible
            && dist_to_minor_x < MINOR_THRESHOLD
            && dist_to_x_axis < MINOR_TICK_LENGTH,
        is_on_major_tick_y: y_axis_visible
            && dist_to_major_y < MAJOR_THRESHOLD
            && dist_to_y_axis < MAJOR_TICK_LENGTH,
        is_on_medium_tick_y: y_axis_visible
            && dist_to_medium_y < MEDIUM_THRESHOLD
            && dist_to_y_axis < MEDIUM_TICK_LENGTH,
        is_on_minor_tick_y: y_axis_visible
            && dist_to_minor_y < MINOR_THRESHOLD
            && dist_to_y_axis < MINOR_TICK_LENGTH,
        is_light_cell: is_light_cell(fx, fy, MAJOR_SPACING),
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
