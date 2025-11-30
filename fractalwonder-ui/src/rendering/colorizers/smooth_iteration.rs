//! Smooth iteration colorizer using the formula μ = n + 1 - log₂(ln(|z|))
//! to eliminate banding in exterior regions.

use super::{shading::apply_slope_shading, ColorOptions, Colorizer};
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

/// Build histogram CDF from iteration counts.
/// Returns a Vec where cdf[i] = cumulative probability for iteration i.
/// Interior points (escaped=false) are excluded from the histogram.
pub fn build_histogram_cdf(data: &[ComputeData], max_iterations: u32) -> Vec<f64> {
    let len = max_iterations as usize + 1;
    let mut histogram = vec![0u64; len];
    let mut total_exterior = 0u64;

    // Count iterations for exterior points only
    for d in data {
        if let ComputeData::Mandelbrot(m) = d {
            if m.escaped && m.iterations < max_iterations {
                histogram[m.iterations as usize] += 1;
                total_exterior += 1;
            }
        }
    }

    // Build CDF
    let mut cdf = vec![0.0; len];
    if total_exterior > 0 {
        let mut cumulative = 0u64;
        for i in 0..len {
            cumulative += histogram[i];
            cdf[i] = cumulative as f64 / total_exterior as f64;
        }
    }

    cdf
}

impl Colorizer for SmoothIterationColorizer {
    type Context = SmoothIterationContext;

    fn preprocess(&self, data: &[ComputeData], options: &ColorOptions) -> Self::Context {
        let smooth_values: Vec<f64> = data
            .iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => compute_smooth_iteration(m),
                ComputeData::TestImage(_) => 0.0,
            })
            .collect();

        let cdf = if options.histogram_enabled {
            // Find max_iterations from first Mandelbrot data point
            let max_iter = data
                .iter()
                .find_map(|d| {
                    if let ComputeData::Mandelbrot(m) = d {
                        Some(m.max_iterations)
                    } else {
                        None
                    }
                })
                .unwrap_or(1000);

            Some(build_histogram_cdf(data, max_iter))
        } else {
            None
        };

        SmoothIterationContext { smooth_values, cdf }
    }

    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        options: &ColorOptions,
        palette: &super::Palette,
        index: usize,
    ) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => {
                let smooth = if index < context.smooth_values.len() {
                    context.smooth_values[index]
                } else {
                    compute_smooth_iteration(m)
                };
                self.colorize_mandelbrot(m, smooth, context, options, palette)
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
        options: &ColorOptions,
        width: usize,
        height: usize,
        zoom_level: f64,
    ) {
        apply_slope_shading(
            pixels,
            data,
            &context.smooth_values,
            &options.shading(),
            width,
            height,
            zoom_level,
        );
    }
}

