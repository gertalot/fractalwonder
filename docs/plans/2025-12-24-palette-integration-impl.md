# Palette Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the legacy palette system with the unified `Palette` struct. Delete `palette_lut.rs`, `settings.rs`, `ColorOptions`, `ShadingSettings`, `PaletteEntry`, and `palettes()`.

**Architecture:** Bottom-up replacement. Update internal plumbing first (colorizers), then delete legacy code, then update callers. Each phase compiles and tests.

**Tech Stack:** Rust, Leptos, WASM, serde_json

---

## Phase 1: Core Plumbing

### Task 1: Add PaletteLut to palette.rs

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/palette.rs`

**Step 1: Add PaletteLut struct after Palette impl**

Add at the end of `palette.rs` (before `#[cfg(test)]`):

```rust
/// Pre-computed lookup table for fast color sampling.
/// Generated from a Palette's gradient.
pub struct PaletteLut {
    lut: Vec<[u8; 3]>,
}

impl PaletteLut {
    /// Create from a Palette.
    pub fn from_palette(palette: &Palette) -> Self {
        Self {
            lut: palette.to_lut(),
        }
    }

    /// Sample the palette at position t ∈ [0,1].
    #[inline]
    pub fn sample(&self, t: f64) -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);
        let index = ((t * 4095.0) as usize).min(4095);
        self.lut[index]
    }
}
```

**Step 2: Add test**

Add to the `tests` module:

```rust
#[test]
fn palette_lut_from_palette_samples_correctly() {
    let palette = Palette::default();
    let lut = PaletteLut::from_palette(&palette);
    // Default palette is black to white
    assert_eq!(lut.sample(0.0), [0, 0, 0]);
    assert_eq!(lut.sample(1.0), [255, 255, 255]);
}
```

**Step 3: Run tests**

```bash
cargo test --package fractalwonder-ui palette_lut_from_palette -- --nocapture
```

Expected: PASS

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/palette.rs
git commit -m "feat(palette): add PaletteLut::from_palette for LUT caching"
```

---

### Task 2: Update Colorizer trait signature

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/colorizer.rs`

**Step 1: Update imports**

Replace the imports at top:

```rust
use super::smooth_iteration::SmoothIterationContext;
use super::{Palette, PaletteLut, RenderSettings, SmoothIterationColorizer};
use fractalwonder_core::ComputeData;
```

**Step 2: Update Colorizer trait**

Replace the trait definition:

```rust
pub trait Colorizer {
    type Context: Default;

    fn preprocess(&self, _data: &[ComputeData], _palette: &Palette) -> Self::Context {
        Self::Context::default()
    }

    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        palette: &Palette,
        lut: &PaletteLut,
        render_settings: &RenderSettings,
        index: usize,
    ) -> [u8; 4];

    #[allow(clippy::too_many_arguments)]
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

**Step 3: Update ColorizerKind::run_pipeline**

Replace the `run_pipeline` method:

```rust
#[allow(clippy::too_many_arguments)]
pub fn run_pipeline(
    &self,
    data: &[ComputeData],
    palette: &Palette,
    lut: &PaletteLut,
    render_settings: &RenderSettings,
    width: usize,
    height: usize,
    xray_enabled: bool,
) -> Vec<[u8; 4]> {
    match self {
        Self::SmoothIteration(c) => {
            let ctx = c.preprocess(data, palette);
            let mut pixels: Vec<[u8; 4]> = data
                .iter()
                .enumerate()
                .map(|(i, d)| c.colorize(d, &ctx, palette, lut, render_settings, i))
                .collect();
            c.postprocess(&mut pixels, data, &ctx, palette, width, height);

            if xray_enabled {
                apply_xray_to_glitched(&mut pixels, data);
            }

            pixels
        }
    }
}
```

**Step 4: Update ColorizerKind::colorize**

Replace the `colorize` method:

```rust
pub fn colorize(
    &self,
    data: &ComputeData,
    palette: &Palette,
    lut: &PaletteLut,
    render_settings: &RenderSettings,
) -> [u8; 4] {
    match self {
        Self::SmoothIteration(c) => c.colorize(
            data,
            &SmoothIterationContext::default(),
            palette,
            lut,
            render_settings,
            0,
        ),
    }
}
```

**Step 5: Update ColorizerKind::colorize_with_cached_histogram**

Replace the method:

```rust
pub fn colorize_with_cached_histogram(
    &self,
    data: &ComputeData,
    cached_context: &SmoothIterationContext,
    palette: &Palette,
    lut: &PaletteLut,
    render_settings: &RenderSettings,
) -> [u8; 4] {
    match self {
        Self::SmoothIteration(c) => {
            c.colorize_with_histogram(data, cached_context, palette, lut, render_settings)
        }
    }
}
```

**Step 6: Update ColorizerKind::create_context**

Replace the method:

```rust
pub fn create_context(
    &self,
    data: &[ComputeData],
    palette: &Palette,
) -> SmoothIterationContext {
    match self {
        Self::SmoothIteration(c) => c.preprocess(data, palette),
    }
}
```

**Step 7: Update ColorizerKind::run_pipeline_with_context**

Replace the method:

```rust
#[allow(clippy::too_many_arguments)]
pub fn run_pipeline_with_context(
    &self,
    data: &[ComputeData],
    context: &SmoothIterationContext,
    palette: &Palette,
    lut: &PaletteLut,
    render_settings: &RenderSettings,
    width: usize,
    height: usize,
    xray_enabled: bool,
) -> Vec<[u8; 4]> {
    match self {
        Self::SmoothIteration(c) => {
            let mut pixels: Vec<[u8; 4]> = data
                .iter()
                .enumerate()
                .map(|(i, d)| c.colorize(d, context, palette, lut, render_settings, i))
                .collect();
            c.postprocess(&mut pixels, data, context, palette, width, height);

            if xray_enabled {
                apply_xray_to_glitched(&mut pixels, data);
            }

            pixels
        }
    }
}
```

**Step 8: Update test**

Replace the test at end of file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::{Palette, PaletteLut, RenderSettings, SmoothIterationColorizer};
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn colorizer_kind_runs_pipeline() {
        use futures::executor::block_on;

        block_on(Palette::factory_defaults()); // ensure loaded
        let palette = block_on(Palette::get("classic")).unwrap();
        let lut = PaletteLut::from_palette(&palette);
        let render_settings = RenderSettings::default();
        let colorizer = ColorizerKind::SmoothIteration(SmoothIterationColorizer);

        let data = vec![
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 500,
                max_iterations: 1000,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 100000.0,
                final_z_re: 0.0,
                final_z_im: 0.0,
                final_derivative_re: 0.0,
                final_derivative_im: 0.0,
            }),
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 0,
                max_iterations: 1000,
                escaped: false,
                glitched: false,
                final_z_norm_sq: 0.0,
                final_z_re: 0.0,
                final_z_im: 0.0,
                final_derivative_re: 0.0,
                final_derivative_im: 0.0,
            }),
        ];

        let pixels = colorizer.run_pipeline(&data, &palette, &lut, &render_settings, 2, 1, false);

        assert_eq!(pixels.len(), 2);
        assert_eq!(pixels[0][3], 255);
        assert_eq!(pixels[1], [0, 0, 0, 255]);
    }
}
```

**Note:** This will not compile yet - SmoothIterationColorizer needs updating first. Continue to Task 3.

---

### Task 3: Update SmoothIterationColorizer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs`

**Step 1: Update imports**

Replace the imports:

```rust
use super::shading::apply_slope_shading;
use super::{Colorizer, Palette, PaletteLut, RenderSettings};
use fractalwonder_core::{ComputeData, MandelbrotData};
```

**Step 2: Update Colorizer impl - preprocess**

Replace the preprocess method:

```rust
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
```

**Step 3: Update Colorizer impl - colorize**

Replace the colorize method:

```rust
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
```

**Step 4: Update Colorizer impl - postprocess**

Replace the postprocess method:

```rust
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
```

**Step 5: Update colorize_with_histogram**

Replace the method:

```rust
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
```

**Step 6: Update colorize_mandelbrot**

Replace the method:

```rust
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
```

**Note:** This will not compile yet - shading needs updating. Continue to Task 4.

---

### Task 4: Update shading.rs

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/shading.rs`

**Step 1: Update imports**

Replace the imports:

```rust
use super::Palette;
use fractalwonder_core::{ComputeData, MandelbrotData};
```

**Step 2: Replace apply_slope_shading signature and implementation**

Replace the entire `apply_slope_shading` function:

```rust
/// Apply derivative-based Blinn-Phong shading to a pixel buffer.
pub fn apply_slope_shading(
    pixels: &mut [[u8; 4]],
    data: &[ComputeData],
    palette: &Palette,
    width: usize,
    height: usize,
) {
    if !palette.shading_enabled {
        return;
    }

    let light = light_direction(palette.lighting.azimuth, palette.lighting.elevation);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;

            if is_interior(&data[idx]) {
                continue;
            }

            let m = match &data[idx] {
                ComputeData::Mandelbrot(m) => m,
                _ => continue,
            };

            let normal = match compute_normal(m) {
                Some(n) => n,
                None => continue,
            };

            let raw_shade = blinn_phong(normal, light, palette);

            // Distance factor using falloff curve
            let normalized_iter = if m.max_iterations > 0 {
                (m.iterations as f64) / (m.max_iterations as f64)
            } else {
                0.0
            };
            // x=0 is near set boundary, x=1 is far from set
            let distance_from_set = 1.0 - normalized_iter;
            let distance_factor = palette.apply_falloff(distance_from_set);

            let shade = 1.0 + (raw_shade - 1.0) * palette.lighting.strength * distance_factor;

            pixels[idx] = apply_shade(pixels[idx], shade);
        }
    }
}
```

**Step 3: Update blinn_phong to use Palette**

Replace the `blinn_phong` function:

```rust
fn blinn_phong(normal: (f64, f64, f64), light: (f64, f64, f64), palette: &Palette) -> f64 {
    let (nx, ny, nz) = normal;
    let (lx, ly, lz) = light;

    let n_dot_l = (nx * lx + ny * ly + nz * lz).max(0.0);

    let vz = 1.0;

    let hx = lx;
    let hy = ly;
    let hz = lz + vz;
    let h_len = (hx * hx + hy * hy + hz * hz).sqrt();
    let (hx, hy, hz) = (hx / h_len, hy / h_len, hz / h_len);

    let n_dot_h = (nx * hx + ny * hy + nz * hz).max(0.0);
    let specular = n_dot_h.powf(palette.lighting.shininess);

    palette.lighting.ambient + palette.lighting.diffuse * n_dot_l + palette.lighting.specular * specular
}
```

**Step 4: Update tests**

Replace the test helper and tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::{LightingParams, Palette};

    fn test_palette() -> Palette {
        Palette {
            shading_enabled: true,
            lighting: LightingParams {
                ambient: 0.15,
                diffuse: 0.7,
                specular: 0.3,
                shininess: 32.0,
                strength: 1.0,
                azimuth: 0.0,
                elevation: std::f64::consts::FRAC_PI_4,
            },
            ..Palette::default()
        }
    }

    #[test]
    fn light_direction_horizontal() {
        let (x, y, z) = light_direction(0.0, 0.0);
        assert!((x - 1.0).abs() < 0.01);
        assert!(y.abs() < 0.01);
        assert!(z.abs() < 0.01);
    }

    #[test]
    fn light_direction_overhead() {
        let (x, y, z) = light_direction(0.0, std::f64::consts::FRAC_PI_2);
        assert!(x.abs() < 0.01);
        assert!(y.abs() < 0.01);
        assert!((z - 1.0).abs() < 0.01);
    }

    #[test]
    fn compute_normal_valid() {
        let m = MandelbrotData {
            iterations: 10,
            max_iterations: 100,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
            final_z_re: 100.0,
            final_z_im: 50.0,
            final_derivative_re: 10.0,
            final_derivative_im: 5.0,
        };
        let normal = compute_normal(&m);
        assert!(normal.is_some());
        let (nx, ny, nz) = normal.unwrap();
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!((len - 1.0).abs() < 0.01);
    }

    #[test]
    fn blinn_phong_facing_light() {
        let normal = (0.0, 0.0, 1.0);
        let light = (0.0, 0.0, 1.0);
        let palette = test_palette();
        let shade = blinn_phong(normal, light, &palette);
        assert!(shade > 0.8, "shade = {}", shade);
    }

    #[test]
    fn blinn_phong_away_from_light() {
        let normal = (0.0, 0.0, 1.0);
        let light = (0.0, 0.0, -1.0);
        let palette = test_palette();
        let shade = blinn_phong(normal, light, &palette);
        assert!(shade < 0.3, "shade = {}", shade);
    }
}
```

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/
git commit -m "refactor(colorizers): update trait signatures to use Palette"
```

---

### Task 5: Update ColorPipeline

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/pipeline.rs`

