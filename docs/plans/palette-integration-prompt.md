# Palette Integration Task

## Objective

Integrate the new `Palette` data structure into the rendering pipeline and remove the legacy `palette_lut.rs` code. The goal is to have ONE way of defining and using palettes throughout the codebase.

## Context

We have two parallel palette systems:

1. **New system (`palette.rs`)**: The unified `Palette` struct with gradient, curves, lighting, and flags. Palettes are loaded from `/assets/factory_palettes.json` at runtime.

2. **Legacy system (`palette_lut.rs`)**: The old `PaletteLut` struct that hardcodes palettes as method factories (e.g., `PaletteLut::ultra_fractal()`). Used by `PaletteEntry` and `palettes()` function.

## Files to Understand

Read these files first:

```
fractalwonder-ui/src/rendering/colorizers/
├── palette.rs          # NEW: Unified Palette struct (keep this)
├── palette_lut.rs      # LEGACY: Remove this entirely
├── mod.rs              # Has palettes() function and PaletteEntry (update this)
├── settings.rs         # Has ColorOptions with palette_id (update this)
├── colorizer.rs        # Uses PaletteLut (update this)
├── pipeline.rs         # Uses PaletteLut (update this)
├── smooth_iteration.rs # Uses PaletteLut (update this)
├── gradient.rs         # Gradient::to_lut() already exists (reference)
└── curve.rs            # Curve::evaluate() already exists (reference)
```

Also read:
- `docs/ux-palette-editor/ARCHITECTURE.md` - Section "Rust Implementation Architecture" for the intended design
- `assets/factory_palettes.json` - The factory palette definitions

## Current State

### New Palette struct (palette.rs)
```rust
pub struct Palette {
    pub id: String,
    pub name: String,
    pub gradient: Gradient,           // Has to_lut() method
    pub transfer_curve: Curve,        // Has evaluate() method
    pub histogram_enabled: bool,
    pub smooth_enabled: bool,
    pub shading_enabled: bool,
    pub falloff_curve: Curve,
    pub lighting: LightingParams,
}

impl Palette {
    pub fn to_lut(&self) -> Vec<[u8; 3]>;           // Delegates to gradient.to_lut()
    pub fn apply_transfer(&self, t: f64) -> f64;    // Uses transfer_curve.evaluate()
    pub fn apply_falloff(&self, t: f64) -> f64;     // Uses falloff_curve.evaluate()
    pub async fn factory_defaults() -> Vec<Palette>; // Loads from JSON
    pub async fn get(id: &str) -> Option<Palette>;   // localStorage + factory fallback
    pub fn save(&self) -> Result<(), JsValue>;       // localStorage
    pub fn load(id: &str) -> Option<Palette>;        // localStorage
    pub fn delete(id: &str);                         // localStorage
}
```

### Legacy PaletteLut (palette_lut.rs) - TO BE REMOVED
```rust
pub struct PaletteLut {
    lut: Vec<[u8; 3]>,  // Pre-computed lookup table
}

impl PaletteLut {
    pub fn new(colors: Vec<[u8; 3]>) -> Self;  // Builds LUT from color array
    pub fn sample(&self, t: f64) -> [u8; 3];   // Fast LUT lookup

    // Factory methods (these are now in factory_palettes.json):
    pub fn grayscale() -> Self;
    pub fn ultra_fractal() -> Self;
    pub fn fire() -> Self;
    pub fn ocean() -> Self;
    pub fn electric() -> Self;
    pub fn rainbow() -> Self;
    pub fn neon() -> Self;
    pub fn twilight() -> Self;
    pub fn candy() -> Self;
    pub fn inferno() -> Self;
    pub fn stripey_inferno() -> Self;
    pub fn aurora() -> Self;
}
```

## Integration Tasks

### 1. Create a LUT wrapper for fast sampling

The colorizer needs fast `sample(t) -> [u8; 3]` lookups. Add this to `palette.rs`:

```rust
/// Pre-computed lookup table for fast color sampling.
/// Generated from a Palette's gradient.
pub struct PaletteLut {
    lut: Vec<[u8; 3]>,
}

impl PaletteLut {
    /// Create from a Palette.
    pub fn from_palette(palette: &Palette) -> Self {
        Self { lut: palette.to_lut() }
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

### 2. Update mod.rs

Remove:
- `pub mod palette_lut;`
- `pub use palette_lut::PaletteLut;`
- The `PaletteEntry` struct
- The `palettes()` function

Update:
- Export `PaletteLut` from `palette` module instead
- `colorize_with_palette` function signature stays the same

### 3. Update settings.rs

The `ColorOptions` struct currently has:
```rust
pub palette_id: String,
pub shading_enabled: bool,
pub smooth_enabled: bool,
pub histogram_enabled: bool,
pub cycle_count: u32,
pub transfer_bias: f32,
pub use_gpu: bool,
```

These should be simplified - most settings now come from the Palette itself:
- `palette_id` → Keep (references which Palette to use)
- `shading_enabled` → Move to Palette (already there)
- `smooth_enabled` → Move to Palette (already there)
- `histogram_enabled` → Move to Palette (already there)
- `transfer_bias` → REMOVE (replaced by Palette.transfer_curve)
- `cycle_count` → Keep in RenderSettings (not part of palette)
- `use_gpu` → Keep in RenderSettings (not part of palette)

New simplified ColorOptions:
```rust
pub struct ColorOptions {
    pub palette_id: String,
}

impl ColorOptions {
    pub async fn palette(&self) -> Option<Palette> {
        Palette::get(&self.palette_id).await
    }
}
```

Or consider removing ColorOptions entirely and just passing `Palette` directly.

### 4. Update colorizer.rs

Change function signatures from:
```rust
fn colorize(&self, data: &ComputeData, options: &ColorOptions, palette: &PaletteLut) -> [u8; 4];
```

To:
```rust
fn colorize(&self, data: &ComputeData, palette: &Palette, lut: &PaletteLut) -> [u8; 4];
```

The colorizer now gets settings from the Palette directly:
- `palette.smooth_enabled` instead of `options.smooth_enabled`
- `palette.apply_transfer(t)` instead of `apply_transfer_bias(t, options.transfer_bias)`
- `palette.shading_enabled` and `palette.lighting` for 3D shading

### 5. Update pipeline.rs

`ColorPipeline` currently stores a `PaletteLut`. Update to store both:
```rust
pub struct ColorPipeline {
    palette: Palette,    // Full palette with settings
    lut: PaletteLut,     // Pre-computed LUT for sampling
    colorizer: ColorizerKind,
}

impl ColorPipeline {
    pub fn new(palette: Palette) -> Self {
        let lut = PaletteLut::from_palette(&palette);
        Self {
            palette,
            lut,
            colorizer: ColorizerKind::default(),
        }
    }
}
```

### 6. Update smooth_iteration.rs

Change signatures to use `&Palette` and `&PaletteLut`:
- Use `palette.smooth_enabled` directly
- Use `lut.sample(t)` for color lookups

### 7. Delete palette_lut.rs

After all references are updated, delete `fractalwonder-ui/src/rendering/colorizers/palette_lut.rs`.

### 8. Update ShadingSettings

The `ShadingSettings` struct in settings.rs duplicates what's now in `Palette.lighting`. Either:
- Remove `ShadingSettings` and use `Palette.lighting` directly
- Or have `ShadingSettings` be constructed from `Palette.lighting`

### 9. Update callers

Search for all uses of:
- `palettes()` function
- `PaletteEntry`
- `ColorOptions` (if simplified)

Update to use `Palette::factory_defaults()` and `Palette::get()`.

Key files to check:
- `fractalwonder-ui/src/app.rs` - uses `palettes`
- `fractalwonder-ui/src/components/` - may reference palettes
- `fractalwonder-ui/src/hooks/persistence.rs` - serializes ColorOptions

## Testing

After changes:
1. Run `cargo check --workspace --all-targets --all-features`
2. Run `cargo test --workspace --all-targets --all-features`
3. Run `cargo clippy --all-targets --all-features -- -D warnings`
4. Run `cargo fmt --all`
5. Test in browser with `trunk serve`

## Key Principles

1. **ONE way of doing things** - Remove all legacy palette code
2. **Palette is the source of truth** - All coloring settings come from Palette
3. **PaletteLut is just a cache** - Pre-computed for fast sampling, regenerated when palette changes
4. **Async loading** - `Palette::factory_defaults()` is async, use Leptos resources

## Migration Notes

- The transfer_bias power function is replaced by transfer_curve spline
- Palette flags (smooth_enabled, etc.) now live in the Palette, not ColorOptions
- cycle_count and use_gpu stay in RenderSettings (not part of palette definition)
- The `palettes()` function returns `Vec<PaletteEntry>` which has `PaletteLut` - this entire pattern is removed

## Commits

Make atomic commits:
1. Add `PaletteLut::from_palette()` to palette.rs
2. Update colorizer.rs to use Palette + PaletteLut
3. Update pipeline.rs
4. Update smooth_iteration.rs
5. Simplify ColorOptions or remove it
6. Update mod.rs (remove legacy exports)
7. Delete palette_lut.rs
8. Update callers (app.rs, components, hooks)
9. Clean up any remaining references
