//! Fractal configuration registry.
//!
//! Defines available fractal types with their natural bounds and metadata.

use fractalwonder_core::Viewport;

/// Determines which renderer implementation to use.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum RendererType {
    /// Simple per-pixel BigFloat computation
    #[default]
    Simple,
    /// Perturbation theory with f64 delta iterations
    Perturbation,
}

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
    /// Which renderer implementation to use
    pub renderer_type: RendererType,
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

/// Registry of available fractal configurations.
pub static FRACTAL_CONFIGS: &[FractalConfig] = &[
    FractalConfig {
        id: "test_image",
        display_name: "Test Pattern",
        default_center: ("0.0", "0.0"),
        default_width: "100.0",
        default_height: "100.0",
        renderer_type: RendererType::Simple,
    },
    FractalConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot Set",
        default_center: ("-0.5", "0.0"),
        default_width: "4.0",
        default_height: "4.0",
        renderer_type: RendererType::Perturbation,
    },
];

/// Look up a fractal configuration by ID.
pub fn get_config(id: &str) -> Option<&'static FractalConfig> {
    FRACTAL_CONFIGS.iter().find(|c| c.id == id)
}

/// Get the default fractal configuration.
pub fn default_config() -> &'static FractalConfig {
    get_config("test_image").expect("Default config must exist")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_config_finds_mandelbrot() {
        let config = get_config("mandelbrot");
        assert!(config.is_some());
        assert_eq!(config.unwrap().display_name, "Mandelbrot Set");
    }

    #[test]
    fn get_config_finds_test_image() {
        let config = get_config("test_image");
        assert!(config.is_some());
        assert_eq!(config.unwrap().display_name, "Test Pattern");
    }

    #[test]
    fn get_config_returns_none_for_unknown() {
        let config = get_config("unknown_fractal");
        assert!(config.is_none());
    }

    #[test]
    fn default_viewport_creates_valid_viewport() {
        let config = get_config("mandelbrot").unwrap();
        let viewport = config.default_viewport(128);

        assert!((viewport.center.0.to_f64() - (-0.5)).abs() < 0.001);
        assert!((viewport.center.1.to_f64() - 0.0).abs() < 0.001);
        assert!((viewport.width.to_f64() - 4.0).abs() < 0.001);
        assert!((viewport.height.to_f64() - 4.0).abs() < 0.001);
        assert_eq!(viewport.precision_bits(), 128);
    }

    #[test]
    fn default_config_returns_test_image() {
        let config = default_config();
        assert_eq!(config.id, "test_image");
    }
}
