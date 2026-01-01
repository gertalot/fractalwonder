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
///
/// Uses log2_approx() instead of to_f64() to handle extreme zoom depths
/// beyond f64 range (e.g., 10^308+).
pub fn calculate_render_max_iterations(viewport: &Viewport, config: Option<&FractalConfig>) -> u32 {
    // Calculate zoom exponent from viewport width using log2_approx to handle
    // extreme values that overflow/underflow f64.
    // zoom = 4 / width, so log10(zoom) = log10(4) - log10(width)
    // log10(x) = log2(x) * log10(2)
    let log2_width = viewport.width.log2_approx();
    let log2_zoom = 2.0 - log2_width; // log2(4) = 2
    let zoom_exponent = if log2_zoom.is_finite() {
        log2_zoom * std::f64::consts::LOG10_2
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
/// Returns HDRFloat to prevent underflow at deep zoom levels where
/// viewport dimensions like 10^-270 would underflow in f64.
pub fn calculate_dc_max(viewport: &Viewport) -> HDRFloat {
    // Convert BigFloat dimensions to HDRFloat to preserve extended exponent range
    let half_width = HDRFloat::from_bigfloat(&viewport.width).div_f64(2.0);
    let half_height = HDRFloat::from_bigfloat(&viewport.height).div_f64(2.0);

    // dc_max = sqrt(half_width² + half_height²)
    let width_sq = half_width.square();
    let height_sq = half_height.square();
    width_sq.add(&height_sq).sqrt()
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
        let dc_max = calculate_dc_max(&viewport);
        // sqrt(2^2 + 2^2) = sqrt(8) ≈ 2.828
        assert!((dc_max.to_f64() - 2.828).abs() < 0.01);
    }

    #[test]
    fn calculate_max_iterations_increases_with_zoom() {
        let shallow = create_test_viewport(4.0, 4.0);
        let deep = create_test_viewport(0.0001, 0.0001);

        let shallow_iter = calculate_render_max_iterations(&shallow, None);
        let deep_iter = calculate_render_max_iterations(&deep, None);

        assert!(deep_iter > shallow_iter);
    }

    #[test]
    fn calculate_max_iterations_handles_extreme_zoom_beyond_f64() {
        // Test zoom at 10^308 - beyond f64 range for direct computation
        // Width ~1.5e-309 would underflow/overflow with to_f64() approach
        let extreme_viewport = Viewport {
            center: (
                BigFloat::with_precision(0.273, 2000),
                BigFloat::with_precision(0.006, 2000),
            ),
            width: BigFloat::from_string("1.5e-309", 2000).unwrap(),
            height: BigFloat::from_string("1.0e-309", 2000).unwrap(),
        };

        let iter = calculate_render_max_iterations(&extreme_viewport, None);

        // At 10^308 zoom, should get much more than 1000 iterations
        // (the bug caused fallback to zoom_exponent=0 → only 1000 iter)
        assert!(
            iter > 10000,
            "At 10^308 zoom, expected >10000 iterations, got {}",
            iter
        );
    }
}
