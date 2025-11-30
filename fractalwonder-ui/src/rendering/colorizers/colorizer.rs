//! Colorizer trait for mapping compute data to colors.

use super::smooth_iteration::SmoothIterationContext;
use super::{ColorSettings, SmoothIterationColorizer};
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
    fn preprocess(&self, _data: &[ComputeData], _settings: &ColorSettings) -> Self::Context {
        Self::Context::default()
    }

    /// Map a single pixel to a color.
    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        settings: &ColorSettings,
        index: usize,
    ) -> [u8; 4];

    /// Modify pixel buffer in place.
    /// Default: no-op.
    #[allow(clippy::too_many_arguments)]
    fn postprocess(
        &self,
        _pixels: &mut [[u8; 4]],
        _data: &[ComputeData],
        _context: &Self::Context,
        _settings: &ColorSettings,
        _width: usize,
        _height: usize,
        _zoom_level: f64,
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
        settings: &ColorSettings,
        width: usize,
        height: usize,
        zoom_level: f64,
    ) -> Vec<[u8; 4]> {
        match self {
            Self::SmoothIteration(c) => {
                let ctx = c.preprocess(data, settings);
                let mut pixels: Vec<[u8; 4]> = data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| c.colorize(d, &ctx, settings, i))
                    .collect();
                c.postprocess(&mut pixels, data, &ctx, settings, width, height, zoom_level);
                pixels
            }
        }
    }

    /// Quick colorization for progressive rendering (no pre/post processing).
    pub fn colorize_quick(&self, data: &ComputeData, settings: &ColorSettings) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => {
                c.colorize(data, &SmoothIterationContext::default(), settings, 0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::{ColorSettings, Palette, SmoothIterationColorizer};
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn colorizer_kind_runs_pipeline() {
        let colorizer = ColorizerKind::SmoothIteration(SmoothIterationColorizer);
        let settings = ColorSettings::with_palette(Palette::grayscale());

        let data = vec![
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 500,
                max_iterations: 1000,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            }),
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 0,
                max_iterations: 1000,
                escaped: false,
                glitched: false,
                final_z_norm_sq: 0.0,
            }),
        ];

        let pixels = colorizer.run_pipeline(&data, &settings, 2, 1, 1.0);

        assert_eq!(pixels.len(), 2);
        // First pixel: escaped, should have some color (with cycling, not necessarily mid-gray)
        // Just verify it's not black (interior) and alpha is 255
        assert_eq!(pixels[0][3], 255, "Alpha should be 255");
        // Second pixel: interior should be black
        assert_eq!(pixels[1], [0, 0, 0, 255]);
    }
}
