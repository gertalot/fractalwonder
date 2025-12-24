# Palette Data Model Design

## Overview

This document defines the Rust data model for the palette editor, which controls fractal coloring through gradient editing, transfer curves, and 3D lighting.

## Design Principles

1. **Unified Palette struct** - One struct holds gradient, curves, lighting, and flags.
2. **Store what code needs** - RGB arrays, not hex strings; radians, not degrees. The UI converts for display.
3. **Cubic interpolating splines** - Store control points only; compute coefficients at evaluation time. The curve passes through each point.
4. **Factory shadowing** - User edits save to localStorage and shadow factory defaults by ID.

## Core Data Structures

### CurvePoint

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f64,  // 0.0-1.0, input
    pub y: f64,  // 0.0-1.0, output
}
```

### Curve

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub points: Vec<CurvePoint>,  // sorted by x, min 2
}

impl Curve {
    pub fn evaluate(&self, x: f64) -> f64;

    pub fn linear() -> Self {
        Self { points: vec![
            CurvePoint { x: 0.0, y: 0.0 },
            CurvePoint { x: 1.0, y: 1.0 },
        ]}
    }
}
```

### ColorStop

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorStop {
    pub position: f64,     // 0.0-1.0
    pub color: [u8; 3],    // RGB
}
```

### Gradient

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub stops: Vec<ColorStop>,    // min 2, sorted by position
    pub midpoints: Vec<f64>,      // one per segment, each 0.0-1.0, default 0.5
}
```

`midpoints[i]` controls the blend center between `stops[i]` and `stops[i+1]`. Values below 0.5 shift the blend toward the first stop; above 0.5, toward the second.

### LightingParams

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightingParams {
    pub ambient: f64,      // 0.0-1.0
    pub diffuse: f64,      // 0.0-1.0
    pub specular: f64,     // 0.0-1.0
    pub shininess: f64,    // 1-128
    pub strength: f64,     // 0.0-2.0
    pub azimuth: f64,      // radians
    pub elevation: f64,    // radians
}
```

### Palette

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub id: String,
    pub name: String,
    pub gradient: Gradient,
    pub transfer_curve: Curve,
    pub histogram_enabled: bool,
    pub smooth_enabled: bool,
    pub shading_enabled: bool,
    pub falloff_curve: Curve,
    pub lighting: LightingParams,
}
```

### RenderSettings

Runtime settings, separate from palette:

```rust
pub struct RenderSettings {
    pub cycle_count: u32,    // default 1, range 1-1024
    pub use_gpu: bool,
    pub xray_enabled: bool,
}
```

## LUT Generation

`Gradient` produces a 4096-entry lookup table using OKLAB interpolation:

```rust
impl Gradient {
    pub fn to_lut(&self) -> Vec<[u8; 3]> {
        // For each position t in [0, 1]:
        // 1. Find the segment containing t
        // 2. Apply midpoint bias
        // 3. Interpolate in OKLAB space
    }
}
```

## Persistence

Palettes persist to localStorage via `web_sys::Storage` and `serde_json`:

```rust
impl Palette {
    pub fn save(&self) -> Result<(), JsValue> {
        let storage = window()?.local_storage().ok()??;
        let json = serde_json::to_string(self)?;
        storage.set_item(&format!("palette:{}", self.id), &json)
    }

    pub fn load(id: &str) -> Option<Self> {
        let storage = window()?.local_storage().ok()??;
        let json = storage.get_item(&format!("palette:{id}")).ok()??;
        serde_json::from_str(&json).ok()
    }

    pub fn delete(id: &str) {
        if let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.remove_item(&format!("palette:{id}"));
        }
    }

    pub fn get(id: &str) -> Option<Self> {
        Self::load(id).or_else(|| {
            Self::factory_defaults().into_iter().find(|p| p.id == id)
        })
    }

    pub fn factory_defaults() -> Vec<Palette>;
}
```

## Factory Shadowing

Factory palettes compile into the binary via `Palette::factory_defaults()`. User edits save to localStorage with key `palette:{id}`. `Palette::get(id)` checks localStorage first, then falls back to factory. Deleting from localStorage resets to factory default.

## Refactoring Map

| Old | New | Notes |
|-----|-----|-------|
| `Palette` (LUT) | `PaletteLut` | Internal |
| `PaletteEntry` | Removed | `Palette` has id/name |
| `ColorOptions.palette_id` | `Palette` | Direct reference |
| `ColorOptions.histogram_enabled` | `Palette.histogram_enabled` | |
| `ColorOptions.smooth_enabled` | `Palette.smooth_enabled` | |
| `ColorOptions.shading_enabled` | `Palette.shading_enabled` | |
| `ColorOptions.transfer_bias` | `Palette.transfer_curve` | Power function becomes spline |
| `ColorOptions.cycle_count` | `RenderSettings.cycle_count` | Separate, default 1 |
| `ColorOptions.use_gpu` | `RenderSettings.use_gpu` | Separate |
| `ShadingSettings` | `Palette.lighting` + `Palette.falloff_curve` | |
| `ShadingSettings.distance_falloff` | `Palette.falloff_curve` | Single value becomes spline |
| `palettes()` | `Palette::factory_defaults()` | |

## Defaults

**Curve** (identity):
```rust
Curve { points: vec![
    CurvePoint { x: 0.0, y: 0.0 },
    CurvePoint { x: 1.0, y: 1.0 },
]}
```

**LightingParams**:
```rust
LightingParams {
    ambient: 0.75,
    diffuse: 0.5,
    specular: 0.9,
    shininess: 64.0,
    strength: 1.5,
    azimuth: -FRAC_PI_2,
    elevation: FRAC_PI_4,
}
```

**RenderSettings**:
```rust
RenderSettings {
    cycle_count: 1,
    use_gpu: true,
    xray_enabled: false,
}
```

**Gradient midpoints**: `vec![0.5; stops.len() - 1]`
