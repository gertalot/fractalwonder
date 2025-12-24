# Palette Integration Design

Replace the legacy palette system with the unified `Palette` struct. Delete all backward compatibility code.

## Goal

One way to define and use palettes: the `Palette` struct with gradient, curves, lighting, and flags.

## Current State

Two parallel systems exist:

| Legacy (delete) | New (keep) |
|-----------------|------------|
| `palette_lut.rs` with 12 hardcoded factories | `palette.rs` with unified `Palette` struct |
| `ColorOptions` mixing all settings | `RenderSettings` for runtime-only settings |
| `ShadingSettings` separate struct | `Palette.lighting` + `Palette.falloff_curve` |
| `PaletteEntry` wrapper | Removed; `Palette` has id/name |
| `palettes()` sync function | `Palette::factory_defaults()` async |
| `apply_transfer_bias()` power function | `Palette.transfer_curve` spline |

## Design

### 1. PaletteLut Consolidation

Move `PaletteLut` into `palette.rs` as a cache wrapper. Delete `palette_lut.rs`.

```rust
/// Pre-computed lookup table for fast color sampling.
pub struct PaletteLut {
    lut: Vec<[u8; 3]>,
}

impl PaletteLut {
    pub fn from_palette(palette: &Palette) -> Self {
        Self { lut: palette.to_lut() }
    }

    #[inline]
    pub fn sample(&self, t: f64) -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);
        let index = ((t * 4095.0) as usize).min(4095);
        self.lut[index]
    }
}
```

### 2. Colorizer Trait

Update signatures to receive `&Palette` instead of `&ColorOptions`:

```rust
fn colorize(
    &self,
    data: &ComputeData,
    context: &Self::Context,
    palette: &Palette,
    lut: &PaletteLut,
    render_settings: &RenderSettings,
    index: usize,
) -> [u8; 4];
```

Inside `SmoothIterationColorizer`:

| Old | New |
|-----|-----|
| `options.smooth_enabled` | `palette.smooth_enabled` |
| `options.histogram_enabled` | `palette.histogram_enabled` |
| `apply_transfer_bias(t, options.transfer_bias)` | `palette.apply_transfer(t)` |
| `options.cycle_count` | `render_settings.cycle_count` |

### 3. Shading

Replace `ShadingSettings` with `Palette.lighting` and `Palette.falloff_curve`.

```rust
pub fn apply_slope_shading(
    pixels: &mut [[u8; 4]],
    data: &[ComputeData],
    palette: &Palette,
    width: usize,
    height: usize,
)
```

| Old | New |
|-----|-----|
| `settings.enabled` | `palette.shading_enabled` |
| `settings.light_azimuth` | `palette.lighting.azimuth` |
| `settings.ambient` | `palette.lighting.ambient` |
| `(1.0 - t).powf(settings.distance_falloff)` | `palette.apply_falloff(1.0 - t)` |

**Falloff curve semantics:**
- x = 0: at set boundary
- x = 1: far from set
- y = strength multiplier (0 = no 3D, 1 = full 3D)

A linear identity curve fades 3D to zero at the boundary.

### 4. ColorPipeline

Store `Palette` + `RenderSettings` instead of `ColorOptions`:

```rust
pub struct ColorPipeline {
    colorizer: ColorizerKind,
    palette: Palette,
    lut: PaletteLut,
    render_settings: RenderSettings,
    cached_context: Option<SmoothIterationContext>,
}

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

    pub fn set_palette(&mut self, palette: Palette) {
        self.lut = PaletteLut::from_palette(&palette);
        self.palette = palette;
    }

    pub fn set_render_settings(&mut self, settings: RenderSettings) {
        self.render_settings = settings;
    }
}
```

### 5. Deletions

**Delete files:**
- `palette_lut.rs`
- `settings.rs`

**Delete from `mod.rs`:**
- `PaletteEntry` struct
- `palettes()` function
- `colorize_with_palette()` function

**Update exports:**
```rust
pub use palette::{Palette, PaletteLut};
pub use render_settings::RenderSettings;
```

### 6. Caller Updates

`Palette::factory_defaults()` is async. Use Leptos resources:

```rust
let factory_palettes = create_resource(
    || (),
    |_| async { Palette::factory_defaults().await }
);
```

Update these files:
- `hooks/persistence.rs` - store `palette_id` + `RenderSettings`
- `components/interactive_canvas.rs` - receive `Palette` + `RenderSettings`
- `parallel_renderer.rs` - update pipeline creation

## Implementation Order

### Phase 1: Core (internal changes)

1. Add `PaletteLut::from_palette()` to `palette.rs`
2. Update `Colorizer` trait signature
3. Update `SmoothIterationColorizer`
4. Update `apply_slope_shading`
5. Update `ColorPipeline`

### Phase 2: Cleanup (deletions)

6. Update `mod.rs` exports
7. Delete `palette_lut.rs`
8. Delete `settings.rs`

### Phase 3: Callers (external changes)

9. Update `parallel_renderer.rs`
10. Update `interactive_canvas.rs`
11. Update `persistence.rs`
12. Update app initialization

### Verification

After each phase:

```bash
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```
