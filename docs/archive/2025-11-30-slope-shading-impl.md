# Slope Shading Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add slope shading (3D lighting effect) to the colorization pipeline using 8-neighbor gradient computation.

**Architecture:** Extend the colorizer trait to use `Vec<f64>` context for smooth iterations, add `ColorSettings` struct with shading parameters, implement postprocess stage that applies slope shading based on iteration height field.

**Tech Stack:** Rust, Leptos, WASM

---

## Task 1: Add ShadingSettings Struct

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/settings.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs:1-12`

**Step 1: Create settings.rs with ShadingSettings**

```rust
//! Color settings for the colorization pipeline.

/// Settings for slope shading effect.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadingSettings {
    /// Whether slope shading is enabled.
    pub enabled: bool,
    /// Light angle in radians. 0 = right, π/2 = top.
    pub light_angle: f64,
    /// Base height factor, auto-scaled by zoom level.
    pub height_factor: f64,
    /// Blend strength. 0.0 = no shading, 1.0 = full effect.
    pub blend: f64,
}

impl Default for ShadingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            light_angle: std::f64::consts::FRAC_PI_4, // 45° (top-right)
            height_factor: 1.5,
            blend: 0.7,
        }
    }
}

impl ShadingSettings {
    /// Shading disabled.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Default enabled shading with top-right light.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shading_is_disabled() {
        let settings = ShadingSettings::default();
        assert!(!settings.enabled);
    }

    #[test]
    fn enabled_shading_has_reasonable_defaults() {
        let settings = ShadingSettings::enabled();
        assert!(settings.enabled);
        assert!(settings.light_angle > 0.0);
        assert!(settings.height_factor > 0.0);
        assert!(settings.blend > 0.0 && settings.blend <= 1.0);
    }
}
```

**Step 2: Run test to verify it compiles**

Run: `cargo test -p fractalwonder-ui settings --no-run`
Expected: Compiles (test binary built)

**Step 3: Run the tests**

Run: `cargo test -p fractalwonder-ui settings`
Expected: 2 tests pass

**Step 4: Add module to mod.rs**

In `fractalwonder-ui/src/rendering/colorizers/mod.rs`, add after line 5:

```rust
pub mod settings;
```

And add to re-exports (after line 12):

```rust
pub use settings::ShadingSettings;
```

**Step 5: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/settings.rs fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add ShadingSettings struct"
```

---

## Task 2: Add ColorSettings Struct

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/settings.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Add ColorSettings to settings.rs**

Append to `settings.rs`:

```rust
use super::Palette;

/// All settings that affect colorization (not compute).
#[derive(Clone, Debug)]
pub struct ColorSettings {
    /// Color palette for mapping iteration values to colors.
    pub palette: Palette,
    /// Number of times to cycle through the palette.
    pub cycle_count: f64,
    /// Slope shading settings.
    pub shading: ShadingSettings,
}

impl Default for ColorSettings {
    fn default() -> Self {
        Self {
            palette: Palette::ultra_fractal(),
            cycle_count: 1.0,
            shading: ShadingSettings::default(),
        }
    }
}

impl ColorSettings {
    /// Create settings with the given palette and default shading.
    pub fn with_palette(palette: Palette) -> Self {
        Self {
            palette,
            cycle_count: 1.0,
            shading: ShadingSettings::default(),
        }
    }

    /// Create settings with shading enabled.
    pub fn with_shading(palette: Palette) -> Self {
        Self {
            palette,
            cycle_count: 1.0,
            shading: ShadingSettings::enabled(),
        }
    }
}
```

**Step 2: Add test for ColorSettings**

Append to tests module in `settings.rs`:

```rust
    #[test]
    fn color_settings_default_has_palette() {
        let settings = ColorSettings::default();
        assert_eq!(settings.cycle_count, 1.0);
        assert!(!settings.shading.enabled);
    }

    #[test]
    fn with_shading_enables_shading() {
        let settings = ColorSettings::with_shading(Palette::grayscale());
        assert!(settings.shading.enabled);
    }
```

**Step 3: Update mod.rs re-export**

Change the settings re-export line in `mod.rs` to:

