//! Unified colorization pipeline with histogram caching.

use super::{ColorOptions, ColorizerKind, Palette, SmoothIterationContext};
use fractalwonder_core::ComputeData;

/// Unified colorization pipeline.
///
/// Groups all colorization state into one component that can be shared
/// between CPU and GPU render paths via `Rc<RefCell<ColorPipeline>>`.
pub struct ColorPipeline {
    colorizer: ColorizerKind,
    options: ColorOptions,
    palette: Palette,
    cached_context: Option<SmoothIterationContext>,
    xray_enabled: bool,
}

impl ColorPipeline {
    /// Create a new pipeline with default colorizer.
    pub fn new(options: ColorOptions) -> Self {
        let palette = options.palette();
        Self {
            colorizer: ColorizerKind::default(),
            options,
            palette,
            cached_context: None,
            xray_enabled: false,
        }
    }

    /// Get current color options.
    pub fn options(&self) -> &ColorOptions {
        &self.options
    }

    /// Update color options. Rebuilds palette cache.
    pub fn set_options(&mut self, options: ColorOptions) {
        self.palette = options.palette();
        self.options = options;
    }

    /// Set xray mode.
    pub fn set_xray(&mut self, enabled: bool) {
        self.xray_enabled = enabled;
    }

    /// Get xray mode.
    pub fn xray_enabled(&self) -> bool {
        self.xray_enabled
    }

    /// Invalidate histogram cache (call on navigation).
    pub fn invalidate_cache(&mut self) {
        self.cached_context = None;
    }

    /// Colorize a chunk during progressive rendering.
    ///
    /// Uses cached histogram from previous render if available.
    /// Does NOT update cache - intermediate results shouldn't pollute cache.
    pub fn colorize_chunk(&self, data: &[ComputeData]) -> Vec<[u8; 4]> {
        data.iter()
            .map(|d| {
                // Handle xray mode for glitched pixels
                if self.xray_enabled {
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

                // Use cached context if available, otherwise simple colorization
                if let Some(ref ctx) = self.cached_context {
                    self.colorizer.colorize_with_cached_histogram(
                        d,
                        ctx,
                        &self.options,
                        &self.palette,
                    )
                } else {
                    self.colorizer.colorize(d, &self.options, &self.palette)
                }
            })
            .collect()
    }

    /// Colorize complete frame with full pipeline.
    ///
    /// Builds fresh histogram, applies shading, updates cache for next render.
    pub fn colorize_final(
        &mut self,
        data: &[ComputeData],
        width: usize,
        height: usize,
        zoom_level: f64,
    ) -> Vec<[u8; 4]> {
        // Build new context (histogram) from complete data
        let context = self.colorizer.create_context(data, &self.options);

        // Run full pipeline with new context
        let pixels = self.colorizer.run_pipeline_with_context(
            data,
            &context,
            &self.options,
            &self.palette,
            width,
            height,
            zoom_level,
            self.xray_enabled,
        );

        // Cache context for next progressive render
        self.cached_context = Some(context);

        pixels
    }
}
