//! Fractal configuration registry.
//!
//! Defines available fractal types with their natural bounds and metadata.
//! Also provides runtime settings persisted to localStorage (but not URL).

use fractalwonder_core::Viewport;
use std::cell::Cell;

#[cfg(target_arch = "wasm32")]
const CPU_THREADS_STORAGE_KEY: &str = "fractalwonder_cpu_threads";
#[cfg(target_arch = "wasm32")]
const GPU_SETTING_STORAGE_KEY: &str = "fractalwonder_use_gpu";

// Runtime cache for CPU threads setting
thread_local! {
    /// Cached CPU thread count. None = not yet loaded from localStorage.
    static CPU_THREADS_CACHE: Cell<Option<i32>> = const { Cell::new(None) };
    /// Cached GPU setting. None = not yet loaded from localStorage.
    static GPU_SETTING_CACHE: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Load CPU threads setting from localStorage.
fn load_cpu_threads_from_storage() -> Option<i32> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let storage = window.local_storage().ok()??;
        let value = storage.get_item(CPU_THREADS_STORAGE_KEY).ok()??;
        value.parse().ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Save CPU threads setting to localStorage.
fn save_cpu_threads_to_storage(value: i32) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(CPU_THREADS_STORAGE_KEY, &value.to_string());
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = value;
    }
}

/// Get the current CPU threads setting, falling back to config default.
pub fn get_cpu_threads(config: Option<&FractalConfig>) -> i32 {
    CPU_THREADS_CACHE.with(|cell| {
        // Load from localStorage on first access
        if cell.get().is_none() {
            if let Some(stored) = load_cpu_threads_from_storage() {
                cell.set(Some(stored));
            }
        }
        cell.get()
            .unwrap_or_else(|| config.map(|c| c.worker_count).unwrap_or(0))
    })
}

/// Set the CPU threads value and persist to localStorage.
pub fn set_cpu_threads(value: i32) {
    CPU_THREADS_CACHE.with(|cell| cell.set(Some(value)));
    save_cpu_threads_to_storage(value);
}

/// Get the raw CPU threads value (None if using default).
pub fn get_cpu_threads_override() -> Option<i32> {
    CPU_THREADS_CACHE.with(|cell| {
        // Load from localStorage on first access
        if cell.get().is_none() {
            if let Some(stored) = load_cpu_threads_from_storage() {
                cell.set(Some(stored));
                return Some(stored);
            }
        }
        cell.get()
    })
}

// =============================================================================
// GPU Setting Persistence (localStorage only, not in URL)
// =============================================================================

/// Load GPU setting from localStorage.
fn load_gpu_setting_from_storage() -> Option<bool> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let storage = window.local_storage().ok()??;
        let value = storage.get_item(GPU_SETTING_STORAGE_KEY).ok()??;
        value.parse().ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Save GPU setting to localStorage.
fn save_gpu_setting_to_storage(value: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(GPU_SETTING_STORAGE_KEY, &value.to_string());
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = value;
    }
}

/// Get the current GPU setting, falling back to config default.
pub fn get_gpu_enabled(config: Option<&FractalConfig>) -> bool {
    GPU_SETTING_CACHE.with(|cell| {
        // Load from localStorage on first access
        if cell.get().is_none() {
            if let Some(stored) = load_gpu_setting_from_storage() {
                cell.set(Some(stored));
            }
        }
        cell.get()
            .unwrap_or_else(|| config.map(|c| c.gpu_enabled).unwrap_or(true))
    })
}

/// Set the GPU setting and persist to localStorage.
pub fn set_gpu_enabled(value: bool) {
    GPU_SETTING_CACHE.with(|cell| cell.set(Some(value)));
    save_gpu_setting_to_storage(value);
}

/// Get the raw GPU setting value (None if using default).
pub fn get_gpu_enabled_override() -> Option<bool> {
    GPU_SETTING_CACHE.with(|cell| {
        // Load from localStorage on first access
        if cell.get().is_none() {
            if let Some(stored) = load_gpu_setting_from_storage() {
                cell.set(Some(stored));
                return Some(stored);
            }
        }
        cell.get()
    })
}

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
    /// Positive: use that exact number of workers.
    /// Zero or negative: use hardware_concurrency + this value (e.g., -1 leaves one core free).
    pub worker_count: i32,
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
pub static FRACTAL_CONFIGS: &[FractalConfig] = &[FractalConfig {
    id: "mandelbrot",
    display_name: "Mandelbrot Set",
    default_center: ("-0.5", "0.0"),
    default_width: "4.0",
    default_height: "4.0",
    renderer_type: RendererType::Perturbation,
    tau_sq: 1e-6,
    worker_count: 0, // all available workers
    iteration_multiplier: 200.0,
    iteration_power: 2.8,
    bigfloat_threshold_bits: 1024, // ~10^300 zoom
    bla_enabled: true,
    gpu_enabled: true,
    gpu_iterations_per_dispatch: 100_000,
    gpu_progressive_row_sets: 16, // 0 = use old tiled renderer, >0 = progressive
}];

/// Look up a fractal configuration by ID.
pub fn get_config(id: &str) -> Option<&'static FractalConfig> {
    FRACTAL_CONFIGS.iter().find(|c| c.id == id)
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
        assert_eq!(config.unwrap().display_name, "Mandelbrot Set");
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
        assert_eq!(config.id, "mandelbrot");
    }
}