```rust
pub use settings::{ColorSettings, ShadingSettings};
```

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-ui settings`
Expected: 4 tests pass

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/settings.rs fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add ColorSettings struct"
```

---

## Task 3: Add Smooth Iteration Computation Helper

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs`

**Step 1: Extract smooth iteration computation to a helper function**

Add this function before the `impl Colorizer for SmoothIterationColorizer` block (around line 11):

```rust
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
```

**Step 2: Add test for the helper**

Add to the tests module:

```rust
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
        assert!(smooth > 9.0 && smooth < 11.0, "smooth = {}", smooth);
    }
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui smooth_iteration`
Expected: All tests pass (existing + 2 new)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs
git commit -m "refactor(colorizers): extract compute_smooth_iteration helper"
```

---

## Task 4: Change Context to Vec<f64> and Update Preprocess

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/colorizer.rs`

**Step 1: Update SmoothIterationColorizer Context type**

In `smooth_iteration.rs`, change the `impl Colorizer` block:

```rust
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

    fn colorize(&self, data: &ComputeData, context: &Self::Context, palette: &Palette, index: usize) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => {
                let smooth = if index < context.len() {
                    context[index]
                } else {
                    compute_smooth_iteration(m)
                };
                self.colorize_mandelbrot_smooth(m, smooth, palette)
            }
            ComputeData::TestImage(_) => [128, 128, 128, 255],
        }
    }
}
```

**Step 2: Update the colorize_mandelbrot helper**

Replace `colorize_mandelbrot` with:

```rust
impl SmoothIterationColorizer {
    fn colorize_mandelbrot_smooth(&self, data: &MandelbrotData, smooth: f64, palette: &Palette) -> [u8; 4] {
        if !data.escaped {
            return [0, 0, 0, 255];
        }

        if data.max_iterations == 0 {
            return [0, 0, 0, 255];
        }

        let t = (smooth / data.max_iterations as f64).clamp(0.0, 1.0);
        let [r, g, b] = palette.sample(t);
        [r, g, b, 255]
    }
}
```

**Step 3: Update the Colorizer trait signature**

In `colorizer.rs`, update the `colorize` method signature (line 23):

```rust
    fn colorize(&self, data: &ComputeData, context: &Self::Context, palette: &Palette, index: usize) -> [u8; 4];
```

**Step 4: Update run_pipeline to pass index**

In `colorizer.rs`, update the `run_pipeline` method (lines 61-69):

```rust
            Self::SmoothIteration(c) => {
                let ctx = c.preprocess(data);
                let mut pixels: Vec<[u8; 4]> = data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| c.colorize(d, &ctx, palette, i))
                    .collect();
                c.postprocess(&mut pixels, data, &ctx, palette, width, height);
                pixels
            }
```

**Step 5: Update colorize_quick**

In `colorizer.rs`, update `colorize_quick` (lines 74-78):

```rust
    pub fn colorize_quick(&self, data: &ComputeData, palette: &Palette) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => c.colorize(data, &Vec::new(), palette, 0),
        }
    }
```

**Step 6: Fix tests in colorizer.rs**

The test `colorizer_kind_runs_pipeline` should still work. Run:

Run: `cargo test -p fractalwonder-ui colorizer`
Expected: Tests pass

**Step 7: Fix tests in smooth_iteration.rs**

Update existing tests to pass index parameter. In `smooth_iteration.rs` tests, update calls like:

```rust
    let color = colorizer.colorize(&make_interior(), &vec![], &palette, 0);
```

Run: `cargo test -p fractalwonder-ui smooth_iteration`
Expected: All tests pass

**Step 8: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs fractalwonder-ui/src/rendering/colorizers/colorizer.rs
git commit -m "refactor(colorizers): change Context to Vec<f64> for smooth iterations"
```

---

## Task 5: Create Shading Module with Core Algorithm

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/shading.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Create shading.rs with mirror_coord helper**

