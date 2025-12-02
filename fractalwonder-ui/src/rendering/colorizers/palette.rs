//! Color palettes with OKLAB interpolation.
//!
//! Colors are pre-computed into a lookup table at construction time for fast sampling.

use super::color_space::{
    linear_rgb_to_oklab, linear_to_srgb, oklab_to_linear_rgb, oklch_to_srgb, srgb_to_linear,
};

/// Number of entries in the pre-computed lookup table.
const LUT_SIZE: usize = 4096;

/// A color palette that maps normalized values [0,1] to RGB colors.
/// OKLAB interpolation is pre-computed into a lookup table for fast sampling.
#[derive(Clone, Debug)]
pub struct Palette {
    /// Pre-computed lookup table with OKLAB-interpolated colors
    lut: Vec<[u8; 3]>,
}

impl Palette {
    /// Create a new palette from control points.
    /// Pre-computes a lookup table using OKLAB interpolation.
    pub fn new(colors: Vec<[u8; 3]>) -> Self {
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

        Self { lut }
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
        let t = t.clamp(0.0, 1.0);
        let index = ((t * (LUT_SIZE - 1) as f64) as usize).min(LUT_SIZE - 1);
        self.lut[index]
    }

    /// Black to white gradient.
    pub fn grayscale() -> Self {
        Self::new(vec![[0, 0, 0], [255, 255, 255]])
    }

    /// Classic Ultra Fractal palette: blue → white → orange → black.
    pub fn ultra_fractal() -> Self {
        Self::new(vec![
            [0, 7, 100],     // Deep blue
            [0, 2, 0],       // Near black
            [0, 7, 100],     // Deep blue
            [32, 107, 203],  // Blue
            [255, 170, 0],   // Orange
            [237, 255, 255], // White
        ])
    }

    /// Fire palette: black → red → orange → yellow → white.
    pub fn fire() -> Self {
        Self::new(vec![
            [0, 0, 0],       // Black
            [128, 0, 0],     // Dark red
            [255, 0, 0],     // Red
            [255, 128, 0],   // Orange
            [255, 255, 0],   // Yellow
            [255, 255, 255], // White
        ])
    }

    /// Ocean palette: deep blue → cyan → white.
    pub fn ocean() -> Self {
        Self::new(vec![
            [0, 0, 64],      // Deep blue
            [0, 64, 128],    // Blue
            [0, 128, 192],   // Cyan-blue
            [64, 192, 255],  // Cyan
            [255, 255, 255], // White
        ])
    }

    /// Electric palette: purple → blue → cyan → green → yellow.
    pub fn electric() -> Self {
        Self::new(vec![
            [32, 0, 64],   // Dark purple
            [64, 0, 128],  // Purple
            [0, 0, 255],   // Blue
            [0, 255, 255], // Cyan
            [0, 255, 0],   // Green
            [255, 255, 0], // Yellow
        ])
    }

    // ============================================================
    // Cyclical palettes - designed for seamless wrapping with cycling
    // ============================================================

    /// Create a hue-cycling palette using OKLCH color space.
    /// Generates `steps` colors around the hue wheel at constant lightness and chroma.
    /// Seamlessly cyclical - first and last colors are adjacent on the wheel.
    fn hue_cycle(lightness: f64, chroma: f64, steps: usize) -> Self {
        let colors: Vec<[u8; 3]> = (0..steps)
            .map(|i| {
                let hue = (i as f64 / steps as f64) * std::f64::consts::TAU;
                oklch_to_srgb(lightness, chroma, hue)
            })
            .collect();

        Self::new(colors)
    }

    /// Rainbow: Full spectrum hue cycle at vibrant saturation.
    pub fn rainbow() -> Self {
        Self::hue_cycle(0.75, 0.15, 64)
    }

    /// Neon: Bright, saturated hue cycle.
    /// High lightness and chroma for electric, glowing appearance.
    pub fn neon() -> Self {
        Self::hue_cycle(0.85, 0.18, 48)
    }

    /// Twilight: Warm to cool cycling palette.
    /// Orange → magenta → purple → blue → cyan → back to orange.
    /// Designed for seamless cycling.
    pub fn twilight() -> Self {
        Self::new(vec![
            [255, 100, 50],  // Warm orange
            [255, 50, 100],  // Coral
            [200, 50, 150],  // Magenta
            [150, 50, 200],  // Purple
            [80, 80, 220],   // Blue-violet
            [50, 150, 255],  // Sky blue
            [80, 200, 200],  // Cyan
            [150, 200, 150], // Soft green
            [200, 180, 100], // Gold
            [255, 100, 50],  // Back to warm orange (seamless)
        ])
    }

