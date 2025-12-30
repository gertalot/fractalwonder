//! Pure helper functions for perturbation rendering.
//!
//! These functions delegate to fractalwonder_core::config for the actual
//! implementations, ensuring consistency between UI and compute layers.

use crate::config::FractalConfig;
use fractalwonder_core::Viewport;

// Re-export core functions - these are the actual implementations
pub use fractalwonder_core::calculate_dc_max;

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
/// Delegates to core implementation using the config's core parameters.
pub fn calculate_render_max_iterations(viewport: &Viewport, config: Option<&FractalConfig>) -> u32 {
    match config {
        Some(cfg) => fractalwonder_core::calculate_render_max_iterations(viewport, cfg.core),
        None => {
            // Fallback to mandelbrot config if none provided
            fractalwonder_core::calculate_render_max_iterations(
                viewport,
                &fractalwonder_core::MANDELBROT_CONFIG,
            )
        }
    }
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