```rust
//! Slope shading for 3D lighting effect on iteration height field.

use super::ShadingSettings;
use fractalwonder_core::ComputeData;

/// Mirror a coordinate at boundaries for seamless edge handling.
fn mirror_coord(coord: i32, max: usize) -> usize {
    if coord < 0 {
        (-coord).min(max as i32 - 1) as usize
    } else if coord >= max as i32 {
        let reflected = 2 * max as i32 - coord - 2;
        reflected.max(0) as usize
    } else {
        coord as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_coord_in_bounds() {
        assert_eq!(mirror_coord(5, 10), 5);
        assert_eq!(mirror_coord(0, 10), 0);
        assert_eq!(mirror_coord(9, 10), 9);
    }

    #[test]
    fn mirror_coord_negative() {
        assert_eq!(mirror_coord(-1, 10), 1);
        assert_eq!(mirror_coord(-2, 10), 2);
    }

    #[test]
    fn mirror_coord_beyond_max() {
        assert_eq!(mirror_coord(10, 10), 8);
        assert_eq!(mirror_coord(11, 10), 7);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-ui shading`
Expected: 3 tests pass

**Step 3: Add module to mod.rs**

Add after line 5 in `mod.rs`:

```rust
pub mod shading;
```

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/shading.rs fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add shading module with mirror_coord"
```

---

## Task 6: Add 8-Neighbor Shade Computation

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/shading.rs`

**Step 1: Add compute_shade_8neighbor function**

Add after `mirror_coord`:

```rust
/// Compute shade value for a single pixel using 8-neighbor gradient.
/// Returns value in [0, 1] range where 0.5 is neutral.
fn compute_shade_8neighbor(
    smooth_iters: &[f64],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    light_x: f64,
    light_y: f64,
    height_factor: f64,
) -> f64 {
    let get = |dx: i32, dy: i32| -> f64 {
        let nx = mirror_coord(x as i32 + dx, width);
        let ny = mirror_coord(y as i32 + dy, height);
        smooth_iters[ny * width + nx]
    };

    let center = get(0, 0);

    let neighbors: [(i32, i32); 8] = [
        (-1, -1), (0, -1), (1, -1),
        (-1,  0),          (1,  0),
        (-1,  1), (0,  1), (1,  1),
    ];

    let mut running_sum = 0.0;
    let mut high = center;
    let mut low = center;

    for (dx, dy) in neighbors {
        let neighbor_val = get(dx, dy);
        high = high.max(neighbor_val);
        low = low.min(neighbor_val);

        let diff = neighbor_val - center;

        // Apply direction based on light position
        let h_diff = if dx < 0 { -diff } else { diff };
        let v_diff = if dy > 0 { -diff } else { diff };

        if dx != 0 {
            running_sum += h_diff * light_x.abs();
        }
        if dy != 0 {
            running_sum += v_diff * light_y.abs();
        }
    }

    // Normalize by range to avoid extreme values
    let range = high - low;
    let slope = if range > 1e-10 {
        (running_sum * height_factor) / range
    } else {
        0.0
    };

    // Map slope to [0, 1] using sigmoid-like function
    (slope / (1.0 + slope.abs()) + 1.0) / 2.0
}
```

**Step 2: Add test for shade computation**

Add to tests module:

```rust
    #[test]
    fn shade_flat_region_is_neutral() {
        // All same values = no slope = neutral shade (0.5)
        let iters = vec![10.0; 9];
        let shade = compute_shade_8neighbor(&iters, 3, 3, 1, 1, 1.0, 1.0, 1.0);
        assert!((shade - 0.5).abs() < 0.01, "shade = {}", shade);
    }

    #[test]
    fn shade_slope_facing_light_is_bright() {
        // Higher values to the right and top = slope facing top-right light
        #[rustfmt::skip]
        let iters = vec![
            1.0, 2.0, 3.0,
            2.0, 3.0, 4.0,
            3.0, 4.0, 5.0,
        ];
        let shade = compute_shade_8neighbor(&iters, 3, 3, 1, 1, 1.0, 1.0, 1.0);
        assert!(shade > 0.5, "shade facing light should be > 0.5, got {}", shade);
    }

    #[test]
    fn shade_slope_away_from_light_is_dark() {
        // Higher values to the left and bottom = slope away from top-right light
        #[rustfmt::skip]
        let iters = vec![
            5.0, 4.0, 3.0,
            4.0, 3.0, 2.0,
            3.0, 2.0, 1.0,
        ];
        let shade = compute_shade_8neighbor(&iters, 3, 3, 1, 1, 1.0, 1.0, 1.0);
        assert!(shade < 0.5, "shade away from light should be < 0.5, got {}", shade);
    }
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui shading`
Expected: 6 tests pass

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/shading.rs
git commit -m "feat(colorizers): add 8-neighbor shade computation"
```

---

## Task 7: Add Blend and Apply Functions

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/shading.rs`

