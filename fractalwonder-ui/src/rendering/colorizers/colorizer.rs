//! Colorizer trait for mapping compute data to colors.

use super::Palette;
use fractalwonder_core::ComputeData;

/// A colorizer algorithm with optional pre/post-processing stages.
///
/// # Pipeline Flow
/// 1. `preprocess` - analyze all pixels, build context (e.g., histogram CDF)
/// 2. `colorize` - map each pixel to a color using context and palette
/// 3. `postprocess` - modify pixel buffer in place (e.g., slope shading)
pub trait Colorizer {
    /// Data passed from preprocess to colorize/postprocess.
    type Context: Default;

    /// Analyze all pixels, build context.
    /// Default: no-op, returns `Default::default()`.
    fn preprocess(&self, _data: &[ComputeData]) -> Self::Context {
        Self::Context::default()
    }

    /// Map a single pixel to a color.
    fn colorize(&self, data: &ComputeData, context: &Self::Context, palette: &Palette) -> [u8; 4];

    /// Modify pixel buffer in place.
    /// Default: no-op.
    fn postprocess(
        &self,
        _pixels: &mut [[u8; 4]],
        _data: &[ComputeData],
        _context: &Self::Context,
        _palette: &Palette,
        _width: usize,
        _height: usize,
    ) {
    }
}
