//! Color palettes with OKLAB interpolation.
//!
//! Colors are pre-computed into a lookup table at construction time for fast sampling.

use super::color_space::{
    linear_rgb_to_oklab, linear_to_srgb, oklab_to_linear_rgb, srgb_to_linear,
};

/// Number of entries in the pre-computed lookup table.
const LUT_SIZE: usize = 4096;

/// A color palette that maps normalized values [0,1] to RGB colors.
/// OKLAB interpolation is pre-computed into a lookup table for fast sampling.
#[derive(Clone, Debug)]
pub struct Palette {
    /// Pre-computed lookup table with OKLAB-interpolated colors
    lut: Vec<[u8; 3]>,
    /// How many times to cycle through the palette (1.0 = no cycling)
    cycle_count: f64,
}

impl Palette {
    /// Create a new palette from control points.
    /// Pre-computes a lookup table using OKLAB interpolation.
    pub fn new(colors: Vec<[u8; 3]>, cycle_count: f64) -> Self {
        assert!(!colors.is_empty(), "Palette must have at least one color");

        // Convert control points to OKLAB
        let oklab_colors: Vec<(f64, f64, f64)> = colors
            .iter()
            .map(|&rgb| {
                let r = srgb_to_linear(rgb[0] as f64 / 255.0);
                let g = srgb_to_linear(rgb[1] as f64 / 255.0);
                let b = srgb_to_linear(rgb[2] as f64 / 255.0);
                linear_rgb_to_oklab(r, g, b)
            })
            .collect();

        // Build the lookup table
        let lut = (0..LUT_SIZE)
            .map(|i| {
                let t = i as f64 / (LUT_SIZE - 1) as f64;
                Self::interpolate_oklab(&oklab_colors, t)
            })
            .collect();

        Self { lut, cycle_count }
    }

    /// Interpolate in OKLAB space and convert back to sRGB.
    fn interpolate_oklab(oklab_colors: &[(f64, f64, f64)], t: f64) -> [u8; 3] {
        if oklab_colors.len() == 1 {
            let (l, a, b) = oklab_colors[0];
            return Self::oklab_to_srgb(l, a, b);
        }

        let t = t.clamp(0.0, 1.0);
        let scaled = t * (oklab_colors.len() - 1) as f64;
        let i = scaled.floor() as usize;
        let frac = scaled.fract();

        // Handle edge case at t=1.0
        if i >= oklab_colors.len() - 1 {
            let (l, a, b) = oklab_colors[oklab_colors.len() - 1];
            return Self::oklab_to_srgb(l, a, b);
        }

        let (l1, a1, b1) = oklab_colors[i];
        let (l2, a2, b2) = oklab_colors[i + 1];

        // Linear interpolation in OKLAB space
        let l = l1 + frac * (l2 - l1);
        let a = a1 + frac * (a2 - a1);
        let b = b1 + frac * (b2 - b1);

        Self::oklab_to_srgb(l, a, b)
    }

    fn oklab_to_srgb(l: f64, a: f64, b: f64) -> [u8; 3] {
        let (r, g, b) = oklab_to_linear_rgb(l, a, b);
        [
            (linear_to_srgb(r) * 255.0).round() as u8,
            (linear_to_srgb(g) * 255.0).round() as u8,
            (linear_to_srgb(b) * 255.0).round() as u8,
        ]
    }

    /// Sample the palette at position t ∈ [0,1].
    /// Fast lookup into pre-computed table.
    #[inline]
    pub fn sample(&self, t: f64) -> [u8; 3] {
        // Apply cycling and clamp
        let t = if self.cycle_count > 1.0 {
            (t * self.cycle_count).fract()
        } else {
            t.clamp(0.0, 1.0)
        };

        // Direct lookup into pre-computed LUT
        let index = ((t * (LUT_SIZE - 1) as f64) as usize).min(LUT_SIZE - 1);
        self.lut[index]
    }

