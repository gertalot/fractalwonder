//! Tile rendering for perturbation-based Mandelbrot computation.
//!
//! Provides pure functions for rendering tiles using pre-computed reference orbits.
//! Supports both f64 (fast path) and HDRFloat (deep zoom) precision.

use super::{
    compute_pixel_perturbation, compute_pixel_perturbation_f64_bla,
    compute_pixel_perturbation_hdr_bla, ReferenceOrbit,
};
use crate::BlaTable;
use fractalwonder_core::{ComplexDelta, ComputeData, F64Complex, HDRComplex, HDRFloat};

/// Statistics from rendering a tile.
#[derive(Clone, Debug, Default)]
pub struct TileStats {
    /// Iterations skipped via BLA across all pixels.
    #[allow(dead_code)] // Used by HDRFloat tile renderer (Task 5)
    pub bla_iterations: u64,
    /// Total iterations computed (BLA + standard) across all pixels.
    pub total_iterations: u64,
    /// Total rebase count across all pixels.
    pub rebase_count: u64,
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
    /// Enable BLA acceleration.
    pub bla_enabled: bool,
}

/// Render a tile using f64 precision with optional BLA acceleration.
///
/// This path is used when delta values fit comfortably in f64 range (~10^±300).
/// BLA acceleration is applied when available and enabled.
///
/// # Arguments
/// * `orbit` - Pre-computed reference orbit
/// * `bla_table` - Optional BLA table for iteration skipping
/// * `delta_origin` - Delta from reference point to top-left pixel (re, im)
/// * `delta_step` - Delta step between pixels (re, im)
/// * `config` - Tile rendering configuration
///
/// # Returns
/// Computed pixel data and rendering statistics including BLA metrics
pub fn render_tile_f64(
    orbit: &ReferenceOrbit,
    bla_table: Option<&BlaTable>,
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
            if config.bla_enabled {
                if let Some(bla) = bla_table {
                    let (result, pixel_stats) = compute_pixel_perturbation_f64_bla(
                        orbit,
                        bla,
                        delta_c,
                        config.max_iterations,
                        config.tau_sq,
                    );
                    stats.bla_iterations += pixel_stats.bla_iterations as u64;
                    stats.total_iterations += pixel_stats.total_iterations as u64;
                    stats.rebase_count += pixel_stats.rebase_count as u64;
                    data.push(ComputeData::Mandelbrot(result));
                } else {
                    // BLA enabled but no table - fall back to generic f64 path
                    let result = compute_pixel_perturbation(
                        orbit,
                        F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                        config.max_iterations,
                        config.tau_sq,
                    );
                    stats.total_iterations += result.iterations as u64;
                    data.push(ComputeData::Mandelbrot(result));
                }
            } else {
                // BLA disabled - use generic f64 path
                let result = compute_pixel_perturbation(
                    orbit,
                    F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                    config.max_iterations,
                    config.tau_sq,
                );
                stats.total_iterations += result.iterations as u64;
                data.push(ComputeData::Mandelbrot(result));
            }

            delta_c.0 += delta_step.0;
        }

        delta_c_row.1 += delta_step.1;
    }

    TileRenderResult { data, stats }
}

/// Render a tile using HDRFloat precision with optional BLA acceleration.
///
/// This path handles arbitrary exponent ranges, necessary for deep zoom
/// where f64 would underflow. BLA acceleration is applied when available.
///
/// # Arguments
/// * `orbit` - Pre-computed reference orbit
/// * `bla_table` - Optional BLA table for iteration skipping
/// * `delta_origin` - Delta from reference point to top-left pixel (re, im)
/// * `delta_step` - Delta step between pixels (re, im)
/// * `config` - Tile rendering configuration
///
/// # Returns
/// Computed pixel data and rendering statistics including BLA metrics
pub fn render_tile_hdr(
    orbit: &ReferenceOrbit,
    bla_table: Option<&BlaTable>,
    delta_origin: (HDRFloat, HDRFloat),
    delta_step: (HDRFloat, HDRFloat),
    config: &TileConfig,
) -> TileRenderResult {
    let capacity = (config.size.0 * config.size.1) as usize;
    let mut data = Vec::with_capacity(capacity);
    let mut stats = TileStats::default();

    let delta_origin_complex = HDRComplex {
        re: delta_origin.0,
        im: delta_origin.1,
    };
    let delta_step_complex = HDRComplex {
        re: delta_step.0,
        im: delta_step.1,
    };

    let mut delta_c_row = delta_origin_complex;

    for _py in 0..config.size.1 {
        let mut delta_c = delta_c_row;

        for _px in 0..config.size.0 {
            if config.bla_enabled {
                if let Some(bla) = bla_table {
                    let (result, pixel_stats) = compute_pixel_perturbation_hdr_bla(
                        orbit,
                        bla,
                        delta_c,
                        config.max_iterations,
                        config.tau_sq,
                    );
                    stats.bla_iterations += pixel_stats.bla_iterations as u64;
                    stats.total_iterations += pixel_stats.total_iterations as u64;
                    stats.rebase_count += pixel_stats.rebase_count as u64;
                    data.push(ComputeData::Mandelbrot(result));
                } else {
                    // BLA enabled but no table - fall back to generic HDRComplex path
                    let result = compute_pixel_perturbation(
                        orbit,
                        delta_c,
                        config.max_iterations,
                        config.tau_sq,
                    );
                    stats.total_iterations += result.iterations as u64;
                    data.push(ComputeData::Mandelbrot(result));
                }
            } else {
                // BLA disabled - use generic HDRComplex path
                let result = compute_pixel_perturbation(
                    orbit,
                    delta_c,
                    config.max_iterations,
                    config.tau_sq,
                );
                stats.total_iterations += result.iterations as u64;
                data.push(ComputeData::Mandelbrot(result));
            }

            delta_c = HDRComplex {
                re: delta_c.re.add(&delta_step_complex.re),
                im: delta_c.im,
            };
        }

        delta_c_row = HDRComplex {
            re: delta_c_row.re,
            im: delta_c_row.im.add(&delta_step_complex.im),
        };
    }

    TileRenderResult { data, stats }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bla::BlaTable;
    use crate::ReferenceOrbit;
    use fractalwonder_core::{BigFloat, HDRFloat};

    #[test]
    fn render_tile_f64_with_bla_uses_bla() {
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 1000);
        let bla_table = BlaTable::compute(&orbit, &HDRFloat::from_f64(1e-10));

        let config = TileConfig {
            size: (4, 4),
            max_iterations: 1000,
            tau_sq: 1e-6,
            bla_enabled: true,
        };

        // Small deltas to trigger BLA
        let delta_origin = (1e-12, 1e-12);
        let delta_step = (1e-14, 1e-14);

        let result = render_tile_f64(&orbit, Some(&bla_table), delta_origin, delta_step, &config);

        // Should have used BLA for at least some iterations
        assert!(
            result.stats.bla_iterations > 0,
            "BLA should be used in f64 path, got bla_iters={}",
            result.stats.bla_iterations
        );
    }
}