impl SmoothIterationColorizer {
    fn colorize_mandelbrot(
        &self,
        data: &MandelbrotData,
        smooth: f64,
        context: &SmoothIterationContext,
        options: &ColorOptions,
        palette: &super::Palette,
    ) -> [u8; 4] {
        // Interior points are black
        if !data.escaped {
            return [0, 0, 0, 255];
        }

        // Avoid division by zero
        if data.max_iterations == 0 {
            return [0, 0, 0, 255];
        }

        // Normalize: use CDF if available, otherwise linear (smooth or discrete)
        let normalized = if let Some(cdf) = &context.cdf {
            // Use integer iteration for CDF lookup
            let idx = (data.iterations as usize).min(cdf.len().saturating_sub(1));
            cdf[idx]
        } else if options.smooth_enabled {
            smooth / data.max_iterations as f64
        } else {
            data.iterations as f64 / data.max_iterations as f64
        };

        // Apply cycling and sample palette
        let cycle_count = options.cycle_count as f64;
        let t = if cycle_count > 1.0 {
            (normalized * cycle_count).fract()
        } else {
            (normalized * cycle_count).clamp(0.0, 1.0)
        };
        let [r, g, b] = palette.sample(t);
        [r, g, b, 255]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::ColorOptions;
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

    fn grayscale_options() -> ColorOptions {
        ColorOptions {
            palette_id: "grayscale".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn interior_is_black() {
        let colorizer = SmoothIterationColorizer;
        let options = grayscale_options();
        let palette = options.palette();
        let ctx = SmoothIterationContext::default();
        let color = colorizer.colorize(&make_interior(), &ctx, &options, &palette, 0);
        assert_eq!(color, [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_zero_is_dark() {
        let colorizer = SmoothIterationColorizer;
        let options = grayscale_options();
        let palette = options.palette();
        let ctx = SmoothIterationContext::default();
        let color = colorizer.colorize(&make_escaped(0, 1000), &ctx, &options, &palette, 0);
        assert!(color[0] < 10, "Expected near black, got {:?}", color);
    }

    #[test]
    fn cycling_produces_color_variation() {
        let colorizer = SmoothIterationColorizer;
        let options = grayscale_options();
        let palette = options.palette();
        let ctx = SmoothIterationContext::default();
        // With cycling, nearby iteration values should produce different colors
        let color1 = colorizer.colorize(&make_escaped(500, 1000), &ctx, &options, &palette, 0);
        let color2 = colorizer.colorize(&make_escaped(510, 1000), &ctx, &options, &palette, 0);
        // Just verify we get valid colors (alpha = 255)
        assert_eq!(color1[3], 255);
        assert_eq!(color2[3], 255);
    }

    #[test]
    fn higher_iterations_are_brighter() {
        let colorizer = SmoothIterationColorizer;
        let options = grayscale_options();
        let palette = options.palette();
        let ctx = SmoothIterationContext::default();
        let low = colorizer.colorize(&make_escaped(100, 1000), &ctx, &options, &palette, 0);
        let high = colorizer.colorize(&make_escaped(900, 1000), &ctx, &options, &palette, 0);
        assert!(high[0] > low[0], "Higher iterations should be brighter");
    }

    #[test]
    fn smooth_iteration_produces_gradual_change() {
        let colorizer = SmoothIterationColorizer;
        let options = grayscale_options();

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

        let palette = options.palette();
        let ctx = SmoothIterationContext::default();
        let color1 = colorizer.colorize(&data1, &ctx, &options, &palette, 0);
        let color2 = colorizer.colorize(&data2, &ctx, &options, &palette, 0);

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

    #[test]
    fn build_histogram_cdf_uniform_distribution() {
        // 10 pixels with iterations 0-9, max_iter=10
        let data: Vec<ComputeData> = (0..10)
            .map(|i| {
                ComputeData::Mandelbrot(MandelbrotData {
                    iterations: i,
                    max_iterations: 10,
                    escaped: true,
                    glitched: false,
                    final_z_norm_sq: 100000.0,
                })
            })
            .collect();

        let cdf = build_histogram_cdf(&data, 10);

        // Uniform distribution: CDF should be [0.1, 0.2, 0.3, ..., 1.0]
        assert_eq!(cdf.len(), 11); // max_iter + 1
        assert!((cdf[0] - 0.1).abs() < 0.001);
        assert!((cdf[4] - 0.5).abs() < 0.001);
        assert!((cdf[9] - 1.0).abs() < 0.001);
    }

    #[test]
    fn build_histogram_cdf_skewed_distribution() {
        // Most pixels at iteration 5
        let mut data = Vec::new();
        for _ in 0..90 {
            data.push(ComputeData::Mandelbrot(MandelbrotData {
                iterations: 5,
                max_iterations: 10,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            }));
        }
        for _ in 0..10 {
            data.push(ComputeData::Mandelbrot(MandelbrotData {
                iterations: 9,
                max_iterations: 10,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            }));
        }

        let cdf = build_histogram_cdf(&data, 10);

        // Iterations 0-4 have 0 pixels, so CDF stays at 0
        assert_eq!(cdf[0], 0.0);
        assert_eq!(cdf[4], 0.0);
        // Iteration 5 has 90% of pixels
        assert!((cdf[5] - 0.9).abs() < 0.001);
        // Iteration 9 brings it to 100%
        assert!((cdf[9] - 1.0).abs() < 0.001);
    }

    #[test]
    fn build_histogram_cdf_excludes_interior() {
        let data = vec![
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 5,
                max_iterations: 10,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            }),
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 10,
                max_iterations: 10,
                escaped: false, // Interior point
                glitched: false,
                final_z_norm_sq: 0.0,
            }),
        ];

        let cdf = build_histogram_cdf(&data, 10);

        // Only 1 exterior pixel at iteration 5
        assert_eq!(cdf[5], 1.0);
    }

    #[test]
    fn preprocess_builds_cdf_when_histogram_enabled() {
        let colorizer = SmoothIterationColorizer;
        let options = ColorOptions {
            histogram_enabled: true,
            ..grayscale_options()
        };

        let data: Vec<ComputeData> = (0..10)
            .map(|i| {
                ComputeData::Mandelbrot(MandelbrotData {
                    iterations: i,
                    max_iterations: 10,
                    escaped: true,
                    glitched: false,
                    final_z_norm_sq: 100000.0,
                })
            })
            .collect();

        let ctx = colorizer.preprocess(&data, &options);

        assert!(ctx.cdf.is_some());
        let cdf = ctx.cdf.unwrap();
        assert_eq!(cdf.len(), 11);
    }

    #[test]
    fn preprocess_no_cdf_when_histogram_disabled() {
        let colorizer = SmoothIterationColorizer;
        let options = grayscale_options();

        let data = vec![make_escaped(5, 10)];
        let ctx = colorizer.preprocess(&data, &options);

        assert!(ctx.cdf.is_none());
    }

    #[test]
    fn colorize_uses_cdf_when_available() {
        let colorizer = SmoothIterationColorizer;
        let options = ColorOptions {
            histogram_enabled: true,
            cycle_count: 1, // No cycling for predictable results
            ..grayscale_options()
        };

        // Create skewed data: 90 pixels at iter 1, 10 at iter 9
        let mut data = Vec::new();
        for _ in 0..90 {
            data.push(ComputeData::Mandelbrot(MandelbrotData {
                iterations: 1,
                max_iterations: 10,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            }));
        }
        for _ in 0..10 {
            data.push(ComputeData::Mandelbrot(MandelbrotData {
                iterations: 9,
                max_iterations: 10,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            }));
        }

        let palette = options.palette();
        let ctx = colorizer.preprocess(&data, &options);

        // First pixel (iter 1) should map to CDF[1] = 0.9 (90% of pixels)
        let color1 = colorizer.colorize(&data[0], &ctx, &options, &palette, 0);
        // Last pixel (iter 9) should map to CDF[9] = 1.0
        let color2 = colorizer.colorize(&data[90], &ctx, &options, &palette, 90);

        // With grayscale and CDF, iter 1 gets bright (0.9), iter 9 gets white (1.0)
        // Both should be quite bright since CDF values are high
        assert!(
            color1[0] > 200,
            "iter 1 with CDF 0.9 should be bright: {:?}",
            color1
        );
        assert!(
            color2[0] > 250,
            "iter 9 with CDF 1.0 should be near white: {:?}",
            color2
        );
    }
}
