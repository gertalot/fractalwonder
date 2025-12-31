//! Tile rendering for perturbation-based Mandelbrot computation.
//!
//! Provides pure functions for rendering tiles using pre-computed reference orbits.
//! Supports both f64 (fast path) and HDRFloat (deep zoom) precision.

use super::{compute_pixel_perturbation, ReferenceOrbit};
use fractalwonder_core::{ComplexDelta, ComputeData, F64Complex};

/// Statistics from rendering a tile.
#[derive(Clone, Debug, Default)]
pub struct TileStats {
    /// Iterations skipped via BLA across all pixels.
    #[allow(dead_code)] // Used by HDRFloat tile renderer (Task 5)
    pub bla_iterations: u64,
    /// Total iterations computed (BLA + standard) across all pixels.
    pub total_iterations: u64,
}

/// Result of rendering a tile.
#[derive(Clone, Debug)]
pub struct TileRenderResult {
    /// Computed data for each pixel in row-major order.
    pub data: Vec<ComputeData>,
    /// Rendering statistics.
    #[allow(dead_code)] // Used by worker integration (Task 6)
    pub stats: TileStats,
}

/// Configuration for tile rendering.
#[derive(Clone, Debug)]
pub struct TileConfig {
    /// Tile dimensions (width, height).
    pub size: (u32, u32),
    /// Maximum iterations for escape check.
    pub max_iterations: u32,
    /// Glitch detection threshold squared (τ²).
    pub tau_sq: f64,
    /// Enable BLA acceleration (only applies to HDRFloat path).
    #[allow(dead_code)] // Used by HDRFloat tile renderer (Task 5)
    pub bla_enabled: bool,
}

/// Render a tile using f64 precision (fast path for moderate zoom levels).
///
/// This path is used when delta values fit comfortably in f64 range (~10^±300).
/// BLA is not supported in this path.
///
/// # Arguments
/// * `orbit` - Pre-computed reference orbit
/// * `delta_origin` - Delta from reference point to top-left pixel (re, im)
/// * `delta_step` - Delta step between pixels (re, im)
/// * `config` - Tile rendering configuration
///
/// # Returns
/// Computed pixel data and rendering statistics
pub fn render_tile_f64(
    orbit: &ReferenceOrbit,
    delta_origin: (f64, f64),
    delta_step: (f64, f64),
    config: &TileConfig,
) -> TileRenderResult {
    let capacity = (config.size.0 * config.size.1) as usize;
    let mut data = Vec::with_capacity(capacity);
    let mut stats = TileStats::default();

    let mut delta_c_row = delta_origin;

    for _py in 0..config.size.1 {
        let mut delta_c = delta_c_row;

        for _px in 0..config.size.0 {
            let result = compute_pixel_perturbation(
                orbit,
                F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                config.max_iterations,
                config.tau_sq,
            );
            stats.total_iterations += result.iterations as u64;
            data.push(ComputeData::Mandelbrot(result));

            delta_c.0 += delta_step.0;
        }

        delta_c_row.1 += delta_step.1;
    }

    TileRenderResult { data, stats }
}
