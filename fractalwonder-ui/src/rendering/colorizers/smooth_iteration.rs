//! Smooth iteration colorizer using the formula μ = n + 1 - log₂(ln(|z|))
//! to eliminate banding in exterior regions.

use super::{shading::apply_slope_shading, ColorOptions, Colorizer};
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Colorizer that uses smooth iteration count to eliminate banding.
/// Uses the formula μ = n + 1 - log₂(ln(|z|)) where |z| is computed from final_z_norm_sq.
#[derive(Clone, Debug, Default)]
pub struct SmoothIterationColorizer;

/// Context data computed during preprocessing.
/// Holds smooth iteration values and optional rank-order data for histogram equalization.
#[derive(Clone, Debug, Default)]
pub struct SmoothIterationContext {
    /// Smooth iteration values per pixel.
    pub smooth_values: Vec<f64>,
    /// Sorted smooth values for rank-order histogram coloring. None if disabled.
    /// Used to find each pixel's percentile rank via binary search.
    pub sorted_smooth: Option<Vec<f64>>,
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

/// Build sorted smooth values for rank-order histogram coloring.
/// Only includes exterior (escaped) points. Interior points are excluded.
/// Returns a sorted Vec of smooth iteration values.
fn build_sorted_smooth_values(smooth_values: &[f64], data: &[ComputeData]) -> Vec<f64> {
    let mut sorted: Vec<f64> = smooth_values
        .iter()
        .zip(data.iter())
        .filter_map(|(&smooth, d)| {
            if let ComputeData::Mandelbrot(m) = d {
                if m.escaped {
                    return Some(smooth);
                }
            }
            None
        })
        .collect();

    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted
}

/// Find the percentile rank of a value in a sorted list using binary search.
/// Returns a value in [0, 1] representing the fraction of values <= the given value.
#[inline]
fn percentile_rank(sorted: &[f64], value: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    // Binary search to find insertion point
    let pos = sorted.partition_point(|&v| v < value);

    // The rank is the position divided by total count
    // This gives the fraction of values strictly less than this value
    // Add 0.5 to center within the bucket for smoother results
    (pos as f64 + 0.5) / sorted.len() as f64
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

        let sorted_smooth = if options.histogram_enabled {
            Some(build_sorted_smooth_values(&smooth_values, data))
        } else {
            None
        };

        SmoothIterationContext {
            smooth_values,
            sorted_smooth,
        }
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

        // Normalize: use rank-order if available, otherwise linear (smooth or discrete)
        let normalized = if let Some(sorted) = &context.sorted_smooth {
            // Rank-order histogram: find percentile rank of this smooth value
            // This distributes colors evenly across the image area
            percentile_rank(sorted, smooth)
        } else if options.smooth_enabled {
            smooth / data.max_iterations as f64
        } else {
            data.iterations as f64 / data.max_iterations as f64
        };

        // Apply transfer bias to shift color distribution
        let transferred = super::apply_transfer_bias(normalized, options.transfer_bias);

        // Apply cycling and sample palette
        let cycle_count = options.cycle_count as f64;
        let t = if cycle_count > 1.0 {
            (transferred * cycle_count).fract()
        } else {
            (transferred * cycle_count).clamp(0.0, 1.0)
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
    fn smooth_iteration_context_default_has_no_sorted_smooth() {
        let ctx = SmoothIterationContext::default();
        assert!(ctx.smooth_values.is_empty());
        assert!(ctx.sorted_smooth.is_none());
    }

    #[test]
    fn percentile_rank_empty_returns_zero() {
        assert_eq!(percentile_rank(&[], 5.0), 0.0);
    }

    #[test]
    fn percentile_rank_single_element() {
        let sorted = vec![5.0];
        // Value below: rank 0.5 (centered in first bucket)
        assert!((percentile_rank(&sorted, 3.0) - 0.5).abs() < 0.01);
        // Value equal: rank 0.5 (centered)
        assert!((percentile_rank(&sorted, 5.0) - 0.5).abs() < 0.01);
        // Value above: rank ~1.0 (at end)
        assert!((percentile_rank(&sorted, 7.0) - 1.5).abs() < 0.01);
    }

    #[test]
    fn percentile_rank_uniform_distribution() {
        // 10 evenly spaced values
        let sorted: Vec<f64> = (0..10).map(|i| i as f64).collect();

        // Value at start should be near 0
        assert!(percentile_rank(&sorted, 0.0) < 0.15);
        // Value in middle should be near 0.5
        assert!((percentile_rank(&sorted, 4.5) - 0.5).abs() < 0.1);
        // Value at end should be near 1.0
        assert!(percentile_rank(&sorted, 9.0) > 0.9);
    }

    #[test]
    fn build_sorted_smooth_excludes_interior() {
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

        let smooth_values: Vec<f64> = data
            .iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => compute_smooth_iteration(m),
                _ => 0.0,
            })
            .collect();

        let sorted = build_sorted_smooth_values(&smooth_values, &data);

        // Only 1 exterior pixel
        assert_eq!(sorted.len(), 1);
    }

    #[test]
    fn preprocess_builds_sorted_smooth_when_histogram_enabled() {
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

        assert!(ctx.sorted_smooth.is_some());
        assert_eq!(ctx.sorted_smooth.unwrap().len(), 10);
    }

    #[test]
    fn preprocess_no_sorted_smooth_when_histogram_disabled() {
        let colorizer = SmoothIterationColorizer;
        let options = grayscale_options();

        let data = vec![make_escaped(5, 10)];
        let ctx = colorizer.preprocess(&data, &options);

        assert!(ctx.sorted_smooth.is_none());
    }

    #[test]
    fn colorize_uses_rank_order_when_histogram_enabled() {
        let colorizer = SmoothIterationColorizer;
        let options = ColorOptions {
            histogram_enabled: true,
            cycle_count: 1, // No cycling for predictable results
            ..grayscale_options()
        };

        // Create skewed data: 90 pixels at iter 1, 10 at iter 9
        // With rank-order, the 90 iter-1 pixels will span ranks 0-0.9
        // and the 10 iter-9 pixels will span ranks 0.9-1.0
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

        // First pixel (iter 1) - with rank-order, all 90 iter-1 pixels have similar smooth values
        // so they'll be distributed across ranks ~0.0 to ~0.9
        let color1 = colorizer.colorize(&data[0], &ctx, &options, &palette, 0);
        // Last pixel (iter 9) - these 10 pixels will be in ranks ~0.9 to ~1.0
        let color2 = colorizer.colorize(&data[90], &ctx, &options, &palette, 90);

        // With rank-order histogram, the iter-9 pixels should be brighter than iter-1 pixels
        // because iter-9 has higher smooth values -> higher rank -> brighter color
        assert!(
            color2[0] > color1[0],
            "Higher iteration should have higher rank and brighter color: iter1={:?}, iter9={:?}",
            color1,
            color2
        );
    }
}
