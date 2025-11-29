//! Smooth iteration colorizer using the formula μ = n + 1 - log₂(ln(|z|))
//! to eliminate banding in exterior regions.

use super::{Colorizer, Palette};
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Colorizer that uses smooth iteration count to eliminate banding.
/// Uses the formula μ = n + 1 - log₂(ln(|z|)) where |z| is computed from final_z_norm_sq.
#[derive(Clone, Debug, Default)]
pub struct SmoothIterationColorizer;

impl Colorizer for SmoothIterationColorizer {
    type Context = ();

    fn colorize(&self, data: &ComputeData, _context: &Self::Context, palette: &Palette) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => self.colorize_mandelbrot(m, palette),
            ComputeData::TestImage(_) => {
                // Test image uses its own colorizer
                [128, 128, 128, 255]
            }
        }
    }
}

impl SmoothIterationColorizer {
    fn colorize_mandelbrot(&self, data: &MandelbrotData, palette: &Palette) -> [u8; 4] {
        // Interior points are black
        if !data.escaped {
            return [0, 0, 0, 255];
        }

        // Avoid division by zero
        if data.max_iterations == 0 {
            return [0, 0, 0, 255];
        }

        // Smooth iteration count: μ = n + 1 - log₂(ln(|z|))
        // Since we have |z|²: ln(|z|) = ln(|z|²) / 2
        let smooth = if data.final_z_norm_sq > 1.0 {
            let z_norm_sq = data.final_z_norm_sq as f64;
            let log_z = z_norm_sq.ln() / 2.0; // ln(|z|)
            let nu = log_z.ln() / std::f64::consts::LN_2; // log₂(ln(|z|))
            data.iterations as f64 + 1.0 - nu
        } else {
            // Fallback for edge cases
            data.iterations as f64
        };

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
        let color = colorizer.colorize(&make_interior(), &(), &palette);
        assert_eq!(color, [0, 0, 0, 255]);
    }

    #[test]
    fn escaped_at_zero_is_dark() {
        let colorizer = SmoothIterationColorizer;
        let palette = Palette::grayscale();
        let color = colorizer.colorize(&make_escaped(0, 1000), &(), &palette);
        assert!(color[0] < 10, "Expected near black, got {:?}", color);
    }

    #[test]
    fn escaped_at_max_is_bright() {
        let colorizer = SmoothIterationColorizer;
        let palette = Palette::grayscale();
        let color = colorizer.colorize(&make_escaped(1000, 1000), &(), &palette);
        assert!(color[0] > 245, "Expected near white, got {:?}", color);
    }

    #[test]
    fn higher_iterations_are_brighter() {
        let colorizer = SmoothIterationColorizer;
        let palette = Palette::grayscale();
        let low = colorizer.colorize(&make_escaped(100, 1000), &(), &palette);
        let high = colorizer.colorize(&make_escaped(900, 1000), &(), &palette);
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

        let color1 = colorizer.colorize(&data1, &(), &palette);
        let color2 = colorizer.colorize(&data2, &(), &palette);

        // With smooth formula, larger |z|² means lower μ, so darker color
        assert!(
            color1[0] > color2[0],
            "Larger z_norm_sq should produce darker color: {:?} vs {:?}",
            color1,
            color2
        );
    }
}