**Step 1: Add blend_shade function**

Add after `compute_shade_8neighbor`:

```rust
/// Blend shade value with a pixel color.
/// shade: 0.5 = neutral, <0.5 = darken, >0.5 = lighten
fn blend_shade(base: [u8; 4], shade: f64, blend: f64) -> [u8; 4] {
    if blend <= 0.0 {
        return base;
    }

    // shade 0.5 = factor 1.0, shade 0 = factor 0.3, shade 1 = factor 1.7
    let factor = 0.3 + shade * 1.4;

    let apply = |c: u8| -> u8 {
        let shaded = (c as f64 * factor).clamp(0.0, 255.0);
        let blended = c as f64 + blend * (shaded - c as f64);
        blended.clamp(0.0, 255.0) as u8
    };

    [apply(base[0]), apply(base[1]), apply(base[2]), base[3]]
}
```

**Step 2: Add test for blend**

Add to tests:

```rust
    #[test]
    fn blend_neutral_unchanged() {
        let base = [128, 128, 128, 255];
        let result = blend_shade(base, 0.5, 1.0);
        // Factor = 0.3 + 0.5 * 1.4 = 1.0, so unchanged
        assert_eq!(result, base);
    }

    #[test]
    fn blend_dark_darkens() {
        let base = [128, 128, 128, 255];
        let result = blend_shade(base, 0.0, 1.0);
        // Factor = 0.3, so darkened
        assert!(result[0] < base[0], "expected darker, got {:?}", result);
    }

    #[test]
    fn blend_bright_brightens() {
        let base = [128, 128, 128, 255];
        let result = blend_shade(base, 1.0, 1.0);
        // Factor = 1.7, so brightened
        assert!(result[0] > base[0], "expected brighter, got {:?}", result);
    }

    #[test]
    fn blend_zero_unchanged() {
        let base = [128, 128, 128, 255];
        let result = blend_shade(base, 0.0, 0.0);
        assert_eq!(result, base);
    }
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui shading`
Expected: 10 tests pass

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/shading.rs
git commit -m "feat(colorizers): add shade blending function"
```

---

## Task 8: Add Main apply_slope_shading Function

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/shading.rs`

**Step 1: Add is_interior helper**

Add after imports:

```rust
/// Check if a compute data point is interior (didn't escape).
fn is_interior(data: &ComputeData) -> bool {
    match data {
        ComputeData::Mandelbrot(m) => !m.escaped,
        ComputeData::TestImage(_) => false,
    }
}
```

**Step 2: Add apply_slope_shading function**

Add as public function:

```rust
/// Apply slope shading to a pixel buffer in place.
///
/// # Arguments
/// * `pixels` - RGBA pixel buffer to modify
/// * `data` - Original compute data (to check for interior points)
/// * `smooth_iters` - Precomputed smooth iteration values
/// * `settings` - Shading settings
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `zoom_level` - Current zoom level for auto-scaling height factor
pub fn apply_slope_shading(
    pixels: &mut [[u8; 4]],
    data: &[ComputeData],
    smooth_iters: &[f64],
    settings: &ShadingSettings,
    width: usize,
    height: usize,
    zoom_level: f64,
) {
    if !settings.enabled || settings.blend <= 0.0 {
        return;
    }

    // Auto-scale height factor with zoom
    let effective_height = settings.height_factor * (1.0 + zoom_level.log10().max(0.0) / 10.0);

    let light_x = settings.light_angle.cos();
    let light_y = settings.light_angle.sin();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;

            // Skip interior pixels - keep them pure black
            if is_interior(&data[idx]) {
                continue;
            }

            let shade = compute_shade_8neighbor(
                smooth_iters,
                width,
                height,
                x,
                y,
                light_x,
                light_y,
                effective_height,
            );

            pixels[idx] = blend_shade(pixels[idx], shade, settings.blend);
        }
    }
}
```

