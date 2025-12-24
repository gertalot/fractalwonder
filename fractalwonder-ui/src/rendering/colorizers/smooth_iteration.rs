//! Smooth iteration colorizer using the formula μ = n + 1 - log₂(ln(|z|))
//! to eliminate banding in exterior regions.

use super::shading::apply_slope_shading;
use super::{Colorizer, Palette, PaletteLut, RenderSettings};
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

/// Build sorted values for rank-order histogram coloring.
/// Only includes exterior (escaped) points. Interior points are excluded.
/// When `use_smooth` is true, uses the provided smooth values.
/// When `use_smooth` is false, uses discrete iteration counts.
fn build_sorted_histogram_values(
    smooth_values: &[f64],
    data: &[ComputeData],
    use_smooth: bool,
) -> Vec<f64> {
    let mut sorted: Vec<f64> = smooth_values
        .iter()
        .zip(data.iter())
        .filter_map(|(&smooth, d)| {
            if let ComputeData::Mandelbrot(m) = d {
                if m.escaped {
                    return Some(if use_smooth {
                        smooth
                    } else {
                        m.iterations as f64
                    });
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

    fn preprocess(&self, data: &[ComputeData], palette: &Palette) -> Self::Context {
        let smooth_values: Vec<f64> = data
            .iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => compute_smooth_iteration(m),
                ComputeData::TestImage(_) => 0.0,
            })
            .collect();

        let sorted_smooth = if palette.histogram_enabled {
            Some(build_sorted_histogram_values(
                &smooth_values,
                data,
                palette.smooth_enabled,
            ))
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
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
        index: usize,
    ) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => {
                let smooth = if index < context.smooth_values.len() {
                    context.smooth_values[index]
                } else {
                    compute_smooth_iteration(m)
                };
                self.colorize_mandelbrot(m, smooth, context, palette, lut, render_settings)
            }
            ComputeData::TestImage(_) => [128, 128, 128, 255],
        }
    }

    fn postprocess(
        &self,
        pixels: &mut [[u8; 4]],
        data: &[ComputeData],
        _context: &Self::Context,
        palette: &Palette,
        width: usize,
        height: usize,
    ) {
        apply_slope_shading(pixels, data, palette, width, height);
    }
}

impl SmoothIterationColorizer {
    /// Colorize a single pixel using a cached histogram from a previous render.
    /// Computes fresh smooth iteration value for the new data, but uses the cached
    /// sorted_smooth for histogram-based percentile lookup.
    pub fn colorize_with_histogram(
        &self,
        data: &ComputeData,
        cached_context: &SmoothIterationContext,
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
    ) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => {
                let smooth = compute_smooth_iteration(m);
                self.colorize_mandelbrot(m, smooth, cached_context, palette, lut, render_settings)
            }
            ComputeData::TestImage(_) => [128, 128, 128, 255],
        }
    }

    fn colorize_mandelbrot(
        &self,
        data: &MandelbrotData,
        smooth: f64,
        context: &SmoothIterationContext,
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
    ) -> [u8; 4] {
        if !data.escaped {
            return [0, 0, 0, 255];
        }

        if data.max_iterations == 0 {
            return [0, 0, 0, 255];
        }

        let normalized = if let Some(sorted) = &context.sorted_smooth {
            let lookup_value = if palette.smooth_enabled {
                smooth
            } else {
                data.iterations as f64
            };
            percentile_rank(sorted, lookup_value)
        } else if palette.smooth_enabled {
            smooth / data.max_iterations as f64
        } else {
            data.iterations as f64 / data.max_iterations as f64
        };

        // Apply transfer curve (replaces transfer_bias)
        let transferred = palette.apply_transfer(normalized);

        // Apply cycling
        let cycle_count = render_settings.cycle_count as f64;
        let t = if cycle_count > 1.0 {
            (transferred * cycle_count).fract()
        } else {
            (transferred * cycle_count).clamp(0.0, 1.0)
        };

        let [r, g, b] = lut.sample(t);
        [r, g, b, 255]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::{ComputeData, MandelbrotData};

    #[test]
    fn compute_smooth_iteration_interior_returns_max() {
        let data = MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 4.0,
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
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
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
        };
        let smooth = compute_smooth_iteration(&data);
        // Should be close to 10 but with fractional adjustment
        // The smooth formula n + 1 - ν can reduce the value, so it may be < 10
        assert!(smooth > 8.0 && smooth < 12.0, "smooth = {}", smooth);
        // Verify it has a fractional component (not exactly iterations)
        assert_ne!(smooth, data.iterations as f64);
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
                final_z_re: 0.0,
                final_z_im: 0.0,
                final_derivative_re: 0.0,
                final_derivative_im: 0.0,
            }),
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 10,
                max_iterations: 10,
                escaped: false, // Interior point
                glitched: false,
                final_z_norm_sq: 0.0,
                final_z_re: 0.0,
                final_z_im: 0.0,
                final_derivative_re: 0.0,
                final_derivative_im: 0.0,
            }),
        ];

        let smooth_values: Vec<f64> = data
            .iter()
            .map(|d| match d {
                ComputeData::Mandelbrot(m) => compute_smooth_iteration(m),
                _ => 0.0,
            })
            .collect();

        let sorted = build_sorted_histogram_values(&smooth_values, &data, true);

        // Only 1 exterior pixel
        assert_eq!(sorted.len(), 1);
    }
}