**Step 1: Update imports**

Replace:

```rust
use super::{ColorizerKind, Palette, PaletteLut, RenderSettings, SmoothIterationContext};
use fractalwonder_core::ComputeData;
```

**Step 2: Replace ColorPipeline struct**

```rust
pub struct ColorPipeline {
    colorizer: ColorizerKind,
    palette: Palette,
    lut: PaletteLut,
    render_settings: RenderSettings,
    cached_context: Option<SmoothIterationContext>,
}
```

**Step 3: Replace ColorPipeline impl**

```rust
impl ColorPipeline {
    pub fn new(palette: Palette, render_settings: RenderSettings) -> Self {
        let lut = PaletteLut::from_palette(&palette);
        Self {
            colorizer: ColorizerKind::default(),
            palette,
            lut,
            render_settings,
            cached_context: None,
        }
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn render_settings(&self) -> &RenderSettings {
        &self.render_settings
    }

    pub fn set_palette(&mut self, palette: Palette) {
        self.lut = PaletteLut::from_palette(&palette);
        self.palette = palette;
    }

    pub fn set_render_settings(&mut self, settings: RenderSettings) {
        self.render_settings = settings;
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_context = None;
    }

    pub fn colorize_chunk(&self, data: &[ComputeData]) -> Vec<[u8; 4]> {
        data.iter()
            .map(|d| {
                if self.render_settings.xray_enabled {
                    if let ComputeData::Mandelbrot(m) = d {
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

                if let Some(ref ctx) = self.cached_context {
                    self.colorizer.colorize_with_cached_histogram(
                        d,
                        ctx,
                        &self.palette,
                        &self.lut,
                        &self.render_settings,
                    )
                } else {
                    self.colorizer.colorize(d, &self.palette, &self.lut, &self.render_settings)
                }
            })
            .collect()
    }

    pub fn colorize_final(
        &mut self,
        data: &[ComputeData],
        width: usize,
        height: usize,
    ) -> Vec<[u8; 4]> {
        let context = self.colorizer.create_context(data, &self.palette);

        let pixels = self.colorizer.run_pipeline_with_context(
            data,
            &context,
            &self.palette,
            &self.lut,
            &self.render_settings,
            width,
            height,
            self.render_settings.xray_enabled,
        );

        self.cached_context = Some(context);

        pixels
    }
}
```

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/pipeline.rs
git commit -m "refactor(pipeline): use Palette + RenderSettings instead of ColorOptions"
```

---

## Phase 2: Cleanup

### Task 6: Update mod.rs exports

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Remove legacy module declarations and imports**

Replace entire file content:

```rust
pub mod color_space;
pub mod colorizer;
pub mod curve;
pub mod gradient;
pub mod lighting_params;
pub mod palette;
pub mod pipeline;
pub mod render_settings;
pub mod shading;
pub mod smooth_iteration;