**Step 3: Add integration test**

Add to tests:

```rust
    use fractalwonder_core::MandelbrotData;

    fn make_exterior_data(iterations: u32) -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations: 100,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
        })
    }

    fn make_interior_data() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 100,
            max_iterations: 100,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 4.0,
        })
    }

    #[test]
    fn apply_shading_disabled_no_change() {
        let mut pixels = vec![[128, 128, 128, 255]; 9];
        let original = pixels.clone();
        let data: Vec<_> = (0..9).map(|i| make_exterior_data(i as u32 * 10)).collect();
        let smooth: Vec<_> = (0..9).map(|i| i as f64 * 10.0).collect();
        let settings = ShadingSettings::disabled();

        apply_slope_shading(&mut pixels, &data, &smooth, &settings, 3, 3, 1.0);

        assert_eq!(pixels, original);
    }

    #[test]
    fn apply_shading_interior_unchanged() {
        let mut pixels = vec![[0, 0, 0, 255]; 9];
        let original = pixels.clone();
        let data = vec![make_interior_data(); 9];
        let smooth = vec![100.0; 9];
        let settings = ShadingSettings::enabled();

        apply_slope_shading(&mut pixels, &data, &smooth, &settings, 3, 3, 1.0);

        assert_eq!(pixels, original);
    }

    #[test]
    fn apply_shading_modifies_exterior() {
        let mut pixels = vec![[128, 128, 128, 255]; 9];
        let original = pixels.clone();
        // Create gradient in iterations
        let data: Vec<_> = (0..9).map(|i| make_exterior_data(i as u32 * 10)).collect();
        let smooth: Vec<_> = (0..9).map(|i| i as f64 * 10.0).collect();
        let settings = ShadingSettings::enabled();

        apply_slope_shading(&mut pixels, &data, &smooth, &settings, 3, 3, 1.0);

        // At least some pixels should be modified (the center has neighbors)
        assert_ne!(pixels, original, "shading should modify some pixels");
    }
```

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-ui shading`
Expected: 13 tests pass

**Step 5: Export function from mod.rs**

Add to `mod.rs` re-exports:

```rust
pub use shading::apply_slope_shading;
```

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/shading.rs fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add apply_slope_shading function"
```

---

## Task 9: Integrate Shading into Colorizer Postprocess

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/colorizer.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs`

**Step 1: Update Colorizer trait postprocess signature**

In `colorizer.rs`, update the `postprocess` method (around line 27):

```rust
    fn postprocess(
        &self,
        _pixels: &mut [[u8; 4]],
        _data: &[ComputeData],
        _context: &Self::Context,
        _settings: &ColorSettings,
        _width: usize,
        _height: usize,
        _zoom_level: f64,
    ) {
    }
```

**Step 2: Update run_pipeline signature and implementation**

In `colorizer.rs`, update `run_pipeline` (around line 53):

```rust
    pub fn run_pipeline(
        &self,
        data: &[ComputeData],
        settings: &ColorSettings,
        width: usize,
        height: usize,
        zoom_level: f64,
    ) -> Vec<[u8; 4]> {
        match self {
            Self::SmoothIteration(c) => {
                let ctx = c.preprocess(data);
                let mut pixels: Vec<[u8; 4]> = data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| c.colorize(d, &ctx, &settings.palette, i))
                    .collect();
                c.postprocess(&mut pixels, data, &ctx, settings, width, height, zoom_level);
                pixels
            }
        }
    }
```

**Step 3: Import ColorSettings in colorizer.rs**

Update imports at top of `colorizer.rs`:

```rust
use super::{ColorSettings, Palette, SmoothIterationColorizer};
```

**Step 4: Implement postprocess in SmoothIterationColorizer**

In `smooth_iteration.rs`, add the postprocess implementation and import:

Add to imports:

```rust
use super::{shading::apply_slope_shading, ColorSettings, Colorizer, Palette};
```

Add to the `impl Colorizer for SmoothIterationColorizer` block:

```rust
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
        apply_slope_shading(pixels, data, context, &settings.shading, width, height, zoom_level);
    }
