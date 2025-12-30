//! Fractal configuration registry.
//!
//! Defines available fractal types with their natural bounds and metadata.
//! Core rendering parameters come from fractalwonder_core::config.

use fractalwonder_core::{Viewport, MANDELBROT_CONFIG as CORE_MANDELBROT};

// Re-export core config functions for convenience
pub use fractalwonder_core::{
    calculate_dc_max, calculate_render_max_iterations, is_bla_useful,
    FractalConfig as CoreFractalConfig,
};

/// Determines which renderer implementation to use.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum RendererType {
    /// Simple per-pixel BigFloat computation
    #[default]
    Simple,
    /// Perturbation theory with f64 delta iterations
    Perturbation,
}

/// UI-specific configuration for a fractal type.
/// Core rendering parameters (tau_sq, iteration_multiplier, etc.) come from
/// fractalwonder_core::config to ensure consistency between UI and compute layers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractalConfig {
    /// Reference to core config (source of truth for rendering parameters)
    pub core: &'static CoreFractalConfig,
    /// Which renderer implementation to use
    pub renderer_type: RendererType,
    /// Number of web workers for parallel rendering.
    /// 0 = use all available hardware threads (hardware_concurrency).
    pub worker_count: usize,
    /// Minimum precision bits before switching to BigFloat delta arithmetic.
    /// Below this threshold, fast f64 arithmetic is used.
    /// 1024 bits ≈ 10^300 zoom depth.
    pub bigfloat_threshold_bits: usize,
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
    /// Get the unique identifier.
    pub fn id(&self) -> &'static str {
        self.core.id
    }

    /// Get the display name.
    pub fn display_name(&self) -> &'static str {
        self.core.display_name
    }

    /// Get tau_sq (glitch detection threshold squared).
    pub fn tau_sq(&self) -> f64 {
        self.core.tau_sq
    }

    /// Get iteration multiplier.
    pub fn iteration_multiplier(&self) -> f64 {
        self.core.iteration_multiplier
    }

    /// Get iteration power.
    pub fn iteration_power(&self) -> f64 {
        self.core.iteration_power
    }

    /// Check if BLA is enabled.
    pub fn bla_enabled(&self) -> bool {
        self.core.bla_enabled
    }

    /// Create the default viewport for this fractal at the given precision.
    pub fn default_viewport(&self, precision_bits: usize) -> Viewport {
        self.core.default_viewport(precision_bits)
    }
}

/// Registry of available fractal configurations.
pub static FRACTAL_CONFIGS: &[FractalConfig] = &[FractalConfig {
    core: &CORE_MANDELBROT,
    renderer_type: RendererType::Perturbation,
    worker_count: 0, // all available workers
    bigfloat_threshold_bits: 1024, // ~10^300 zoom
    gpu_enabled: true,
    gpu_iterations_per_dispatch: 50_000,
    gpu_progressive_row_sets: 32, // 0 = use old tiled renderer, >0 = progressive
}];

/// Look up a fractal configuration by ID.
pub fn get_config(id: &str) -> Option<&'static FractalConfig> {
    FRACTAL_CONFIGS.iter().find(|c| c.id() == id)
}

/// Get the default fractal configuration.
pub fn default_config() -> &'static FractalConfig {
    get_config("mandelbrot").expect("Default config must exist")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_config_finds_mandelbrot() {
        let config = get_config("mandelbrot");
        assert!(config.is_some());
        assert_eq!(config.unwrap().display_name(), "Mandelbrot Set");
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
    fn default_config_returns_mandelbrot() {
        let config = default_config();
        assert_eq!(config.id(), "mandelbrot");
    }

    #[test]
    fn config_uses_core_values() {
        let config = get_config("mandelbrot").unwrap();
        // Verify core values are accessible
        assert_eq!(config.tau_sq(), 1e-6);
        assert_eq!(config.iteration_multiplier(), 200.0);
        assert_eq!(config.iteration_power(), 2.8);
        assert!(config.bla_enabled());
    }
}
