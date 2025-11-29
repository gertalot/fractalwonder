# Slope Shading Design

## Overview

Add slope shading (3D lighting effect) to the colorization pipeline. Treats smooth iteration counts as a height field and computes surface normals to apply Lambert-style lighting.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Store smooth iterations | In `Context` as `Vec<f64>` | Computed once in preprocess, used by colorize and postprocess |
| Settings structure | Separate `ColorSettings` struct | Decouples algorithm from configuration; enables future settings dialog |
| Gradient algorithm | 8-neighbor | Per research doc recommendation |
| Edge handling | Mirror/reflect | Clean edges everywhere, no border artifacts |
| Blend method | Configurable lerp | `final = lerp(base, shaded, blend)`; user controls intensity |
| Interior pixels | No shading | Interior stays pure black; no meaningful iteration height |
| Height factor scaling | Base + auto-scale with zoom | `effective = base * (1.0 + zoom.log10() / 10.0)` |
| Glitch handling | Ignore | Shading applies uniformly to all escaped pixels |

## Settings Structures

```rust
/// All settings that affect colorization (not compute).
pub struct ColorSettings {
    pub palette: Palette,
    pub cycle_count: f64,
    pub shading: ShadingSettings,
}

pub struct ShadingSettings {
    pub enabled: bool,
    pub light_angle: f64,     // Radians, 0 = right, π/2 = top
    pub height_factor: f64,   // Base factor, auto-scaled by zoom
    pub blend: f64,           // 0.0 = off, 1.0 = full effect
}

impl Default for ShadingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            light_angle: std::f64::consts::FRAC_PI_4, // Top-right (45°)
            height_factor: 1.5,
            blend: 0.7,
        }
    }
}
```

## Pipeline Changes

### Colorizer Trait

```rust
pub trait Colorizer {
    type Context: Default;

    fn preprocess(&self, data: &[ComputeData]) -> Self::Context;

    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        settings: &ColorSettings,
        index: usize,  // For accessing context by position
    ) -> [u8; 4];

    fn postprocess(
        &self,
        pixels: &mut [[u8; 4]],
        data: &[ComputeData],
        context: &Self::Context,
        settings: &ColorSettings,
        width: usize,
        height: usize,
        zoom_level: f64,
    );
}
```

### SmoothIterationColorizer Changes

```rust
impl Colorizer for SmoothIterationColorizer {
    type Context = Vec<f64>;  // Smooth iteration per pixel

    fn preprocess(&self, data: &[ComputeData]) -> Self::Context {
        data.iter().map(|d| compute_smooth_iteration(d)).collect()
    }

    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        settings: &ColorSettings,
        index: usize,
    ) -> [u8; 4] {
        // Use precomputed smooth value from context
        let smooth = context[index];
        let t = apply_cycle_count(smooth, settings.cycle_count);
        let [r, g, b] = settings.palette.sample(t);
        [r, g, b, 255]
    }

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
        if !settings.shading.enabled {
            return;
        }
        apply_slope_shading(pixels, data, context, settings, width, height, zoom_level);
    }
}
```

## Slope Shading Algorithm

### 8-Neighbor Gradient with Mirror Edge Handling

```rust
fn apply_slope_shading(
    pixels: &mut [[u8; 4]],
    data: &[ComputeData],
    smooth_iters: &[f64],
    settings: &ColorSettings,
    width: usize,
    height: usize,
    zoom_level: f64,
) {
    let shading = &settings.shading;
    let effective_height = shading.height_factor * (1.0 + zoom_level.log10().max(0.0) / 10.0);

    let light_x = shading.light_angle.cos();
    let light_y = shading.light_angle.sin();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;

            // Skip interior pixels
            if is_interior(&data[idx]) {
                continue;
            }

            let shade = compute_shade_8neighbor(
                smooth_iters, width, height, x, y,
                light_x, light_y, effective_height,
            );

            // Blend: final = lerp(base, shaded, blend)
            pixels[idx] = blend_shade(pixels[idx], shade, shading.blend);
        }
    }
}

fn compute_shade_8neighbor(
    iters: &[f64],
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
        iters[ny * width + nx]
    };

    let center = get(0, 0);

    // 8-neighbor gradient accumulation (per research doc)
    let mut running_sum = 0.0;
    let mut high = center;
    let mut low = center;

    let neighbors = [
        (-1, -1), (0, -1), (1, -1),
        (-1,  0),          (1,  0),
        (-1,  1), (0,  1), (1,  1),
    ];

    for (dx, dy) in neighbors {
        let neighbor_val = get(dx, dy);
        high = high.max(neighbor_val);
        low = low.min(neighbor_val);

        let diff = neighbor_val - center;

        // Apply direction based on light position
        let h_diff = if dx < 0 { -diff } else { diff };
        let v_diff = if dy > 0 { -diff } else { diff };

        if dx != 0 { running_sum += h_diff * light_x.abs(); }
        if dy != 0 { running_sum += v_diff * light_y.abs(); }
    }

    // Normalize by range
    let range = high - low;
    let slope = if range > 0.0 {
        (running_sum * height_factor) / range
    } else {
        0.0
    };

    // Map to [0, 1] range
    ((slope / (1.0 + slope.abs())) + 1.0) / 2.0
}

fn mirror_coord(coord: i32, max: usize) -> usize {
    if coord < 0 {
        (-coord) as usize
    } else if coord >= max as i32 {
        (2 * max as i32 - coord - 2) as usize
    } else {
        coord as usize
    }
}

fn blend_shade(base: [u8; 4], shade: f64, blend: f64) -> [u8; 4] {
    // shade 0.5 = neutral, <0.5 = darken, >0.5 = lighten
    let factor = 0.3 + shade * 1.4;  // [0.3, 1.7]

    let apply = |c: u8| -> u8 {
        let shaded = (c as f64 * factor).clamp(0.0, 255.0);
        let blended = c as f64 + blend * (shaded - c as f64);
        blended.clamp(0.0, 255.0) as u8
    };

    [apply(base[0]), apply(base[1]), apply(base[2]), base[3]]
}
```

## File Changes

| File | Change |
|------|--------|
| `colorizers/mod.rs` | Add `ColorSettings`, `ShadingSettings`, re-exports |
| `colorizers/colorizer.rs` | Update trait signatures, pipeline to use `ColorSettings` + zoom |
| `colorizers/smooth_iteration.rs` | Change `Context` to `Vec<f64>`, implement `postprocess` |
| `colorizers/shading.rs` (new) | `apply_slope_shading`, gradient computation, blending |
| `parallel_renderer.rs` | Pass `ColorSettings` and zoom level to pipeline |
| `presets.rs` | Update presets to return `ColorSettings` |

## Default Presets

```rust
impl ColorSettings {
    pub fn ultra_fractal() -> Self {
        Self {
            palette: Palette::ultra_fractal(),
            cycle_count: 1.0,
            shading: ShadingSettings::default(),
        }
    }

    pub fn ultra_fractal_shaded() -> Self {
        Self {
            palette: Palette::ultra_fractal(),
            cycle_count: 1.0,
            shading: ShadingSettings {
                enabled: true,
                light_angle: std::f64::consts::FRAC_PI_4,
                height_factor: 1.5,
                blend: 0.7,
            },
        }
    }
}
```