```

**Step 5: Update colorize signature to use ColorSettings**

Update `colorize` method signature:

```rust
    fn colorize(&self, data: &ComputeData, context: &Self::Context, settings: &ColorSettings, index: usize) -> [u8; 4] {
        match data {
            ComputeData::Mandelbrot(m) => {
                let smooth = if index < context.len() {
                    context[index]
                } else {
                    compute_smooth_iteration(m)
                };
                self.colorize_mandelbrot_smooth(m, smooth, &settings.palette)
            }
            ComputeData::TestImage(_) => [128, 128, 128, 255],
        }
    }
```

**Step 6: Update Colorizer trait colorize signature**

In `colorizer.rs`, update trait:

```rust
    fn colorize(&self, data: &ComputeData, context: &Self::Context, settings: &ColorSettings, index: usize) -> [u8; 4];
```

**Step 7: Update colorize_quick**

In `colorizer.rs`:

```rust
    pub fn colorize_quick(&self, data: &ComputeData, settings: &ColorSettings) -> [u8; 4] {
        match self {
            Self::SmoothIteration(c) => c.colorize(data, &Vec::new(), settings, 0),
        }
    }
```

**Step 8: Fix tests**

Update tests in both files to use `ColorSettings`:

In `colorizer.rs` tests:

```rust
    use super::*;
    use crate::rendering::colorizers::{ColorSettings, Palette, SmoothIterationColorizer};
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn colorizer_kind_runs_pipeline() {
        let colorizer = ColorizerKind::SmoothIteration(SmoothIterationColorizer);
        let settings = ColorSettings::with_palette(Palette::grayscale());

        let data = vec![
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 500,
                max_iterations: 1000,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
            }),
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 0,
                max_iterations: 1000,
                escaped: false,
                glitched: false,
                final_z_norm_sq: 0.0,
            }),
        ];

        let pixels = colorizer.run_pipeline(&data, &settings, 2, 1, 1.0);

        assert_eq!(pixels.len(), 2);
        assert!(
            pixels[0][0] > 50 && pixels[0][0] < 150,
            "Expected mid gray, got {:?}",
            pixels[0]
        );
        assert_eq!(pixels[1], [0, 0, 0, 255]);
    }
```

In `smooth_iteration.rs` tests, update all `colorize` calls:

```rust
    let settings = ColorSettings::with_palette(Palette::grayscale());
    let color = colorizer.colorize(&make_interior(), &vec![], &settings, 0);
```

**Step 9: Run all tests**

Run: `cargo test -p fractalwonder-ui`
Expected: All tests pass

**Step 10: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/
git commit -m "feat(colorizers): integrate shading into postprocess pipeline"
```

---

## Task 10: Update Presets to Use ColorSettings

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/presets.rs`

**Step 1: Update ColorSchemePreset struct**

Replace the entire file:

```rust
//! Color scheme presets bundling settings and colorizers.

use super::{ColorSettings, ColorizerKind, Palette, ShadingSettings, SmoothIterationColorizer};

/// A color scheme preset combining settings and colorizer.
#[derive(Clone, Debug)]
pub struct ColorSchemePreset {
    pub name: &'static str,
    pub settings: ColorSettings,
    pub colorizer: ColorizerKind,
}

