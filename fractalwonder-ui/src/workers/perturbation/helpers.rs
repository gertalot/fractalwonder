//! Pure helper functions for perturbation rendering.
//!
//! These functions are stateless and easily testable.

use crate::config::FractalConfig;
use fractalwonder_core::{calculate_max_iterations, HDRFloat, Viewport};

/// Validate viewport dimensions for rendering.
///
/// Returns Ok(()) if valid, Err with message if invalid.
pub fn validate_viewport(viewport: &Viewport) -> Result<(), String> {
    let vp_width = viewport.width.to_f64();
    let vp_height = viewport.height.to_f64();

    if !vp_width.is_finite() || !vp_height.is_finite() || vp_width <= 0.0 || vp_height <= 0.0 {
        return Err(format!(
            "Invalid viewport dimensions: width={}, height={}",
            vp_width, vp_height
        ));
    }

    Ok(())
}

/// Calculate maximum iterations for a render based on zoom level.
pub fn calculate_render_max_iterations(viewport: &Viewport, config: Option<&FractalConfig>) -> u32 {
    let vp_width = viewport.width.to_f64();

    // Calculate zoom exponent from viewport width
    // Default Mandelbrot width is ~4, so zoom = 4 / width
    let zoom = 4.0 / vp_width;
    let zoom_exponent = if zoom.is_finite() && zoom > 0.0 {
        zoom.log10()
    } else {
        0.0
    };

    let multiplier = config.map(|c| c.iteration_multiplier).unwrap_or(200.0);
    let power = config.map(|c| c.iteration_power).unwrap_or(2.5);

    calculate_max_iterations(zoom_exponent, multiplier, power)
}

/// Calculate maximum |delta_c| for any pixel in the viewport.
///
/// This is the distance from viewport center to the farthest corner,
/// used for BLA table construction.
///
/// Uses HDRFloat to avoid underflow when squaring very small viewport dimensions
/// at extreme zoom levels (e.g., 10^270 where f64 squaring underflows to 0).
pub fn calculate_dc_max(viewport: &Viewport) -> HDRFloat {
    let half_width = HDRFloat::from_bigfloat(&viewport.width).div_f64(2.0);
    let half_height = HDRFloat::from_bigfloat(&viewport.height).div_f64(2.0);
    half_width.square().add(&half_height.square()).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::BigFloat;

    fn create_test_viewport(width: f64, height: f64) -> Viewport {
        Viewport {
            center: (
                BigFloat::with_precision(-0.5, 64),
                BigFloat::with_precision(0.0, 64),
            ),
            width: BigFloat::with_precision(width, 64),
            height: BigFloat::with_precision(height, 64),
        }
    }

    #[test]
    fn validate_viewport_accepts_valid() {
        let viewport = create_test_viewport(4.0, 4.0);
        assert!(validate_viewport(&viewport).is_ok());
    }

    #[test]
    fn validate_viewport_rejects_zero_width() {
        let viewport = create_test_viewport(0.0, 4.0);
        assert!(validate_viewport(&viewport).is_err());
    }

    #[test]
    fn validate_viewport_rejects_negative_height() {
        let viewport = create_test_viewport(4.0, -1.0);
        assert!(validate_viewport(&viewport).is_err());
    }

    #[test]
    fn calculate_dc_max_at_default_zoom() {
        let viewport = create_test_viewport(4.0, 4.0);
        let dc_max = calculate_dc_max(&viewport).to_f64();
        // sqrt(2^2 + 2^2) = sqrt(8) ≈ 2.828
        assert!((dc_max - 2.828).abs() < 0.01);
    }

    #[test]
    fn calculate_max_iterations_increases_with_zoom() {
        let shallow = create_test_viewport(4.0, 4.0);
        let deep = create_test_viewport(0.0001, 0.0001);

        let shallow_iter = calculate_render_max_iterations(&shallow, None);
        let deep_iter = calculate_render_max_iterations(&deep, None);

        assert!(deep_iter > shallow_iter);
    }
}
