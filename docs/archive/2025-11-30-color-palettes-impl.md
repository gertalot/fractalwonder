# Color Palettes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace grayscale coloring with rich palettes using OKLAB interpolation and a composable colorizer architecture.

**Architecture:** Colorizers are algorithms with optional pre/post-process stages. Palettes are color mappings with OKLAB interpolation. Color scheme presets bundle a palette and colorizer together for UI selection.

**Tech Stack:** Rust, Leptos, WebAssembly

---

## Task 1: OKLAB Color Space Module

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/color_space.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write failing tests for sRGB/linear conversion**

Create the test file with tests for gamma conversion:

```rust
// In color_space.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_to_linear_black() {
        assert!((srgb_to_linear(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn srgb_to_linear_white() {
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn srgb_to_linear_mid_gray() {
        // sRGB 0.5 ≈ linear 0.214
        let result = srgb_to_linear(0.5);
        assert!((result - 0.214).abs() < 0.01);
    }

    #[test]
    fn linear_to_srgb_roundtrip() {
        for i in 0..=10 {
            let original = i as f64 / 10.0;
            let roundtrip = linear_to_srgb(srgb_to_linear(original));
            assert!((original - roundtrip).abs() < 1e-6, "Failed at {original}");
        }
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-ui color_space --no-default-features`

Expected: FAIL with "cannot find function `srgb_to_linear`"

**Step 3: Implement sRGB/linear conversions**

```rust
// fractalwonder-ui/src/rendering/colorizers/color_space.rs

//! OKLAB color space conversions for perceptually uniform palette interpolation.

/// Convert sRGB component [0,1] to linear RGB (remove gamma).
pub fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear RGB component to sRGB [0,1] (apply gamma).
pub fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui color_space --no-default-features`

Expected: PASS

**Step 5: Add OKLAB conversion tests**

```rust
    #[test]
    fn oklab_white() {
        let (l, a, b) = linear_rgb_to_oklab(1.0, 1.0, 1.0);
        assert!((l - 1.0).abs() < 0.01, "L should be ~1.0, got {l}");
        assert!(a.abs() < 0.01, "a should be ~0, got {a}");
        assert!(b.abs() < 0.01, "b should be ~0, got {b}");
    }

    #[test]
    fn oklab_black() {
        let (l, a, b) = linear_rgb_to_oklab(0.0, 0.0, 0.0);
        assert!(l.abs() < 0.01, "L should be ~0, got {l}");
        assert!(a.abs() < 0.01, "a should be ~0, got {a}");
        assert!(b.abs() < 0.01, "b should be ~0, got {b}");
    }

    #[test]
    fn oklab_roundtrip() {
        let test_colors = [
            (1.0, 0.0, 0.0), // Red
            (0.0, 1.0, 0.0), // Green
            (0.0, 0.0, 1.0), // Blue
            (0.5, 0.5, 0.5), // Gray
        ];
        for (r, g, b) in test_colors {
            let (l, a, ob) = linear_rgb_to_oklab(r, g, b);
            let (r2, g2, b2) = oklab_to_linear_rgb(l, a, ob);
            assert!((r - r2).abs() < 1e-4, "R mismatch for ({r},{g},{b})");
            assert!((g - g2).abs() < 1e-4, "G mismatch for ({r},{g},{b})");
            assert!((b - b2).abs() < 1e-4, "B mismatch for ({r},{g},{b})");
        }
    }
```

**Step 6: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-ui color_space --no-default-features`

Expected: FAIL with "cannot find function `linear_rgb_to_oklab`"

**Step 7: Implement OKLAB conversions**

```rust
/// Convert linear RGB to OKLAB (L, a, b).
/// L is perceptual lightness [0,1], a is green-red, b is blue-yellow.
pub fn linear_rgb_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    // RGB to LMS cone responses
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    // Cube root (perceptual non-linearity)
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    // LMS to OKLAB
    let lab_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let lab_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let lab_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    (lab_l, lab_a, lab_b)
}

