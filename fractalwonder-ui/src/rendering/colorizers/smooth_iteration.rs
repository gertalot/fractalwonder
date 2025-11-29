//! Smooth iteration colorizer - basic palette mapping without smoothing.
//! (True smooth iteration requires `final_z_norm_sq` in MandelbrotData - Increment 2)

use super::{Colorizer, Palette};
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Basic colorizer that maps iteration count to palette position.
/// Currently uses linear mapping; will use smooth iteration formula
/// once `final_z_norm_sq` is available in MandelbrotData.
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

        // Linear normalization for now (smooth iteration in Increment 2)
        let t = data.iterations as f64 / data.max_iterations as f64;
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
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations,
            escaped: true,
            glitched: false,
        })
    }

    fn make_interior() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
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
}
