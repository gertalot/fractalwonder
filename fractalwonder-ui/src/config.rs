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
    /// Glitch detection threshold squared (τ²).
    /// Default 1e-6 corresponds to τ = 10⁻³ (standard).
    /// See docs/research/perturbation-theory.md Section 2.5.
    pub tau_sq: f64,
    /// Number of web workers for parallel rendering.
    /// 0 = use all available hardware threads (hardware_concurrency).
    pub worker_count: usize,
    /// Multiplier for max iterations formula: multiplier * zoom_exp^power.
    pub iteration_multiplier: f64,
    /// Power for max iterations formula: multiplier * zoom_exp^power.
    pub iteration_power: f64,
    /// Minimum precision bits before switching to BigFloat delta arithmetic.
    /// Below this threshold, fast f64 arithmetic is used.
    /// 1024 bits ≈ 10^300 zoom depth.
    pub bigfloat_threshold_bits: usize,
    /// Enable BLA (Bivariate Linear Approximation) for iteration skipping.
    /// Provides significant speedup at deep zoom levels.
    pub bla_enabled: bool,
    /// Enable GPU acceleration via WebGPU compute shaders.
    /// Falls back to CPU if GPU unavailable or disabled.
    pub gpu_enabled: bool,
    /// Iterations per GPU dispatch (prevents timeout).
    /// Default 100,000 keeps each dispatch under browser timeout threshold.
    pub gpu_iterations_per_dispatch: u32,
    /// Number of row-sets for progressive rendering (venetian blinds).
    /// Default 16 means rows 0,16,32... render first, then 1,17,33..., etc.
    pub gpu_progressive_row_sets: u32,
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
        tau_sq: 1e-6,
        worker_count: 1,
        iteration_multiplier: 200.0,
        iteration_power: 2.5,
        bigfloat_threshold_bits: 1024,
        bla_enabled: false,
        gpu_enabled: false,
        gpu_iterations_per_dispatch: 100_000,
        gpu_progressive_row_sets: 16,
    },
    FractalConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot Set",
        default_center: ("-0.5", "0.0"),
        default_width: "4.0",
        default_height: "4.0",
        renderer_type: RendererType::Perturbation,
        tau_sq: 1e-6,
        worker_count: 0, // all available workers
        iteration_multiplier: 200.0,
        iteration_power: 2.7,
        bigfloat_threshold_bits: 1024, // ~10^300 zoom
        bla_enabled: true,
        gpu_enabled: true,
        gpu_iterations_per_dispatch: 100_000,
        gpu_progressive_row_sets: 16, // 0 = use old tiled renderer, >0 = progressive
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
