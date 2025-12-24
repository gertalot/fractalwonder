//! Unified colorization pipeline with histogram caching.

use super::palette::{Palette, PaletteLut};
use super::{ColorizerKind, RenderSettings, SmoothIterationContext};
use fractalwonder_core::ComputeData;

/// Unified colorization pipeline.
///
/// Groups all colorization state into one component that can be shared
/// between CPU and GPU render paths via `Rc<RefCell<ColorPipeline>>`.
pub struct ColorPipeline {
    colorizer: ColorizerKind,
    palette: Palette,
    lut: PaletteLut,
    render_settings: RenderSettings,
    cached_context: Option<SmoothIterationContext>,
}

impl ColorPipeline {
    pub fn new(palette: Palette, render_settings: RenderSettings) -> Self {
        let lut = PaletteLut::from_palette(&palette);
        Self {
            colorizer: ColorizerKind::default(),
            palette,
            lut,
            render_settings,
            cached_context: None,
        }
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn render_settings(&self) -> &RenderSettings {
        &self.render_settings
    }

    pub fn set_palette(&mut self, palette: Palette) {
        self.lut = PaletteLut::from_palette(&palette);
        self.palette = palette;
    }

    pub fn set_render_settings(&mut self, settings: RenderSettings) {
        self.render_settings = settings;
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_context = None;
    }

    pub fn colorize_chunk(&self, data: &[ComputeData]) -> Vec<[u8; 4]> {
        data.iter()
            .map(|d| {
                if self.render_settings.xray_enabled {
                    if let ComputeData::Mandelbrot(m) = d {
                        if m.glitched {
                            if m.max_iterations == 0 {
                                return [0, 255, 255, 255];
                            }
                            let normalized = m.iterations as f64 / m.max_iterations as f64;
                            let brightness = (64.0 + normalized * 191.0) as u8;
                            return [0, brightness, brightness, 255];
                        }
                    }
                }

                if let Some(ref ctx) = self.cached_context {
                    self.colorizer.colorize_with_cached_histogram(
                        d,
                        ctx,
                        &self.palette,
                        &self.lut,
                        &self.render_settings,
                    )
                } else {
                    self.colorizer
                        .colorize(d, &self.palette, &self.lut, &self.render_settings)
                }
            })
            .collect()
    }

    pub fn colorize_final(
        &mut self,
        data: &[ComputeData],
        width: usize,
        height: usize,
    ) -> Vec<[u8; 4]> {
        let context = self.colorizer.create_context(data, &self.palette);

        let pixels = self.colorizer.run_pipeline_with_context(
            data,
            &context,
            &self.palette,
            &self.lut,
            &self.render_settings,
            width,
            height,
            self.render_settings.xray_enabled,
        );

        self.cached_context = Some(context);

        pixels
    }
}
