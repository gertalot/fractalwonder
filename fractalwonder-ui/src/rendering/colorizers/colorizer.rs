//! Colorizer trait for mapping compute data to colors.

use super::{Palette, SmoothIterationColorizer};
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

/// Enum of all available colorizer algorithms.
/// Uses enum dispatch to avoid trait object complexity with associated types.
#[derive(Clone, Debug)]
pub enum ColorizerKind {
    SmoothIteration(SmoothIterationColorizer),
}

impl Default for ColorizerKind {
    fn default() -> Self {
        Self::SmoothIteration(SmoothIterationColorizer)
    }
}

impl ColorizerKind {
    /// Run the full colorization pipeline: preprocess → colorize → postprocess.
    pub fn run_pipeline(
        &self,
        data: &[ComputeData],
        palette: &Palette,
        width: usize,
        height: usize,
    ) -> Vec<[u8; 4]> {
        match self {
            Self::SmoothIteration(c) => {
                #[allow(clippy::let_unit_value)]
                let ctx = c.preprocess(data);
                let mut pixels: Vec<[u8; 4]> =
                    data.iter().map(|d| c.colorize(d, &ctx, palette)).collect();
                c.postprocess(&mut pixels, data, &ctx, palette, width, height);
                pixels
            }
        }
    }

    /// Quick colorization for progressive rendering (no pre/post processing).
    pub fn colorize_quick(&self, data: &ComputeData, palette: &Palette) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => c.colorize(data, &(), palette),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::{Palette, SmoothIterationColorizer};
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn colorizer_kind_runs_pipeline() {
        let colorizer = ColorizerKind::SmoothIteration(SmoothIterationColorizer);
        let palette = Palette::grayscale();

        let data = vec![
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 500,
                max_iterations: 1000,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 0.0,
            }),
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 0,
                max_iterations: 1000,
                escaped: false,
                glitched: false,
                final_z_norm_sq: 0.0,
            }),
        ];

        let pixels = colorizer.run_pipeline(&data, &palette, 2, 1);

        assert_eq!(pixels.len(), 2);
        // First pixel: escaped at 50% should be mid-gray
        assert!(
            pixels[0][0] > 50 && pixels[0][0] < 150,
            "Expected mid gray, got {:?}",
            pixels[0]
        );
        // Second pixel: interior should be black
        assert_eq!(pixels[1], [0, 0, 0, 255]);
    }
}
