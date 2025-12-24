//! Colorizer trait for mapping compute data to colors.

use super::smooth_iteration::SmoothIterationContext;
use super::{Palette, PaletteLut, RenderSettings, SmoothIterationColorizer};
use fractalwonder_core::ComputeData;

/// A colorizer algorithm with optional pre/post-processing stages.
///
/// # Pipeline Flow
/// 1. `preprocess` - analyze all pixels, build context (e.g., histogram CDF)
/// 2. `colorize` - map each pixel to a color using context and palette
/// 3. `postprocess` - modify pixel buffer in place (e.g., slope shading)
pub trait Colorizer {
    type Context: Default;

    fn preprocess(&self, _data: &[ComputeData], _palette: &Palette) -> Self::Context {
        Self::Context::default()
    }

    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
        index: usize,
    ) -> [u8; 4];

    #[allow(clippy::too_many_arguments)]
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
    #[allow(clippy::too_many_arguments)]
    pub fn run_pipeline(
        &self,
        data: &[ComputeData],
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
        width: usize,
        height: usize,
        xray_enabled: bool,
    ) -> Vec<[u8; 4]> {
        match self {
            Self::SmoothIteration(c) => {
                let ctx = c.preprocess(data, palette);
                let mut pixels: Vec<[u8; 4]> = data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| c.colorize(d, &ctx, palette, lut, render_settings, i))
                    .collect();
                c.postprocess(&mut pixels, data, &ctx, palette, width, height);

                if xray_enabled {
                    apply_xray_to_glitched(&mut pixels, data);
                }

                pixels
            }
        }
    }

    pub fn colorize(
        &self,
        data: &ComputeData,
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
    ) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => c.colorize(
                data,
                &SmoothIterationContext::default(),
                palette,
                lut,
                render_settings,
                0,
            ),
        }
    }

    pub fn colorize_with_cached_histogram(
        &self,
        data: &ComputeData,
        cached_context: &SmoothIterationContext,
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
    ) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => {
                c.colorize_with_histogram(data, cached_context, palette, lut, render_settings)
            }
        }
    }

    pub fn create_context(
        &self,
        data: &[ComputeData],
        palette: &Palette,
    ) -> SmoothIterationContext {
        match self {
            Self::SmoothIteration(c) => c.preprocess(data, palette),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_pipeline_with_context(
        &self,
        data: &[ComputeData],
        context: &SmoothIterationContext,
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
        width: usize,
        height: usize,
        xray_enabled: bool,
    ) -> Vec<[u8; 4]> {
        match self {
            Self::SmoothIteration(c) => {
                let mut pixels: Vec<[u8; 4]> = data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| c.colorize(d, context, palette, lut, render_settings, i))
                    .collect();
                c.postprocess(&mut pixels, data, context, palette, width, height);

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
    use crate::rendering::colorizers::{
        Palette, PaletteLut, RenderSettings, SmoothIterationColorizer,
    };
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn colorizer_kind_runs_pipeline() {
        use futures::executor::block_on;

        block_on(Palette::factory_defaults()); // ensure loaded
        let palette = block_on(Palette::get("classic")).unwrap();
        let lut = PaletteLut::from_palette(&palette);
        let render_settings = RenderSettings::default();
        let colorizer = ColorizerKind::SmoothIteration(SmoothIterationColorizer);

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

        let pixels = colorizer.run_pipeline(&data, &palette, &lut, &render_settings, 2, 1, false);

        assert_eq!(pixels.len(), 2);
        assert_eq!(pixels[0][3], 255);
        assert_eq!(pixels[1], [0, 0, 0, 255]);
    }
}
