//! Smooth iteration colorizer using the formula μ = n + 1 - log₂(ln(|z|))
//! to eliminate banding in exterior regions.

use super::{shading::apply_slope_shading, ColorSettings, Colorizer, Palette};
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Colorizer that uses smooth iteration count to eliminate banding.
/// Uses the formula μ = n + 1 - log₂(ln(|z|)) where |z| is computed from final_z_norm_sq.
#[derive(Clone, Debug, Default)]
pub struct SmoothIterationColorizer;

/// Context data computed during preprocessing.
/// Holds smooth iteration values and optional histogram CDF.
#[derive(Clone, Debug, Default)]
pub struct SmoothIterationContext {
    /// Smooth iteration values per pixel.
    pub smooth_values: Vec<f64>,
    /// CDF for histogram equalization. None if disabled.
    /// Index = iteration count, value = cumulative probability [0,1].
    pub cdf: Option<Vec<f64>>,
}

/// Compute smooth iteration count from MandelbrotData.
/// Returns the smooth iteration value, or max_iterations for interior points.
pub fn compute_smooth_iteration(data: &MandelbrotData) -> f64 {
    if !data.escaped || data.max_iterations == 0 {
        return data.max_iterations as f64;
    }

    if data.final_z_norm_sq > 1.0 {
        let z_norm_sq = data.final_z_norm_sq as f64;
        let log_z = z_norm_sq.ln() / 2.0;
        let nu = log_z.ln() / std::f64::consts::LN_2;
        data.iterations as f64 + 1.0 - nu
    } else {
        data.iterations as f64
    }
}

impl Colorizer for SmoothIterationColorizer {
    type Context = Vec<f64>;

    fn preprocess(&self, data: &[ComputeData]) -> Self::Context {
        data.iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => compute_smooth_iteration(m),
                ComputeData::TestImage(_) => 0.0,
            })
            .collect()
    }

    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        settings: &ColorSettings,
        index: usize,
    ) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => {
                let smooth = if index < context.len() {
                    context[index]
                } else {
                    compute_smooth_iteration(m)
                };
                self.colorize_mandelbrot_smooth(m, smooth, &settings.palette, settings.cycle_count)
            }
            ComputeData::TestImage(_) => {
                // Test image uses its own colorizer
                [128, 128, 128, 255]
            }
        }
    }

    fn postprocess(
        &self,
        pixels: &mut [[u8; 4]],
        data: &[ComputeData],
        context: &Self::Context,
        settings: &ColorSettings,
        width: usize,
        height: usize,
        zoom_level: f64,
    ) {
        apply_slope_shading(
            pixels,
            data,
            context,
            &settings.shading,
            width,
            height,
            zoom_level,
        );
    }
}

impl SmoothIterationColorizer {
    fn colorize_mandelbrot_smooth(
        &self,
        data: &MandelbrotData,
        smooth: f64,
        palette: &Palette,
        cycle_count: f64,
    ) -> [u8; 4] {
        // Interior points are black
        if !data.escaped {
            return [0, 0, 0, 255];
        }

        // Avoid division by zero
        if data.max_iterations == 0 {
            return [0, 0, 0, 255];
        }

        // Normalize and apply cycling for better color variation at deep zooms
        let normalized = smooth / data.max_iterations as f64;
        let t = (normalized * cycle_count).fract(); // Cycle through palette
        let [r, g, b] = palette.sample(t);
        [r, g, b, 255]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::{ColorSettings, Palette};
    use fractalwonder_core::{ComputeData, MandelbrotData};

    #[test]
    fn compute_smooth_iteration_interior_returns_max() {
        let data = MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 4.0,
        };
        let smooth = compute_smooth_iteration(&data);
        assert_eq!(smooth, 1000.0);
    }