/// Get all available color scheme presets.
pub fn presets() -> Vec<ColorSchemePreset> {
    vec![
        // Non-shaded palettes
        ColorSchemePreset {
            name: "Classic",
            settings: ColorSettings::with_palette(Palette::ultra_fractal()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Fire",
            settings: ColorSettings::with_palette(Palette::fire()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Ocean",
            settings: ColorSettings::with_palette(Palette::ocean()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Electric",
            settings: ColorSettings::with_palette(Palette::electric()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Grayscale",
            settings: ColorSettings::with_palette(Palette::grayscale()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        // Shaded variants
        ColorSchemePreset {
            name: "Classic 3D",
            settings: ColorSettings::with_shading(Palette::ultra_fractal()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Fire 3D",
            settings: ColorSettings::with_shading(Palette::fire()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Ocean 3D",
            settings: ColorSettings::with_shading(Palette::ocean()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        // Cycling palettes
        ColorSchemePreset {
            name: "Rainbow",
            settings: ColorSettings::with_palette(Palette::rainbow()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Neon",
            settings: ColorSettings::with_palette(Palette::neon()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Twilight",
            settings: ColorSettings::with_palette(Palette::twilight()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Candy",
            settings: ColorSettings::with_palette(Palette::candy()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Inferno",
            settings: ColorSettings::with_palette(Palette::inferno()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        ColorSchemePreset {
            name: "Aurora",
            settings: ColorSettings::with_palette(Palette::aurora()),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
    ]
}

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

    #[test]
    fn shaded_presets_have_shading_enabled() {
        let presets = presets();
        for preset in presets.iter().filter(|p| p.name.contains("3D")) {
            assert!(preset.settings.shading.enabled, "{} should have shading enabled", preset.name);
        }
    }
}
```

**Step 2: Update mod.rs re-export**

Change in `mod.rs`:

```rust
pub use presets::{presets, ColorSchemePreset};
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui presets`
Expected: 4 tests pass

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/presets.rs fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): update presets to use ColorSettings with 3D variants"
```

---

## Task 11: Update ParallelRenderer for ColorSettings

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Update colorize_with_palette to use ColorSettings**

In `mod.rs`, replace the `colorize_with_palette` function:

```rust
/// Colorize a single pixel using the provided settings and colorizer.
/// For progressive rendering (quick path, no pre/post processing).
pub fn colorize_with_palette(
    data: &ComputeData,
    settings: &ColorSettings,
    colorizer: &ColorizerKind,
) -> [u8; 4] {
    colorizer.colorize_quick(data, settings)
}
```

**Step 2: Update ParallelRenderer fields**

In `parallel_renderer.rs`, replace `palette: Rc<RefCell<Palette>>` with `settings`:

Change line 42-44:

```rust
    /// Current color settings
    settings: Rc<RefCell<ColorSettings>>,
    /// Current colorizer algorithm
    colorizer: Rc<RefCell<ColorizerKind>>,
```

**Step 3: Update imports in parallel_renderer.rs**

Update imports at top (lines 5-6):

```rust
use crate::rendering::colorizers::{
    colorize_with_palette, presets, ColorSchemePreset, ColorSettings, ColorizerKind,
};
```

**Step 4: Update ParallelRenderer::new**

Update initialization (around lines 60-64):

```rust
        let default_preset = &presets()[0];
        let settings: Rc<RefCell<ColorSettings>> = Rc::new(RefCell::new(default_preset.settings.clone()));
        let colorizer: Rc<RefCell<ColorizerKind>> =
            Rc::new(RefCell::new(default_preset.colorizer.clone()));
```

**Step 5: Update on_tile_complete callback**

Update the callback (around lines 71-81):

```rust
        let settings_clone = Rc::clone(&settings);
        let colorizer_clone = Rc::clone(&colorizer);
        let on_tile_complete = move |result: TileResult| {
            if let Some(ctx) = ctx_clone.borrow().as_ref() {
                let s = settings_clone.borrow();
                let col = colorizer_clone.borrow();
                let pixels: Vec<u8> = result
                    .data
                    .iter()
                    .flat_map(|d| colorize_with_palette(d, &s, &col))
                    .collect();
                // ... rest unchanged
            }
        };
```

**Step 6: Update struct initialization**

Change `palette` to `settings` in the Self return (around line 112):

```rust
            settings,
            colorizer,
```

**Step 7: Update recolorize method**

Update the method (around lines 123-146):

```rust
    pub fn recolorize(&self) {
        let settings = self.settings.borrow();
        let colorizer = self.colorizer.borrow();
        let ctx_ref = self.canvas_ctx.borrow();
        let Some(ctx) = ctx_ref.as_ref() else {
            return;
        };

        for result in self.tile_results.borrow().iter() {
            let pixels: Vec<u8> = result
                .data
                .iter()
                .flat_map(|d| colorize_with_palette(d, &settings, &colorizer))
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

**Step 8: Update set_color_scheme**

Update the method (around lines 162-165):

```rust
    pub fn set_color_scheme(&self, preset: &ColorSchemePreset) {
        *self.settings.borrow_mut() = preset.settings.clone();
        *self.colorizer.borrow_mut() = preset.colorizer.clone();
    }
```

**Step 9: Update GPU render callback**

This is extensive. Update the GPU render path to pass settings instead of palette:

Around line 253, change `palette` clone to `settings`:

```rust
            let settings = Rc::clone(&self.settings);
```

And update all the schedule_adam7_pass calls and the function signature to use `settings: Rc<RefCell<ColorSettings>>` instead of `palette: Rc<RefCell<Palette>>`.

In `schedule_adam7_pass` function signature (around line 447):

```rust
    settings: Rc<RefCell<ColorSettings>>,
```

And inside the function, update the colorization (around line 549):

```rust
                    let s = settings_spawn.borrow();
                    let col = colorizer_spawn.borrow();
                    let pixels: Vec<u8> = display_data
                        .iter()
                        .flat_map(|d| colorize_with_palette(d, &s, &col))
                        .collect();
```

**Step 10: Run check**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles (may have warnings)

**Step 11: Run tests**

Run: `cargo test -p fractalwonder-ui`
Expected: Tests pass

**Step 12: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/mod.rs fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "refactor(renderer): use ColorSettings instead of Palette"
```

---

## Task 12: Add Full Pipeline Recolorize with Shading

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Add recolorize_full method**

Add after the `recolorize` method:

```rust
    /// Re-colorize with full pipeline (including shading postprocess).
    /// Use this when shading settings may have changed.
    pub fn recolorize_full(&self, zoom_level: f64) {
        let settings = self.settings.borrow();
        let colorizer = self.colorizer.borrow();
        let ctx_ref = self.canvas_ctx.borrow();
        let Some(ctx) = ctx_ref.as_ref() else {
            return;
        };

        for result in self.tile_results.borrow().iter() {
            let pixels = colorizer.run_pipeline(
                &result.data,
                &settings,
                result.tile.width as usize,
                result.tile.height as usize,
                zoom_level,
            );
            let pixel_bytes: Vec<u8> = pixels.into_iter().flat_map(|p| p).collect();
            let _ = draw_pixels_to_canvas(
                ctx,
                &pixel_bytes,
                result.tile.width,
                result.tile.x as f64,
                result.tile.y as f64,
            );
        }
    }
```

**Step 2: Run check**

Run: `cargo check -p fractalwonder-ui`
Expected: Compiles

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(renderer): add recolorize_full for shaded re-render"
```

---

## Task 13: Final Integration Test

**Files:**
- Run full test suite and browser verification

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run fmt check**

Run: `cargo fmt --all -- --check`
Expected: No formatting issues

**Step 4: Build for WASM**

Run: `trunk build`
Expected: Builds successfully

**Step 5: Manual browser test**

1. Open http://localhost:8080
2. Select "Classic 3D" from color menu
3. Verify 3D shading effect is visible
4. Zoom in and verify shading scales appropriately
5. Switch to non-3D preset, verify shading disappears

**Step 6: Commit any final fixes and tag**

```bash
git add -A
git commit -m "feat(colorizers): complete slope shading implementation"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add ShadingSettings struct | settings.rs (new), mod.rs |
| 2 | Add ColorSettings struct | settings.rs, mod.rs |
| 3 | Extract smooth iteration helper | smooth_iteration.rs |
| 4 | Change Context to Vec<f64> | smooth_iteration.rs, colorizer.rs |
| 5 | Create shading module | shading.rs (new), mod.rs |
| 6 | Add 8-neighbor shade computation | shading.rs |
| 7 | Add blend function | shading.rs |
| 8 | Add apply_slope_shading | shading.rs, mod.rs |
| 9 | Integrate into postprocess | colorizer.rs, smooth_iteration.rs |
| 10 | Update presets | presets.rs |
| 11 | Update ParallelRenderer | parallel_renderer.rs, mod.rs |
| 12 | Add recolorize_full | parallel_renderer.rs |
| 13 | Final integration test | All |