use fractalwonder_core::ComputeData;

pub use colorizer::{Colorizer, ColorizerKind};
pub use curve::{Curve, CurvePoint};
pub use gradient::{ColorStop, Gradient};
pub use lighting_params::LightingParams;
pub use palette::{Palette, PaletteLut};
pub use pipeline::ColorPipeline;
pub use render_settings::RenderSettings;
pub use shading::apply_slope_shading;
pub use smooth_iteration::{SmoothIterationColorizer, SmoothIterationContext};
```

**Step 2: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "refactor(mod): remove legacy palette exports"
```

---

### Task 7: Delete palette_lut.rs

**Files:**
- Delete: `fractalwonder-ui/src/rendering/colorizers/palette_lut.rs`

**Step 1: Delete the file**

```bash
rm fractalwonder-ui/src/rendering/colorizers/palette_lut.rs
```

**Step 2: Commit**

```bash
git add -A
git commit -m "chore: delete legacy palette_lut.rs"
```

---

### Task 8: Delete settings.rs

**Files:**
- Delete: `fractalwonder-ui/src/rendering/colorizers/settings.rs`

**Step 1: Delete the file**

```bash
rm fractalwonder-ui/src/rendering/colorizers/settings.rs
```

**Step 2: Commit**

```bash
git add -A
git commit -m "chore: delete legacy settings.rs (ColorOptions, ShadingSettings)"
```

---

### Task 9: Verify Phase 2

**Step 1: Run full test suite**

```bash
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: All pass

---

## Phase 3: Callers

### Task 10: Update parallel_renderer.rs

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Update imports**

Replace line 5:

```rust
use crate::rendering::colorizers::{ColorPipeline, Palette, PaletteLut, RenderSettings};
```

**Step 2: Update ParallelRenderer::new**

Replace line 56:

```rust
let pipeline = Rc::new(RefCell::new(ColorPipeline::new(
    Palette::default(),
    RenderSettings::default(),
)));
```

**Step 3: Update set_xray_enabled**

Replace the method:

```rust
pub fn set_xray_enabled(&self, enabled: bool) {
    self.pipeline.borrow_mut().set_render_settings(RenderSettings {
        xray_enabled: enabled,
        ..self.pipeline.borrow().render_settings().clone()
    });
}
```

**Step 4: Remove set_color_options, add set_palette and set_render_settings**

Replace lines 190-193:

```rust
pub fn set_palette(&self, palette: Palette) {
    self.pipeline.borrow_mut().set_palette(palette);
}

pub fn set_render_settings(&self, settings: RenderSettings) {
    self.pipeline.borrow_mut().set_render_settings(settings);
}
```

**Step 5: Update use_gpu check in render method**

Replace line 230:

```rust
let use_gpu = self.config.gpu_enabled && self.pipeline.borrow().render_settings().use_gpu;
```

**Step 6: Update colorize_final calls**

In the render complete callback (around line 114-115), remove zoom_level parameter:

```rust
let final_pixels =
    pipeline.colorize_final(&full_buffer, width as usize, height as usize);
```

And in recolorize (around line 169-170):

```rust
let final_pixels =
    pipeline.colorize_final(&full_buffer, width as usize, height as usize);
```

And in schedule_row_set (around line 583-588):

```rust
let final_pixels = pipeline.colorize_final(
    &full_buffer,
    width as usize,
    height as usize,
);
```

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "refactor(renderer): use Palette + RenderSettings"
```

---

### Task 11: Update persistence.rs

**Files:**
- Modify: `fractalwonder-ui/src/hooks/persistence.rs`

**Step 1: Update imports**

