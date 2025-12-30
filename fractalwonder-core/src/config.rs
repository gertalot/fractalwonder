//! Fractal configuration and rendering helpers.
//!
//! This module contains configuration for fractal types and pure helper
//! functions used by both the UI coordinator and compute workers.

use crate::{calculate_max_iterations, HDRFloat, Viewport};

/// Configuration for a fractal type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractalConfig {
    /// Unique identifier (matches renderer ID in compute layer)
    pub id: &'static str,
    /// Human-readable name for UI display
    pub display_name: &'static str,
    /// Default center coordinates as strings (preserves precision)
    pub default_center: (&'static str, &'static str),
    /// Default width in fractal space as string
    pub default_width: &'static str,
    /// Default height in fractal space as string
    pub default_height: &'static str,
    /// Glitch detection threshold squared (τ²).
    /// Default 1e-6 corresponds to τ = 10⁻³ (standard).
    pub tau_sq: f64,
    /// Multiplier for max iterations formula: multiplier * zoom_exp^power.
    pub iteration_multiplier: f64,
    /// Power for max iterations formula: multiplier * zoom_exp^power.
    pub iteration_power: f64,
    /// Enable BLA (Bivariate Linear Approximation) for iteration skipping.
    pub bla_enabled: bool,
}

impl FractalConfig {
    /// Create the default viewport for this fractal at the given precision.
    pub fn default_viewport(&self, precision_bits: usize) -> Viewport {
        Viewport::from_strings(
            self.default_center.0,
            self.default_center.1,
            self.default_width,
            self.default_height,
            precision_bits,
        )
        .expect("Invalid default viewport coordinates in FractalConfig")
    }
}

/// Mandelbrot set configuration.
/// This is the canonical source of truth for Mandelbrot rendering parameters.
pub static MANDELBROT_CONFIG: FractalConfig = FractalConfig {
    id: "mandelbrot",
    display_name: "Mandelbrot Set",
    default_center: ("-0.5", "0.0"),
    default_width: "4.0",
    default_height: "4.0",
    tau_sq: 1e-6,
    iteration_multiplier: 200.0,
    iteration_power: 2.8,
    bla_enabled: true,
};

/// Look up a fractal configuration by ID.
pub fn get_fractal_config(id: &str) -> Option<&'static FractalConfig> {
    match id {
        "mandelbrot" => Some(&MANDELBROT_CONFIG),
        _ => None,
    }
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

/// Calculate maximum iterations for a render based on zoom level and config.
///
/// Uses formula: multiplier * zoom_exp^power, clamped to [1000, 10_000_000].
pub fn calculate_render_max_iterations(viewport: &Viewport, config: &FractalConfig) -> u32 {
    let vp_width = viewport.width.to_f64();

    // Calculate zoom exponent from viewport width
    // Default Mandelbrot width is ~4, so zoom = 4 / width
    let zoom = 4.0 / vp_width;
    let zoom_exponent = if zoom.is_finite() && zoom > 0.0 {
        zoom.log10()
    } else {
        0.0
    };

    calculate_max_iterations(zoom_exponent, config.iteration_multiplier, config.iteration_power)
}

/// Check if BLA is useful at the current zoom level.
///
/// BLA helps at deep zoom where iteration counts are high.
/// Phil Thompson enables BLA at scale > 1e25 (dc_max < ~1e-25).
/// Reference: https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html
pub fn is_bla_useful(dc_max: &HDRFloat) -> bool {
    dc_max.log2() < -80.0 // Roughly 10^-25 (scale > 1e25)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BigFloat;

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
    fn get_fractal_config_finds_mandelbrot() {
        let config = get_fractal_config("mandelbrot");
        assert!(config.is_some());
        assert_eq!(config.unwrap().display_name, "Mandelbrot Set");
    }

    #[test]
    fn get_fractal_config_returns_none_for_unknown() {
        let config = get_fractal_config("unknown_fractal");
        assert!(config.is_none());
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

        let shallow_iter = calculate_render_max_iterations(&shallow, &MANDELBROT_CONFIG);
        let deep_iter = calculate_render_max_iterations(&deep, &MANDELBROT_CONFIG);

        assert!(deep_iter > shallow_iter);
    }

    #[test]
    fn bla_useful_at_deep_zoom() {
        // At deep zoom, dc_max is tiny
        let tiny_dc_max = HDRFloat::from_f64(1e-100);
        assert!(is_bla_useful(&tiny_dc_max));
    }

    #[test]
    fn bla_not_useful_at_shallow_zoom() {
        // At shallow zoom, dc_max is large
        let large_dc_max = HDRFloat::from_f64(2.0);
        assert!(!is_bla_useful(&large_dc_max));
    }

    #[test]
    fn mandelbrot_config_values() {
        assert_eq!(MANDELBROT_CONFIG.tau_sq, 1e-6);
        assert_eq!(MANDELBROT_CONFIG.iteration_multiplier, 200.0);
        assert_eq!(MANDELBROT_CONFIG.iteration_power, 2.8);
        assert!(MANDELBROT_CONFIG.bla_enabled);
    }
}
