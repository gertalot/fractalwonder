# Color Palettes Design

This document describes the architecture for color palettes and colorizer algorithms in Fractal Wonder.

## Goals

1. Replace grayscale rendering with rich color palettes
2. Support colorizer algorithms with optional pre/post-processing stages (histogram equalization, slope shading)
3. Allow users to select color schemes from a dropdown
4. Design for future extensibility: separate palette and colorizer selection

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        RenderState                              │
│  ┌──────────────┐    ┌──────────────────┐                       │
│  │   Palette    │    │  ColorizerKind   │                       │
│  │ (color map)  │    │   (algorithm)    │                       │
│  └──────────────┘    └──────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Colorizer Pipeline                          │
│  ┌────────────┐    ┌────────────┐    ┌─────────────┐            │
│  │ preprocess │ →  │  colorize  │ →  │ postprocess │            │
│  │ (optional) │    │ (required) │    │ (optional)  │            │
│  └────────────┘    └────────────┘    └─────────────┘            │
│        │                  │                  │                  │
│        ▼                  ▼                  ▼                  │
│    Context           [u8; 4]           mutate pixels            │
└─────────────────────────────────────────────────────────────────┘
```

## Colorizer Trait

Each colorizer algorithm implements this trait:

```rust
pub trait Colorizer {
    /// Data passed from preprocess to colorize/postprocess
    type Context: Default;

    /// Analyze all pixels, build context (e.g., histogram CDF)
    /// Default: no-op, returns Default::default()
    fn preprocess(&self, data: &[ComputeData]) -> Self::Context {
        Self::Context::default()
    }

    /// Map single pixel to color (required)
    fn colorize(
        &self,
        data: &ComputeData,
        context: &Self::Context,
        palette: &Palette,
    ) -> [u8; 4];

    /// Modify pixel buffer in place (e.g., slope shading)
    /// Default: no-op
    fn postprocess(
        &self,
        pixels: &mut [[u8; 4]],
        data: &[ComputeData],
        context: &Self::Context,
        palette: &Palette,
    ) {
    }
}
```

### Stage Data Flow

Each stage receives:
- **preprocess**: All `ComputeData` for the image
- **colorize**: Single `ComputeData`, the `Context` from preprocess, and the `Palette`
- **postprocess**: All pixels, all `ComputeData`, the `Context`, and the `Palette`

### Examples

**Smooth iteration colorizer** (Increment 1):
- `preprocess`: no-op (Context = `()`)
- `colorize`: normalize iteration, sample palette
- `postprocess`: no-op

**Histogram equalization** (future):
- `preprocess`: build histogram CDF from all iteration counts
- `colorize`: use CDF to equalize value, then sample palette
- `postprocess`: no-op

**Slope shading** (future):
- `preprocess`: no-op or extract iteration buffer
- `colorize`: normal palette sampling
- `postprocess`: compute slopes from neighbors, adjust brightness

## Enum Dispatch

Use an enum to avoid trait object complexity with associated types:

```rust
pub enum ColorizerKind {
    SmoothIteration(SmoothIterationColorizer),
    // Future:
    // Histogram(HistogramColorizer),
    // SlopeShaded(SlopeShadedColorizer),
}

impl ColorizerKind {
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
                c.postprocess(&mut pixels, data, &ctx, palette);
                pixels
            }
            // Each arm handles its own Context type
        }
    }
}
```

## Palette

Palettes map normalized values `t ∈ [0,1]` to RGB colors using OKLAB interpolation.

```rust
pub struct Palette {
    /// Control points in sRGB [0-255]
    colors: Vec<[u8; 3]>,
    /// How many times to cycle through the palette
    cycle_count: f64,
}

impl Palette {
    pub fn new(colors: Vec<[u8; 3]>, cycle_count: f64) -> Self {
        Self { colors, cycle_count }
    }

    /// Sample at t ∈ [0,1], interpolating in OKLAB space
    pub fn sample(&self, t: f64) -> [u8; 3] {
        let t = (t * self.cycle_count).fract();
        // OKLAB interpolation between control points
        // ...
    }

    /// Fast sample for progressive rendering (no preprocessing)
    pub fn sample_direct(&self, iterations: u32, max_iterations: u32) -> [u8; 4] {
        let t = iterations as f64 / max_iterations as f64;
        let [r, g, b] = self.sample(t);
        [r, g, b, 255]
    }
}
```

### OKLAB Color Space

OKLAB provides perceptually uniform interpolation. Conversion functions live in `color_space.rs`:

```rust
pub fn srgb_to_linear(c: f64) -> f64;
pub fn linear_to_srgb(c: f64) -> f64;
pub fn linear_rgb_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64);
pub fn oklab_to_linear_rgb(l: f64, a: f64, b: f64) -> (f64, f64, f64);
```

### Predefined Palettes

| Name | Description |
|------|-------------|
| Ultra Fractal | Blue → white → orange → black |
| Fire | Black → red → orange → yellow → white |
| Ocean | Deep blue → cyan → white |
| Grayscale | Black → white |
| Electric | Dark purple → blue → cyan → green → yellow |

## Color Scheme Presets

Presets bundle a palette and colorizer for the UI dropdown:

```rust
pub struct ColorSchemePreset {
    pub name: &'static str,
    pub palette: Palette,
    pub colorizer: ColorizerKind,
}

pub fn presets() -> Vec<ColorSchemePreset> {
    vec![
        ColorSchemePreset {
            name: "Classic",
            palette: Palette::ultra_fractal(),
            colorizer: ColorizerKind::SmoothIteration(SmoothIterationColorizer),
        },
        // ...
    ]
}
```

## Render State

The palette and colorizer are stored separately, allowing independent selection later:

```rust
pub struct RenderState {
    pub palette: Palette,
    pub colorizer: ColorizerKind,
}
```

When user selects a preset, both fields are set. Future UI can have separate dropdowns.

## Rendering Integration

**Progressive rendering (Adam7 passes 1-6):**
- Use fast path: `palette.sample_direct(iterations, max_iterations)`
- Skip pre/post processing for speed

**Final render (pass 7 complete):**
- Run full pipeline: `colorizer.run_pipeline(data, palette, width, height)`
- Display full-quality result

## File Organization

```
fractalwonder-ui/src/rendering/colorizers/
├── mod.rs                 # Re-exports, presets(), ColorizerKind enum
├── color_space.rs         # OKLAB/sRGB conversions
├── palette.rs             # Palette struct, predefined palettes
├── colorizer.rs           # Colorizer trait definition
├── smooth_iteration.rs    # SmoothIterationColorizer
└── test_image.rs          # Unchanged
```

## Future Extensions

- **Increment 2**: Add `final_z_norm_sq` to `MandelbrotData` for true smooth iteration
- **Increment 3**: Add `SlopeShadedColorizer` with postprocess stage
- **Increment 4**: Add `HistogramColorizer` with preprocess stage
- **Increment 5**: Add distance estimation to `MandelbrotData`
- **UI**: Separate dropdowns for palette and colorizer algorithm
- **UI**: Palette editor for custom control points
