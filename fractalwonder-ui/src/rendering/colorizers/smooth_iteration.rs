//! Smooth iteration colorizer using the formula μ = n + 1 - log₂(ln(|z|))
//! to eliminate banding in exterior regions.

use super::{Colorizer, Palette};
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Colorizer that uses smooth iteration count to eliminate banding.
/// Uses the formula μ = n + 1 - log₂(ln(|z|)) where |z| is computed from final_z_norm_sq.
#[derive(Clone, Debug, Default)]
pub struct SmoothIterationColorizer;

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
        palette: &Palette,
        index: usize,
    ) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => {
                let smooth = if index < context.len() {
                    context[index]
                } else {
                    compute_smooth_iteration(m)
                };
                self.colorize_mandelbrot_smooth(m, smooth, palette)
            }
            ComputeData::TestImage(_) => {
                // Test image uses its own colorizer
                [128, 128, 128, 255]
            }
        }
    }
}

impl SmoothIterationColorizer {
    fn colorize_mandelbrot_smooth(
        &self,
        data: &MandelbrotData,
        smooth: f64,
        palette: &Palette,
    ) -> [u8; 4] {
        // Interior points are black
        if !data.escaped {
            return [0, 0, 0, 255];
        }

        // Avoid division by zero
        if data.max_iterations == 0 {
            return [0, 0, 0, 255];
        }

        let t = (smooth / data.max_iterations as f64).clamp(0.0, 1.0);
        let [r, g, b] = palette.sample(t);
        [r, g, b, 255]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::Palette;
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
        let palette = Palette::grayscale();
        let color = colorizer.colorize(&make_interior(), &vec![], &palette, 0);
        assert_eq!(color, [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_zero_is_dark() {
        let colorizer = SmoothIterationColorizer;
        let palette = Palette::grayscale();
        let color = colorizer.colorize(&make_escaped(0, 1000), &vec![], &palette, 0);
        assert!(color[0] < 10, "Expected near black, got {:?}", color);
    }

    #[test]
    fn escaped_at_max_is_bright() {
        let colorizer = SmoothIterationColorizer;
        let palette = Palette::grayscale();
        let color = colorizer.colorize(&make_escaped(1000, 1000), &vec![], &palette, 0);
        assert!(color[0] > 245, "Expected near white, got {:?}", color);
    }

    #[test]
    fn higher_iterations_are_brighter() {
        let colorizer = SmoothIterationColorizer;
        let palette = Palette::grayscale();
        let low = colorizer.colorize(&make_escaped(100, 1000), &vec![], &palette, 0);
        let high = colorizer.colorize(&make_escaped(900, 1000), &vec![], &palette, 0);
        assert!(high[0] > low[0], "Higher iterations should be brighter");
    }

    #[test]
    fn smooth_iteration_produces_gradual_change() {
        let colorizer = SmoothIterationColorizer;
        let palette = Palette::grayscale();

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

        let color1 = colorizer.colorize(&data1, &vec![], &palette, 0);
        let color2 = colorizer.colorize(&data2, &vec![], &palette, 0);

        // With smooth formula, larger |z|² means lower μ, so darker color
        assert!(
            color1[0] > color2[0],
            "Larger z_norm_sq should produce darker color: {:?} vs {:?}",
            color1,
            color2
        );
    }
}
