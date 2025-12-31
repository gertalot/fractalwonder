//! Tile rendering for perturbation-based Mandelbrot computation.
//!
//! Provides pure functions for rendering tiles using pre-computed reference orbits.
//! Supports both f64 (fast path) and HDRFloat (deep zoom) precision.

use fractalwonder_core::ComputeData;

/// Statistics from rendering a tile.
#[derive(Clone, Debug, Default)]
#[allow(dead_code)] // Will be used in upcoming tile rendering functions
pub struct TileStats {
    /// Iterations skipped via BLA across all pixels.
    pub bla_iterations: u64,
    /// Total iterations computed (BLA + standard) across all pixels.
    pub total_iterations: u64,
}

/// Result of rendering a tile.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Will be used in upcoming tile rendering functions
pub struct TileRenderResult {
    /// Computed data for each pixel in row-major order.
    pub data: Vec<ComputeData>,
    /// Rendering statistics.
    pub stats: TileStats,
}