/// Convert OKLAB to linear RGB.
pub fn oklab_to_linear_rgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    // OKLAB to LMS (cube-root space)
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    // Cube to undo perceptual non-linearity
    let lms_l = l_ * l_ * l_;
    let lms_m = m_ * m_ * m_;
    let lms_s = s_ * s_ * s_;

    // LMS to linear RGB
    let r = 4.0767416621 * lms_l - 3.3077115913 * lms_m + 0.2309699292 * lms_s;
    let g = -1.2684380046 * lms_l + 2.6097574011 * lms_m - 0.3413193965 * lms_s;
    let b = -0.0041960863 * lms_l - 0.7034186147 * lms_m + 1.7076147010 * lms_s;

    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}
```

**Step 8: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui color_space --no-default-features`

Expected: PASS

**Step 9: Add module to colorizers/mod.rs**

Add to `fractalwonder-ui/src/rendering/colorizers/mod.rs`:

```rust
pub mod color_space;
```

**Step 10: Run full test suite**

Run: `cargo test -p fractalwonder-ui --no-default-features`

Expected: PASS

**Step 11: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/color_space.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add OKLAB color space conversions"
```

---

## Task 2: Palette Struct

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/palette.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write failing tests for Palette**

```rust
// fractalwonder-ui/src/rendering/colorizers/palette.rs

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
        assert!(mid[0] > 100 && mid[0] < 160, "Expected mid gray, got {:?}", mid);
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
        // At t=0.5 with 2 cycles, we should be at the end of first cycle (white)
        let at_half = palette.sample(0.5);
        assert!(at_half[0] > 200, "Expected near white at t=0.5 with 2 cycles");
    }

    #[test]
    fn grayscale_preset_exists() {
        let palette = Palette::grayscale();
        assert_eq!(palette.sample(0.0), [0, 0, 0]);
        assert_eq!(palette.sample(1.0), [255, 255, 255]);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-ui palette --no-default-features`

Expected: FAIL with "cannot find struct `Palette`"

**Step 3: Implement Palette struct**

```rust
// fractalwonder-ui/src/rendering/colorizers/palette.rs

//! Color palettes with OKLAB interpolation.

use super::color_space::{linear_rgb_to_oklab, linear_to_srgb, oklab_to_linear_rgb, srgb_to_linear};

/// A color palette that maps normalized values [0,1] to RGB colors.
/// Interpolation happens in OKLAB space for perceptually uniform gradients.
#[derive(Clone, Debug)]
pub struct Palette {
    /// Control points in sRGB [0-255]
    colors: Vec<[u8; 3]>,
    /// How many times to cycle through the palette (1.0 = no cycling)
    cycle_count: f64,
}

impl Palette {
    /// Create a new palette from control points.
    pub fn new(colors: Vec<[u8; 3]>, cycle_count: f64) -> Self {
        assert!(!colors.is_empty(), "Palette must have at least one color");
        Self { colors, cycle_count }
    }

    /// Sample the palette at position t ∈ [0,1].
    /// Interpolates between control points in OKLAB space.
    pub fn sample(&self, t: f64) -> [u8; 3] {
        if self.colors.len() == 1 {
            return self.colors[0];
        }

        // Apply cycling and clamp
        let t = if self.cycle_count > 1.0 {
            (t * self.cycle_count).fract()
        } else {
            t.clamp(0.0, 1.0)
        };

        // Scale to color index
        let scaled = t * (self.colors.len() - 1) as f64;
        let i = scaled.floor() as usize;
        let frac = scaled.fract();

        // Handle edge case at t=1.0
        if i >= self.colors.len() - 1 {
            return self.colors[self.colors.len() - 1];
        }

        // Convert both colors to OKLAB
        let c1 = self.colors[i];
        let c2 = self.colors[i + 1];

        let (l1, a1, b1) = self.to_oklab(c1);
        let (l2, a2, b2) = self.to_oklab(c2);

        // Linear interpolation in OKLAB space
        let l = l1 + frac * (l2 - l1);
        let a = a1 + frac * (a2 - a1);
        let b = b1 + frac * (b2 - b1);

        // Convert back to sRGB
        self.from_oklab(l, a, b)
    }

    fn to_oklab(&self, rgb: [u8; 3]) -> (f64, f64, f64) {
        let r = srgb_to_linear(rgb[0] as f64 / 255.0);
        let g = srgb_to_linear(rgb[1] as f64 / 255.0);
        let b = srgb_to_linear(rgb[2] as f64 / 255.0);
        linear_rgb_to_oklab(r, g, b)
    }

    fn from_oklab(&self, l: f64, a: f64, b: f64) -> [u8; 3] {
        let (r, g, b) = oklab_to_linear_rgb(l, a, b);
        [
            (linear_to_srgb(r) * 255.0).round() as u8,
            (linear_to_srgb(g) * 255.0).round() as u8,
            (linear_to_srgb(b) * 255.0).round() as u8,
        ]
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui palette --no-default-features`

Expected: FAIL (grayscale_preset_exists still fails)

**Step 5: Add predefined palettes**

Add to `palette.rs`:

```rust
impl Palette {
    // ... existing methods ...

    /// Black to white gradient.
    pub fn grayscale() -> Self {
        Self::new(vec![[0, 0, 0], [255, 255, 255]], 1.0)
    }

    /// Classic Ultra Fractal palette: blue → white → orange → black.
    pub fn ultra_fractal() -> Self {
        Self::new(
            vec![
                [0, 7, 100],      // Deep blue
                [32, 107, 203],   // Blue
                [237, 255, 255],  // White
                [255, 170, 0],    // Orange
                [0, 2, 0],        // Near black
            ],
            1.0,
        )
    }

    /// Fire palette: black → red → orange → yellow → white.
    pub fn fire() -> Self {
        Self::new(
            vec![
                [0, 0, 0],        // Black
                [128, 0, 0],      // Dark red
                [255, 0, 0],      // Red
                [255, 128, 0],    // Orange
                [255, 255, 0],    // Yellow
                [255, 255, 255],  // White
            ],
            1.0,
        )
    }

    /// Ocean palette: deep blue → cyan → white.
    pub fn ocean() -> Self {
        Self::new(
            vec![
                [0, 0, 64],       // Deep blue
                [0, 64, 128],     // Blue
                [0, 128, 192],    // Cyan-blue
                [64, 192, 255],   // Cyan
                [255, 255, 255],  // White
            ],
            1.0,
        )
    }

    /// Electric palette: purple → blue → cyan → green → yellow.
    pub fn electric() -> Self {
        Self::new(
            vec![
                [32, 0, 64],      // Dark purple
                [64, 0, 128],     // Purple
                [0, 0, 255],      // Blue
                [0, 255, 255],    // Cyan
                [0, 255, 0],      // Green
                [255, 255, 0],    // Yellow
            ],
            1.0,
        )
    }
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui palette --no-default-features`

Expected: PASS

**Step 7: Add module to colorizers/mod.rs**

Add to `fractalwonder-ui/src/rendering/colorizers/mod.rs`:

```rust
pub mod palette;

pub use palette::Palette;
```

**Step 8: Run full test suite**

Run: `cargo test -p fractalwonder-ui --no-default-features`

Expected: PASS

**Step 9: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/palette.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add Palette with OKLAB interpolation and presets"
```

---

## Task 3: Colorizer Trait

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/colorizer.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Create colorizer trait definition**

```rust
// fractalwonder-ui/src/rendering/colorizers/colorizer.rs

//! Colorizer trait for mapping compute data to colors.

use super::Palette;
use fractalwonder_core::ComputeData;

/// A colorizer algorithm with optional pre/post-processing stages.
///
/// # Pipeline Flow
/// 1. `preprocess` - analyze all pixels, build context (e.g., histogram CDF)
/// 2. `colorize` - map each pixel to a color using context and palette
/// 3. `postprocess` - modify pixel buffer in place (e.g., slope shading)
pub trait Colorizer {
    /// Data passed from preprocess to colorize/postprocess.
    type Context: Default;

    /// Analyze all pixels, build context.
    /// Default: no-op, returns `Default::default()`.
    fn preprocess(&self, _data: &[ComputeData]) -> Self::Context {
        Self::Context::default()
    }

    /// Map a single pixel to a color.
    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        palette: &Palette,
    ) -> [u8; 4];

    /// Modify pixel buffer in place.
    /// Default: no-op.
    fn postprocess(
        &self,
        _pixels: &mut [[u8; 4]],
        _data: &[ComputeData],
        _context: &Self::Context,
        _palette: &Palette,
        _width: usize,
        _height: usize,
    ) {
    }
}
```

**Step 2: Add module to colorizers/mod.rs**

```rust
pub mod colorizer;

pub use colorizer::Colorizer;
```

**Step 3: Run clippy to verify no errors**

Run: `cargo clippy -p fractalwonder-ui --no-default-features -- -D warnings`

Expected: PASS (or existing warnings unrelated to new code)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/colorizer.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add Colorizer trait with pre/post-process stages"
```

---

## Task 4: SmoothIterationColorizer

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write failing tests**

```rust
// fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::MandelbrotData;

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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-ui smooth_iteration --no-default-features`

Expected: FAIL with "cannot find struct `SmoothIterationColorizer`"

**Step 3: Implement SmoothIterationColorizer**

```rust
// fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs

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

    fn colorize(
        &self,
        data: &ComputeData,
        _context: &Self::Context,
        palette: &Palette,
    ) -> [u8; 4] {
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui smooth_iteration --no-default-features`

Expected: PASS

**Step 5: Add module to colorizers/mod.rs**

```rust
pub mod smooth_iteration;

pub use smooth_iteration::SmoothIterationColorizer;
```

**Step 6: Run full test suite**

Run: `cargo test -p fractalwonder-ui --no-default-features`

Expected: PASS

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add SmoothIterationColorizer"
```

---

## Task 5: ColorizerKind Enum and Pipeline

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/colorizer.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write failing tests for ColorizerKind**

Add to `colorizer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::{Palette, SmoothIterationColorizer};
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn colorizer_kind_runs_pipeline() {
        let colorizer = ColorizerKind::SmoothIteration(SmoothIterationColorizer);
        let palette = Palette::grayscale();

        let data = vec![
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 500,
                max_iterations: 1000,
                escaped: true,
                glitched: false,
            }),
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 0,
                max_iterations: 1000,
                escaped: false,
                glitched: false,
            }),
        ];

        let pixels = colorizer.run_pipeline(&data, &palette, 2, 1);

        assert_eq!(pixels.len(), 2);
        // First pixel: escaped at 50% should be mid-gray
        assert!(pixels[0][0] > 100 && pixels[0][0] < 200);
        // Second pixel: interior should be black
        assert_eq!(pixels[1], [0, 0, 0, 255]);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-ui colorizer --no-default-features`

Expected: FAIL with "cannot find enum `ColorizerKind`"

**Step 3: Implement ColorizerKind**

Add to `colorizer.rs`:

```rust
use super::SmoothIterationColorizer;

/// Enum of all available colorizer algorithms.
/// Uses enum dispatch to avoid trait object complexity with associated types.
#[derive(Clone, Debug)]
pub enum ColorizerKind {
    SmoothIteration(SmoothIterationColorizer),
}

impl Default for ColorizerKind {
    fn default() -> Self {
        Self::SmoothIteration(SmoothIterationColorizer)
    }
}

impl ColorizerKind {
    /// Run the full colorization pipeline: preprocess → colorize → postprocess.
    pub fn run_pipeline(
        &self,
        data: &[ComputeData],
        palette: &Palette,
        width: usize,
        height: usize,
    ) -> Vec<[u8; 4]> {
        match self {
            Self::SmoothIteration(c) => {
                let ctx = c.preprocess(data);
                let mut pixels: Vec<[u8; 4]> = data
                    .iter()
                    .map(|d| c.colorize(d, &ctx, palette))
                    .collect();
                c.postprocess(&mut pixels, data, &ctx, palette, width, height);
                pixels
            }
        }
    }

    /// Quick colorization for progressive rendering (no pre/post processing).
    pub fn colorize_quick(&self, data: &ComputeData, palette: &Palette) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => c.colorize(data, &(), palette),
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui colorizer --no-default-features`

Expected: PASS

**Step 5: Export from mod.rs**

```rust
pub use colorizer::ColorizerKind;
```

**Step 6: Run full test suite**

Run: `cargo test -p fractalwonder-ui --no-default-features`

Expected: PASS

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/colorizer.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add ColorizerKind enum with pipeline execution"
```

---

## Task 6: Color Scheme Presets

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/presets.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write tests for presets**

```rust
// fractalwonder-ui/src/rendering/colorizers/presets.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_not_empty() {
        assert!(!presets().is_empty());
    }

    #[test]
    fn all_presets_have_unique_names() {
        let presets = presets();
        let names: Vec<_> = presets.iter().map(|p| p.name).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len(), "Duplicate preset names found");
    }

    #[test]
    fn classic_preset_exists() {
        let presets = presets();
        assert!(presets.iter().any(|p| p.name == "Classic"));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-ui presets --no-default-features`

Expected: FAIL

**Step 3: Implement presets**

```rust
// fractalwonder-ui/src/rendering/colorizers/presets.rs

//! Color scheme presets bundling palettes and colorizers.

use super::{ColorizerKind, Palette, SmoothIterationColorizer};

/// A color scheme preset combining a palette and colorizer.
#[derive(Clone, Debug)]
pub struct ColorSchemePreset {
    pub name: &'static str,
    pub palette: Palette,
    pub colorizer: ColorizerKind,
}

/// Get all available color scheme presets.
pub fn presets() -> Vec<ColorSchemePreset> {
    vec![
        ColorSchemePreset {
            name: "Classic",
            palette: Palette::ultra_fractal(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Fire",
            palette: Palette::fire(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Ocean",
            palette: Palette::ocean(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Electric",
            palette: Palette::electric(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Grayscale",
            palette: Palette::grayscale(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
    ]
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui presets --no-default-features`

Expected: PASS

**Step 5: Export from mod.rs**

```rust
pub mod presets;

pub use presets::{presets, ColorSchemePreset};
```

**Step 6: Run full test suite**

Run: `cargo test -p fractalwonder-ui --no-default-features`

Expected: PASS

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/presets.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add color scheme presets"
```

---

## Task 7: Integrate with Renderer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Update colorize dispatch function**

Modify `fractalwonder-ui/src/rendering/colorizers/mod.rs` to add new colorize function:

```rust
/// Colorize a single pixel using the provided palette and colorizer.
/// For progressive rendering (quick path, no pre/post processing).
pub fn colorize_with_palette(
    data: &ComputeData,
    palette: &Palette,
    colorizer: &ColorizerKind,
    xray_enabled: bool,
) -> [u8; 4] {
    // Handle xray mode for glitched pixels
    if xray_enabled {
        if let ComputeData::Mandelbrot(m) = data {
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

    colorizer.colorize_quick(data, palette)
}
```

**Step 2: Verify existing tests still pass**

Run: `cargo test -p fractalwonder-ui --no-default-features`

Expected: PASS

**Step 3: Commit colorizers changes**

```bash
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add colorize_with_palette function"
```

**Step 4: Update parallel_renderer.rs imports**

At top of file, add:

```rust
use crate::rendering::colorizers::{colorize_with_palette, presets, ColorizerKind, Palette};
```

**Step 5: Add palette and colorizer to ParallelRenderer**

Add fields to `ParallelRenderer` struct:

```rust
pub struct ParallelRenderer {
    // ... existing fields ...
    /// Current palette for colorization
    palette: Rc<RefCell<Palette>>,
    /// Current colorizer algorithm
    colorizer: Rc<RefCell<ColorizerKind>>,
}
```

**Step 6: Initialize in ParallelRenderer::new**

In the `new` function, add initialization:

```rust
let default_preset = &presets()[0]; // Classic
let palette: Rc<RefCell<Palette>> = Rc::new(RefCell::new(default_preset.palette.clone()));
let colorizer: Rc<RefCell<ColorizerKind>> = Rc::new(RefCell::new(default_preset.colorizer.clone()));
```

And add to Self:

```rust
Ok(Self {
    // ... existing fields ...
    palette,
    colorizer,
})
```

**Step 7: Update on_tile_complete closure**

Clone the new refs before the closure:

```rust
let palette_clone = Rc::clone(&palette);
let colorizer_clone = Rc::clone(&colorizer);
```

Update the colorize call in the closure:

```rust
let xray = xray_clone.get();
let pal = palette_clone.borrow();
let col = colorizer_clone.borrow();
let pixels: Vec<u8> = result.data
    .iter()
    .flat_map(|d| colorize_with_palette(d, &pal, &col, xray))
    .collect();
```

**Step 8: Update recolorize method**

```rust
pub fn recolorize(&self) {
    let xray = self.xray_enabled.get();
    let palette = self.palette.borrow();
    let colorizer = self.colorizer.borrow();
    let ctx_ref = self.canvas_ctx.borrow();
    let Some(ctx) = ctx_ref.as_ref() else {
        return;
    };

    for result in self.tile_results.borrow().iter() {
        let pixels: Vec<u8> = result.data
            .iter()
            .flat_map(|d| colorize_with_palette(d, &palette, &colorizer, xray))
            .collect();
        let _ = draw_pixels_to_canvas(
            ctx,
            &pixels,
            result.tile.width,
            result.tile.x as f64,
            result.tile.y as f64,
        );
    }
}
```

**Step 9: Add method to set color scheme**

```rust
/// Set the color scheme (palette and colorizer).
pub fn set_color_scheme(&self, preset: &ColorSchemePreset) {
    *self.palette.borrow_mut() = preset.palette.clone();
    *self.colorizer.borrow_mut() = preset.colorizer.clone();
}

/// Get available color scheme presets.
pub fn color_scheme_presets(&self) -> Vec<ColorSchemePreset> {
    presets()
}
```

**Step 10: Update GPU rendering colorize call**

In `schedule_adam7_pass`, update the colorize call (around line 484-487):

First, add clones before the closure:

```rust
let palette_clone = Rc::clone(&palette);
let colorizer_clone = Rc::clone(&colorizer);
```

Wait - the GPU render path needs access to palette/colorizer too. This requires passing them through the callback chain.

For now, let's use the simpler approach: store them in a way the GPU path can access. Actually, looking at the code, the palette/colorizer are in ParallelRenderer, and we need to pass Rc clones into the orbit_complete_callback.

This is getting complex. Let's do a simpler integration first:

**Step 10 (revised): Keep old colorize for GPU path temporarily**

The GPU rendering path is complex with many closures. For this task, keep the GPU path using the old `colorize` function. We'll refactor it in a follow-up task.

Update only the tile-based paths (on_tile_complete and recolorize).

For GPU path, the colorization happens in line 484-487. Keep it as:
```rust
let xray = xray_enabled_spawn.get();
let pixels: Vec<u8> = display_data
    .iter()
    .flat_map(|d| colorize(d, xray))
    .collect();
```

This means GPU rendering stays grayscale for now. Document this limitation.

**Step 11: Build and test**

Run: `cargo build -p fractalwonder-ui --no-default-features`

Expected: PASS (may need to fix import issues)

Run: `cargo test -p fractalwonder-ui --no-default-features`

Expected: PASS

**Step 12: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(renderer): integrate color palettes (tile path)

GPU rendering path still uses grayscale - will be updated in follow-up."
```

---

## Task 8: Run Full Quality Checks

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Fix any warnings.

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features`

**Step 4: Check WASM build**

Run: `cargo build --target wasm32-unknown-unknown -p fractalwonder-ui`

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Summary

After completing all tasks, you will have:

1. **color_space.rs** - OKLAB conversion functions
2. **palette.rs** - Palette struct with OKLAB interpolation and 5 presets
3. **colorizer.rs** - Colorizer trait and ColorizerKind enum
4. **smooth_iteration.rs** - SmoothIterationColorizer implementation
5. **presets.rs** - ColorSchemePreset bundles for UI
6. **parallel_renderer.rs** - Integration with tile rendering path

**Known limitations to address in follow-up:**
- GPU rendering path still uses old grayscale colorize
- No UI dropdown yet (requires Leptos component work)
- Smooth iteration uses linear normalization (needs `final_z_norm_sq` from Increment 2)
