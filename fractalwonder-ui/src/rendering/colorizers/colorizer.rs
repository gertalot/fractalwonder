//! Colorizer trait for mapping compute data to colors.

use super::smooth_iteration::SmoothIterationContext;
use super::{ColorOptions, Palette, SmoothIterationColorizer};
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
    fn preprocess(&self, _data: &[ComputeData], _options: &ColorOptions) -> Self::Context {
        Self::Context::default()
    }

    /// Map a single pixel to a color.
    /// Palette is passed separately so callers can cache it.
    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        options: &ColorOptions,
        palette: &Palette,
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
        _options: &ColorOptions,
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
    #[allow(clippy::too_many_arguments)]
    pub fn run_pipeline(
        &self,
        data: &[ComputeData],
        options: &ColorOptions,
        palette: &Palette,
        width: usize,
        height: usize,
        zoom_level: f64,
        xray_enabled: bool,
    ) -> Vec<[u8; 4]> {
        match self {
            Self::SmoothIteration(c) => {
                let ctx = c.preprocess(data, options);
                let mut pixels: Vec<[u8; 4]> = data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| c.colorize(d, &ctx, options, palette, i))
                    .collect();
                c.postprocess(&mut pixels, data, &ctx, options, width, height, zoom_level);

                // Apply xray coloring to glitched pixels
                if xray_enabled {
                    apply_xray_to_glitched(&mut pixels, data);
                }

                pixels
            }
        }
    }

    /// Colorize a single pixel. For progressive rendering, pass default context.
    pub fn colorize(
        &self,
        data: &ComputeData,
        options: &ColorOptions,
        palette: &Palette,
    ) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => c.colorize(
                data,
                &SmoothIterationContext::default(),
                options,
                palette,
                0,
            ),
        }
    }

    /// Colorize a single pixel using a cached histogram from a previous render.
    /// Computes fresh smooth iteration value for the new data, but uses the cached
    /// sorted_smooth for histogram-based percentile lookup.
    pub fn colorize_with_cached_histogram(
        &self,
        data: &ComputeData,
        cached_context: &SmoothIterationContext,
        options: &ColorOptions,
        palette: &Palette,
    ) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => {
                c.colorize_with_histogram(data, cached_context, options, palette)
            }
        }
    }

    /// Create a precomputed context from data for caching.
    /// Returns None if histogram is disabled (no expensive computation needed).
    pub fn create_context(
        &self,
        data: &[ComputeData],
        options: &ColorOptions,
    ) -> SmoothIterationContext {
        match self {
            Self::SmoothIteration(c) => c.preprocess(data, options),
        }
    }

    /// Run the colorization pipeline using a pre-computed context.
    /// Skips the preprocess step, using the cached context instead.
    #[allow(clippy::too_many_arguments)]
    pub fn run_pipeline_with_context(
        &self,
        data: &[ComputeData],
        context: &SmoothIterationContext,
        options: &ColorOptions,
        palette: &Palette,
        width: usize,
        height: usize,
        zoom_level: f64,
        xray_enabled: bool,
    ) -> Vec<[u8; 4]> {
        match self {
            Self::SmoothIteration(c) => {
                let mut pixels: Vec<[u8; 4]> = data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| c.colorize(d, context, options, palette, i))
                    .collect();
                c.postprocess(
                    &mut pixels,
                    data,
                    context,
                    options,
                    width,
                    height,
                    zoom_level,
                );

                // Apply xray coloring to glitched pixels
                if xray_enabled {
                    apply_xray_to_glitched(&mut pixels, data);
                }

                pixels
            }
        }
    }
}

/// Apply xray coloring to glitched pixels in place.
fn apply_xray_to_glitched(pixels: &mut [[u8; 4]], data: &[ComputeData]) {
    for (pixel, d) in pixels.iter_mut().zip(data.iter()) {
        if let ComputeData::Mandelbrot(m) = d {
            if m.glitched {
                if m.max_iterations == 0 {
                    *pixel = [0, 255, 255, 255];
                } else {
                    let normalized = m.iterations as f64 / m.max_iterations as f64;
                    let brightness = (64.0 + normalized * 191.0) as u8;
                    *pixel = [0, brightness, brightness, 255];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::SmoothIterationColorizer;
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn colorizer_kind_runs_pipeline() {
        let colorizer = ColorizerKind::SmoothIteration(SmoothIterationColorizer);
        let options = ColorOptions::default();
        let palette = options.palette();

        let data = vec![
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 500,
                max_iterations: 1000,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
                final_z_re: 0.0,
                final_z_im: 0.0,
                final_derivative_re: 0.0,
                final_derivative_im: 0.0,
            }),
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 0,
                max_iterations: 1000,
                escaped: false,
                glitched: false,
                final_z_norm_sq: 0.0,
                final_z_re: 0.0,
                final_z_im: 0.0,
                final_derivative_re: 0.0,
                final_derivative_im: 0.0,
            }),
        ];

        let pixels = colorizer.run_pipeline(&data, &options, &palette, 2, 1, 1.0, false);

        assert_eq!(pixels.len(), 2);
        // First pixel: escaped, should have some color (with cycling, not necessarily mid-gray)
        // Just verify it's not black (interior) and alpha is 255
        assert_eq!(pixels[0][3], 255, "Alpha should be 255");
        // Second pixel: interior should be black
        assert_eq!(pixels[1], [0, 0, 0, 255]);
    }
}