Replace line 8:

```rust
use crate::rendering::colorizers::RenderSettings;
```

**Step 2: Update PersistedState struct**

Replace the struct:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedState {
    pub viewport: Viewport,
    pub config_id: String,
    /// Palette ID to load on startup
    #[serde(default = "default_palette_id")]
    pub palette_id: String,
    /// Render settings (cycle_count, use_gpu, xray)
    #[serde(default)]
    pub render_settings: RenderSettings,
    version: u32,
}

fn default_palette_id() -> String {
    "classic".to_string()
}
```

**Step 3: Update PersistedState impl**

Replace the impl:

```rust
impl PersistedState {
    const CURRENT_VERSION: u32 = 4;

    pub fn new(viewport: Viewport, config_id: String, palette_id: String, render_settings: RenderSettings) -> Self {
        Self {
            viewport,
            config_id,
            palette_id,
            render_settings,
            version: Self::CURRENT_VERSION,
        }
    }

    pub fn with_defaults(viewport: Viewport, config_id: String) -> Self {
        Self::new(viewport, config_id, "classic".to_string(), RenderSettings::default())
    }
}
```

**Step 4: Update version check**

Replace line 71 (and similar at line 183):

```rust
if state.version >= 1 && state.version <= PersistedState::CURRENT_VERSION {
```

**Step 5: Update log message**

Replace line 73-75:

```rust
log::info!(
    "Loaded persisted state from localStorage: config={}, palette={}",
    state.config_id,
    state.palette_id
);
```

**Step 6: Update tests**

Replace the tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_state_roundtrips() {
        let viewport = fractalwonder_core::Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 64);
        let settings = RenderSettings {
            cycle_count: 64,
            use_gpu: false,
            xray_enabled: true,
        };

        let state = PersistedState::new(
            viewport.clone(),
            "mandelbrot".to_string(),
            "fire".to_string(),
            settings.clone(),
        );

        let encoded = encode_state(&state).expect("encoding should succeed");
        let decoded = decode_state(&encoded).expect("decoding should succeed");

        assert_eq!(decoded.palette_id, "fire");
        assert_eq!(decoded.render_settings.cycle_count, 64);
        assert!(!decoded.render_settings.use_gpu);
        assert!(decoded.render_settings.xray_enabled);
    }
}
```

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/hooks/persistence.rs
git commit -m "refactor(persistence): store palette_id + RenderSettings"
```

---

### Task 12: Update interactive_canvas.rs

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

This file likely uses ColorOptions. Search and update all references.

**Step 1: Search for ColorOptions usage**

```bash
grep -n "ColorOptions" fractalwonder-ui/src/components/interactive_canvas.rs
```

Update imports and usages to use `Palette` + `RenderSettings` based on what you find.

**Step 2: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "refactor(canvas): use Palette + RenderSettings"
```

---

### Task 13: Update app.rs and remaining files

**Files:**
- Modify: Any remaining files that reference `ColorOptions`, `palettes()`, or `PaletteEntry`

**Step 1: Search for remaining usages**

```bash
grep -rn "ColorOptions\|palettes()\|PaletteEntry" fractalwonder-ui/src/
```

Update each file found.

**Step 2: Commit**

```bash
git add -A
git commit -m "refactor: complete migration to Palette + RenderSettings"
```

---

### Task 14: Final verification

**Step 1: Run all checks**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features -- --nocapture
```

Expected: All pass

**Step 2: Test in browser**

```bash
trunk serve
```

Open http://localhost:8080 and verify:
- Palette selection works
- Colors render correctly
- 3D shading works
- Smooth/histogram toggles work
- Cycle count adjustments work

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 1 | 1-5 | Core plumbing: PaletteLut, Colorizer trait, shading, pipeline |
| 2 | 6-9 | Cleanup: delete legacy files, update exports |
| 3 | 10-14 | Callers: renderer, persistence, canvas, app |

Each task produces a working, testable commit.