    /// Candy: Pastel cycling palette.
    /// Soft, desaturated colors that cycle smoothly.
    /// Great for high-iteration deep zooms.
    pub fn candy() -> Self {
        Self::new(vec![
            [255, 180, 200], // Pink
            [200, 180, 255], // Lavender
            [180, 220, 255], // Baby blue
            [180, 255, 220], // Mint
            [220, 255, 180], // Lime
            [255, 240, 180], // Cream
            [255, 200, 180], // Peach
            [255, 180, 200], // Back to pink (seamless)
        ])
    }

    /// Inferno: Dark cycling palette with hot accents.
    /// Black → deep red → orange → gold → black.
    /// Dramatic contrast.
    pub fn inferno() -> Self {
        Self::new(vec![
            [80, 50, 40],    // Brown
            [20, 10, 20],    // Dark purple
            [5, 0, 10],      // Back to near black (seamless)
            [5, 0, 10],      // Near black (purple tint)
            [40, 0, 20],     // Dark burgundy
            [100, 10, 10],   // Dark red
            [180, 40, 0],    // Red-orange
            [255, 100, 0],   // Orange
            [255, 180, 50],  // Gold
            [200, 150, 100], // Muted tan
        ])
    }

    /// Inferno: Dark cycling palette with hot accents.
    /// Black → deep red → orange → gold → black.
    /// Dramatic contrast.
    pub fn stripey_inferno() -> Self {
        Self::new(vec![
            [5, 0, 10],      // Near black (purple tint)
            [200, 150, 100], // Muted tan
            [5, 0, 10],      // Near black (purple tint)
            [200, 150, 100], // Muted tan
            [5, 0, 10],      // Near black (purple tint)
            [200, 150, 100], // Muted tan

            [80, 50, 40],    // Brown
            [20, 10, 20],    // Dark purple
            [5, 0, 10],      // Near black (purple tint)
            [40, 0, 20],     // Dark burgundy
            [100, 10, 10],   // Dark red
            [180, 40, 0],    // Red-orange
            [255, 100, 0],   // Orange
            [255, 180, 50],  // Gold
            [200, 150, 100], // Muted tan
        ])
    }

    /// Aurora: Northern lights inspired palette.
    /// Green → cyan → blue → purple → green.
    /// Ethereal, glowing appearance.
    pub fn aurora() -> Self {
        Self::new(vec![
            [50, 255, 100],  // Bright green
            [50, 255, 180],  // Cyan-green
            [50, 200, 255],  // Cyan
            [80, 120, 255],  // Blue
            [150, 80, 255],  // Purple
            [200, 100, 200], // Magenta
            [150, 150, 150], // Gray (dimmer band)
            [100, 200, 100], // Soft green
            [50, 255, 100],  // Back to bright green (seamless)
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_two_color_palette_at_zero() {
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255]]);
        assert_eq!(palette.sample(0.0), [0, 0, 0]);
    }

    #[test]
    fn sample_two_color_palette_at_one() {
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255]]);
        assert_eq!(palette.sample(1.0), [255, 255, 255]);
    }

    #[test]
    fn sample_two_color_palette_at_midpoint() {
        let palette = Palette::new(vec![[0, 0, 0], [255, 255, 255]]);
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
        let palette = Palette::new(vec![[100, 100, 100], [200, 200, 200]]);
        assert_eq!(palette.sample(-0.5), [100, 100, 100]);
    }

    #[test]
    fn sample_clamps_above_one() {
        let palette = Palette::new(vec![[100, 100, 100], [200, 200, 200]]);
        assert_eq!(palette.sample(1.5), [200, 200, 200]);
    }

    #[test]
    fn grayscale_preset_exists() {
        let palette = Palette::grayscale();
        assert_eq!(palette.sample(0.0), [0, 0, 0]);
        assert_eq!(palette.sample(1.0), [255, 255, 255]);
    }

    #[test]
    fn cyclical_palettes_have_distinct_colors() {
        // Verify cycling palettes produce varied colors
        for palette in [
            Palette::rainbow(),
            Palette::neon(),
            Palette::twilight(),
            Palette::candy(),
            Palette::inferno(),
            Palette::aurora(),
        ] {
            let samples: Vec<_> = (0..10).map(|i| palette.sample(i as f64 / 10.0)).collect();
            // At least some colors should be different
            let all_same = samples.windows(2).all(|w| w[0] == w[1]);
            assert!(!all_same, "Palette should have color variation");
        }
    }
}