    /// Black to white gradient.
    pub fn grayscale() -> Self {
        Self::new(vec![[0, 0, 0], [255, 255, 255]], 1.0)
    }

    /// Classic Ultra Fractal palette: blue → white → orange → black.
    pub fn ultra_fractal() -> Self {
        Self::new(
            vec![
                [0, 7, 100],     // Deep blue
                [32, 107, 203],  // Blue
                [237, 255, 255], // White
                [255, 170, 0],   // Orange
                [0, 2, 0],       // Near black
            ],
            1.0,
        )
    }

    /// Fire palette: black → red → orange → yellow → white.
    pub fn fire() -> Self {
        Self::new(
            vec![
                [0, 0, 0],       // Black
                [128, 0, 0],     // Dark red
                [255, 0, 0],     // Red
                [255, 128, 0],   // Orange
                [255, 255, 0],   // Yellow
                [255, 255, 255], // White
            ],
            1.0,
        )
    }

    /// Ocean palette: deep blue → cyan → white.
    pub fn ocean() -> Self {
        Self::new(
            vec![
                [0, 0, 64],      // Deep blue
                [0, 64, 128],    // Blue
                [0, 128, 192],   // Cyan-blue
                [64, 192, 255],  // Cyan
                [255, 255, 255], // White
            ],
            1.0,
        )
    }

    /// Electric palette: purple → blue → cyan → green → yellow.
    pub fn electric() -> Self {
        Self::new(
            vec![
                [32, 0, 64],   // Dark purple
                [64, 0, 128],  // Purple
                [0, 0, 255],   // Blue
                [0, 255, 255], // Cyan
                [0, 255, 0],   // Green
                [255, 255, 0], // Yellow
            ],
            1.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_two_color_palette_at_zero() {
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255]], 1.0);
        assert_eq!(palette.sample(0.0), [0, 0, 0]);
    }

    #[test]
    fn sample_two_color_palette_at_one() {
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255]], 1.0);
        assert_eq!(palette.sample(1.0), [255, 255, 255]);
    }

    #[test]
    fn sample_two_color_palette_at_midpoint() {
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255]], 1.0);
        let mid = palette.sample(0.5);
        // OKLAB interpolation of black-white at 0.5 should be ~middle gray
        // Not exactly 127 due to perceptual uniformity
        assert!(
            mid[0] > 90 && mid[0] < 160,
            "Expected mid gray, got {:?}",
            mid
        );
        assert_eq!(mid[0], mid[1]);
        assert_eq!(mid[1], mid[2]);
    }

    #[test]
    fn sample_clamps_below_zero() {
        let palette = Palette::new(vec![[100, 100, 100], [200, 200, 200]], 1.0);
        assert_eq!(palette.sample(-0.5), [100, 100, 100]);
    }

    #[test]
    fn sample_clamps_above_one() {
        let palette = Palette::new(vec![[100, 100, 100], [200, 200, 200]], 1.0);
        assert_eq!(palette.sample(1.5), [200, 200, 200]);
    }

    #[test]
    fn cycling_wraps_around() {
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255]], 2.0);
        // At t=0.25 with 2 cycles: 0.25 * 2 = 0.5, which is halfway through first cycle (mid-gray)
        let at_quarter = palette.sample(0.25);
        assert!(
            at_quarter[0] > 90 && at_quarter[0] < 110,
            "Expected mid gray at t=0.25 with 2 cycles, got {:?}",
            at_quarter
        );

        // At t=0.5 with 2 cycles: 0.5 * 2 = 1.0, which wraps back to 0.0 (black)
        let at_half = palette.sample(0.5);
        assert_eq!(at_half, [0, 0, 0]);
    }

    #[test]
    fn grayscale_preset_exists() {
        let palette = Palette::grayscale();
        assert_eq!(palette.sample(0.0), [0, 0, 0]);
        assert_eq!(palette.sample(1.0), [255, 255, 255]);
    }
}