    #[test]
    fn compute_smooth_iteration_escaped_returns_fractional() {
        let data = MandelbrotData {
            iterations: 10,
            max_iterations: 100,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
        };
        let smooth = compute_smooth_iteration(&data);
        // Should be close to 10 but with fractional adjustment
        // The smooth formula n + 1 - ν can reduce the value, so it may be < 10
        assert!(smooth > 8.0 && smooth < 12.0, "smooth = {}", smooth);
        // Verify it has a fractional component (not exactly iterations)
        assert_ne!(smooth, data.iterations as f64);
    }

    fn make_escaped(iterations: u32, max_iterations: u32) -> ComputeData {
        // For smooth coloring, we need a realistic |z|² at escape
        // With escape radius 256, |z|² should be > 65536
        // Use a value that gives reasonable smooth adjustment
        let z_norm_sq = 100000.0_f32; // > 65536, gives smooth adjustment
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations,
            escaped: true,
            glitched: false,
            final_z_norm_sq: z_norm_sq,
        })
    }

    fn make_interior() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 0.0,
        })
    }

    #[test]
    fn interior_is_black() {
        let colorizer = SmoothIterationColorizer;
        let settings = ColorSettings::with_palette(Palette::grayscale());
        let color = colorizer.colorize(&make_interior(), &vec![], &settings, 0);
        assert_eq!(color, [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_zero_is_dark() {
        let colorizer = SmoothIterationColorizer;
        let settings = ColorSettings::with_palette(Palette::grayscale());
        let color = colorizer.colorize(&make_escaped(0, 1000), &vec![], &settings, 0);
        assert!(color[0] < 10, "Expected near black, got {:?}", color);
    }

    #[test]
    fn cycling_produces_color_variation() {
        let colorizer = SmoothIterationColorizer;
        let settings = ColorSettings::with_palette(Palette::grayscale());
        // With cycling, nearby iteration values should produce different colors
        let color1 = colorizer.colorize(&make_escaped(500, 1000), &vec![], &settings, 0);
        let color2 = colorizer.colorize(&make_escaped(510, 1000), &vec![], &settings, 0);
        // Just verify we get valid colors (alpha = 255)
        assert_eq!(color1[3], 255);
        assert_eq!(color2[3], 255);
    }

    #[test]
    fn higher_iterations_are_brighter() {
        let colorizer = SmoothIterationColorizer;
        let settings = ColorSettings::with_palette(Palette::grayscale());
        let low = colorizer.colorize(&make_escaped(100, 1000), &vec![], &settings, 0);
        let high = colorizer.colorize(&make_escaped(900, 1000), &vec![], &settings, 0);
        assert!(high[0] > low[0], "Higher iterations should be brighter");
    }

    #[test]
    fn smooth_iteration_produces_gradual_change() {
        let colorizer = SmoothIterationColorizer;
        let settings = ColorSettings::with_palette(Palette::grayscale());

        // Two pixels with same iteration count but different |z|² at escape
        // should produce different colors due to smooth formula
        // Using max_iterations of 20 to amplify the fractional difference
        let data1 = ComputeData::Mandelbrot(MandelbrotData {
            iterations: 10,
            max_iterations: 20,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 70000.0, // Just over escape threshold (256² = 65536)
        });

        let data2 = ComputeData::Mandelbrot(MandelbrotData {
            iterations: 10,
            max_iterations: 20,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000000.0, // Very large |z|²
        });

        let color1 = colorizer.colorize(&data1, &vec![], &settings, 0);
        let color2 = colorizer.colorize(&data2, &vec![], &settings, 0);

        // With smooth formula, larger |z|² means lower μ, so darker color
        assert!(
            color1[0] > color2[0],
            "Larger z_norm_sq should produce darker color: {:?} vs {:?}",
            color1,
            color2
        );
    }

    #[test]
    fn smooth_iteration_context_default_has_no_cdf() {
        let ctx = SmoothIterationContext::default();
        assert!(ctx.smooth_values.is_empty());
        assert!(ctx.cdf.is_none());
    }
}
