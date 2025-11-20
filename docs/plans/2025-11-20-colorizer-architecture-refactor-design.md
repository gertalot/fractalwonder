# Colorizer Architecture Refactor

**Date:** 2025-11-20
**Status:** Design - Ready for Implementation

## Problem Statement

The current architecture violates separation of concerns:

1. **RenderConfig in `fractalwonder-compute` contains colorizer metadata** (`ColorScheme`, `color_schemes` field, `default_color_scheme_id`)
   - Violates the principle: compute should purely compute data, UI should handle presentation
   - Colorizing only happens in UI, yet compute knows about color schemes

2. **Adding a colorizer requires touching 4 locations:**
   - Add `ColorScheme` entry to `RENDER_CONFIGS` in `fractalwonder-compute/src/render_config.rs`
   - Write colorizer function in `fractalwonder-ui/src/rendering/colorizers.rs`
   - Export it in `fractalwonder-ui/src/rendering/mod.rs`
   - Add match arm to `get_colorizer()` function
   - Error-prone, not DRY, no compile-time safety

3. **Hardcoded string-based dispatch:**
   - `get_colorizer(renderer_id, scheme_id)` uses pattern matching
   - No type safety - can request non-existent scheme, get `None` at runtime
   - Doesn't scale well

4. **No discoverability:**
   - Colorizers are loose functions
   - No programmatic way to discover what colorizers exist for a renderer
   - Manual maintenance of mappings

## Design Goals

1. **Clean separation of concerns:**
   - Compute layer: pure computation, no presentation metadata
   - UI layer: all presentation logic (display names, colorizers, etc.)

2. **Single source of truth:**
   - One data structure containing all colorizers
   - Easy to iterate for building UI menus

3. **Minimal touch points:**
   - Adding a colorizer should touch ONE place

4. **Idiomatic Rust:**
   - Compile-time registration using static arrays (zero-cost abstraction)
   - No runtime overhead, no macros, no complexity
   - Follow existing patterns (like current `RENDER_CONFIGS`)

5. **Type safety where practical:**
   - Keep `Colorizer<AppData>` with enum matching (Rust's idiomatic approach)
   - Single match per pixel (already happening, no additional cost)

## Architecture

### Compute Layer (`fractalwonder-compute`)

**NEW: `src/renderer_factory.rs`**

```rust
use crate::{AdaptiveMandelbrotRenderer, AppDataRenderer, PixelRenderer, Renderer, TestImageComputer};
use fractalwonder_core::{AppData, BigFloat};

/// Create a renderer by ID for use by workers
pub fn create_renderer(
    renderer_id: &str,
) -> Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>> {
    match renderer_id {
        "mandelbrot" => Some(Box::new(AdaptiveMandelbrotRenderer::new(1e10))),
        "test_image" => {
            let computer = TestImageComputer::<BigFloat>::new();
            let pixel_renderer = PixelRenderer::new(computer);
            let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
            Some(Box::new(app_renderer))
        }
        _ => None,
    }
}
```

**CHANGE: `src/lib.rs`**

```rust
pub mod renderer_factory;

// DELETE these exports:
// pub use render_config::{get_color_scheme, get_config, ColorScheme, RenderConfig, RENDER_CONFIGS};

// ADD:
pub use renderer_factory::create_renderer;
```

**DELETE:** `src/render_config.rs` (entire file)

**Reasoning:**
- Workers only need `create_renderer(id: &str)` to instantiate renderers
- No color schemes, display names, or UI metadata in compute
- Clean, focused responsibility

### UI Layer (`fractalwonder-ui`)

**NEW: `src/rendering/colorizers.rs`** (replaces existing)

```rust
use fractalwonder_core::{AppData, MandelbrotData, TestImageData};

pub type Colorizer = fn(&AppData) -> (u8, u8, u8, u8);

pub struct ColorizerInfo {
    pub id: &'static str,
    pub display_name: &'static str,
    pub is_default: bool,
    pub colorizer: Colorizer,
}

pub struct RendererColorizers {
    pub renderer_id: &'static str,
    pub colorizers: &'static [ColorizerInfo],
}

/// Single static registry - all colorizers organized by renderer
pub static COLORIZERS: &[RendererColorizers] = &[
    RendererColorizers {
        renderer_id: "mandelbrot",
        colorizers: &[
            ColorizerInfo {
                id: "default",
                display_name: "Default",
                is_default: true,
                colorizer: mandelbrot_default_colorizer,
            },
            ColorizerInfo {
                id: "fire",
                display_name: "Fire",
                is_default: false,
                colorizer: mandelbrot_fire_colorizer,
            },
            ColorizerInfo {
                id: "opal",
                display_name: "Opal",
                is_default: false,
                colorizer: mandelbrot_opal_colorizer,
            },
        ],
    },
    RendererColorizers {
        renderer_id: "test_image",
        colorizers: &[
            ColorizerInfo {
                id: "default",
                display_name: "Default",
                is_default: true,
                colorizer: test_image_default_colorizer,
            },
            ColorizerInfo {
                id: "pastel",
                display_name: "Pastel",
                is_default: false,
                colorizer: test_image_pastel_colorizer,
            },
        ],
    },
];

// === Mandelbrot Colorizers ===

fn mandelbrot_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255);
            }
            let normalized = (d.iterations as f64 / 256.0).min(1.0);
            let intensity = (normalized * 255.0) as u8;
            (intensity, intensity, intensity, 255)
        }
        _ => (0, 0, 0, 255),
    }
}

fn mandelbrot_fire_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255);
            }
            let t = (d.iterations as f64 / 256.0).min(1.0);
            let r = (t * 255.0) as u8;
            let g = if t > 0.5 {
                ((t - 0.5) * 2.0 * 255.0) as u8
            } else {
                0
            };
            let b = if t > 0.8 {
                ((t - 0.8) * 5.0 * 255.0) as u8
            } else {
                0
            };
            (r, g, b, 255)
        }
        _ => (0, 0, 0, 255),
    }
}

fn mandelbrot_opal_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255);
            }
            let t = (d.iterations as f64 / 256.0).min(1.0);
            let r = if t > 0.6 {
                ((t - 0.6) * 2.5 * 255.0) as u8
            } else {
                0
            };
            let g = if t > 0.4 {
                ((t - 0.4) * 1.67 * 255.0) as u8
            } else {
                0
            };
            let b = (t * 255.0) as u8;
            (r, g, b, 255)
        }
        _ => (0, 0, 0, 255),
    }
}

// === Test Image Colorizers ===

fn test_image_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => test_image_data_to_rgba(d),
        _ => (0, 0, 0, 255),
    }
}

fn test_image_data_to_rgba(data: &TestImageData) -> (u8, u8, u8, u8) {
    if data.circle_distance < 0.1 {
        return (255, 0, 0, 255);
    }
    if data.checkerboard {
        (255, 255, 255, 255)
    } else {
        (204, 204, 204, 255)
    }
}

fn test_image_pastel_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => {
            let base_hue = if d.checkerboard { 200.0 } else { 50.0 };
            let lightness = 0.7 + (d.circle_distance.sin() * 0.2);
            let saturation = 0.4;

            let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
            let h_prime = base_hue / 60.0;
            let h_mod: f64 = h_prime % 2.0;
            let x = c * (1.0 - (h_mod - 1.0).abs());

            let (r1, g1, b1) = match h_prime as i32 {
                0..=1 => (c, x, 0.0),
                2..=3 => (0.0, c, x),
                4..=5 => (x, 0.0, c),
                _ => (c, x, 0.0),
            };

            let m = lightness - c / 2.0;
            let r = ((r1 + m) * 255.0) as u8;
            let g = ((g1 + m) * 255.0) as u8;
            let b = ((b1 + m) * 255.0) as u8;

            (r, g, b, 255)
        }
        _ => (0, 0, 0, 255),
    }
}
```

**NEW: `src/rendering/presentation_config.rs`**

```rust
use crate::rendering::colorizers::{Colorizer, RendererColorizers, COLORIZERS};
use fractalwonder_compute::RendererInfo;
use fractalwonder_core::AppData;

pub struct RendererPresentationConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Scalar = f64>>,
}

pub static RENDERER_CONFIGS: &[RendererPresentationConfig] = &[
    RendererPresentationConfig {
        id: "test_image",
        display_name: "Test Image",
        create_info_provider: || Box::new(fractalwonder_compute::TestImageComputer::<f64>::new()),
    },
    RendererPresentationConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot",
        create_info_provider: || Box::new(fractalwonder_compute::MandelbrotComputer::<f64>::new()),
    },
];

pub fn get_renderer_config(id: &str) -> Option<&'static RendererPresentationConfig> {
    RENDERER_CONFIGS.iter().find(|c| c.id == id)
}

pub fn get_colorizers_for_renderer(renderer_id: &str) -> Option<&'static RendererColorizers> {
    COLORIZERS.iter().find(|c| c.renderer_id == renderer_id)
}

pub fn get_colorizer(renderer_id: &str, colorizer_id: &str) -> Option<Colorizer> {
    let renderer_colorizers = get_colorizers_for_renderer(renderer_id)?;
    renderer_colorizers
        .colorizers
        .iter()
        .find(|c| c.id == colorizer_id)
        .map(|c| c.colorizer)
}

pub fn get_default_colorizer_id(renderer_id: &str) -> Option<&'static str> {
    let renderer_colorizers = get_colorizers_for_renderer(renderer_id)?;
    renderer_colorizers
        .colorizers
        .iter()
        .find(|c| c.is_default)
        .map(|c| c.id)
}
```

**CHANGE: `src/rendering/mod.rs`**

```rust
pub mod canvas_renderer;
pub mod colorizers;
pub mod parallel_canvas_renderer;
pub mod presentation_config;

pub use canvas_renderer::CanvasRenderer;
pub use colorizers::{Colorizer, ColorizerInfo, RendererColorizers, COLORIZERS};
pub use parallel_canvas_renderer::ParallelCanvasRenderer;
pub use presentation_config::{
    get_colorizer, get_colorizers_for_renderer, get_default_colorizer_id, get_renderer_config,
    RendererPresentationConfig, RENDERER_CONFIGS,
};

// Re-export compute types
pub use fractalwonder_compute::{
    create_renderer, AdaptiveMandelbrotRenderer, AppDataRenderer, PixelRenderer,
    PrecisionCalculator, Renderer, RendererInfo, TestImageComputer,
};
pub use fractalwonder_core::{
    apply_pixel_transform_to_viewport, AppData, BigFloat, Point, Rect, ToF64, Viewport,
};

// Progress tracking (unchanged)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderProgress {
    pub completed_tiles: u32,
    pub total_tiles: u32,
    pub render_id: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}

impl RenderProgress {
    pub fn new(total_tiles: u32, render_id: u32) -> Self {
        Self {
            completed_tiles: 0,
            total_tiles,
            render_id,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }

    pub fn percentage(&self) -> f32 {
        if self.total_tiles == 0 {
            0.0
        } else {
            (self.completed_tiles as f32 / self.total_tiles as f32) * 100.0
        }
    }
}

impl Default for RenderProgress {
    fn default() -> Self {
        Self {
            completed_tiles: 0,
            total_tiles: 0,
            render_id: 0,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }
}
```

## Migration Guide

### UI Code Changes

**Before:**
```rust
use crate::rendering::{get_config, RENDER_CONFIGS};

let config = get_config(&renderer_id).unwrap();
let color_schemes = config.color_schemes;
let default_color_scheme_id = config.default_color_scheme_id;
```

**After:**
```rust
use crate::rendering::{
    get_renderer_config, get_colorizers_for_renderer, get_default_colorizer_id,
    RENDERER_CONFIGS, COLORIZERS
};

let config = get_renderer_config(&renderer_id).unwrap();
let colorizers = get_colorizers_for_renderer(&renderer_id).unwrap();
let default_colorizer_id = get_default_colorizer_id(&renderer_id).unwrap();
```

### Building UI Menus

**Renderer dropdown:**
```rust
let renderer_options = RENDERER_CONFIGS
    .iter()
    .map(|c| (c.id.to_string(), c.display_name.to_string()))
    .collect::<Vec<_>>();
```

**Color scheme dropdown:**
```rust
let renderer_id = selected_renderer_id.get();
let colorizers = get_colorizers_for_renderer(&renderer_id).unwrap();
let color_options = colorizers
    .colorizers
    .iter()
    .map(|c| (c.id.to_string(), c.display_name.to_string()))
    .collect::<Vec<_>>();
```

### App State Initialization

**Before:**
```rust
for config in RENDER_CONFIGS.iter() {
    renderer_states.insert(
        config.id.to_string(),
        RendererState {
            viewport: default_viewport,
            color_scheme_id: config.default_color_scheme_id.to_string(),
        },
    );
}
```

**After:**
```rust
for config in RENDERER_CONFIGS.iter() {
    let default_colorizer_id = get_default_colorizer_id(config.id).unwrap();
    renderer_states.insert(
        config.id.to_string(),
        RendererState {
            viewport: default_viewport,
            color_scheme_id: default_colorizer_id.to_string(),
        },
    );
}
```

### Worker Code

**No changes required** - workers continue to use:
```rust
fractalwonder_compute::create_renderer(renderer_id)
```

## Adding New Colorizers

**Example: Adding "rainbow" colorizer to mandelbrot**

Edit `fractalwonder-ui/src/rendering/colorizers.rs`:

```rust
pub static COLORIZERS: &[RendererColorizers] = &[
    RendererColorizers {
        renderer_id: "mandelbrot",
        colorizers: &[
            ColorizerInfo {
                id: "default",
                display_name: "Default",
                is_default: true,
                colorizer: mandelbrot_default_colorizer,
            },
            ColorizerInfo {
                id: "fire",
                display_name: "Fire",
                is_default: false,
                colorizer: mandelbrot_fire_colorizer,
            },
            ColorizerInfo {
                id: "opal",
                display_name: "Opal",
                is_default: false,
                colorizer: mandelbrot_opal_colorizer,
            },
            // 👇 ADD THIS
            ColorizerInfo {
                id: "rainbow",
                display_name: "Rainbow",
                is_default: false,
                colorizer: mandelbrot_rainbow_colorizer,
            },
        ],
    },
    // ... test_image unchanged
];

// Add implementation
fn mandelbrot_rainbow_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255);
            }
            // Rainbow implementation
            let hue = (d.iterations as f64 / 256.0) * 360.0;
            // ... HSL to RGB conversion
            (r, g, b, 255)
        }
        _ => (0, 0, 0, 255),
    }
}
```

**That's it!** The colorizer automatically:
- Appears in UI dropdown menu
- Can be selected and applied
- Is type-checked at compile time

## Benefits

1. **Clean separation:** Compute knows nothing about presentation
2. **Single source of truth:** `COLORIZERS` static contains all colorizers
3. **One file to edit:** Adding colorizers touches only `colorizers.rs`
4. **Zero runtime cost:** Static arrays, compile-time registration
5. **Type safe:** Compiler enforces correct colorizer signatures
6. **Idiomatic Rust:** Follows existing patterns (RENDER_CONFIGS style)
7. **Easy to discover:** Iterate `COLORIZERS` to build UI menus

## Testing Strategy

1. **Unit tests for colorizers** (move existing tests to `colorizers.rs`)
2. **Integration test:** Verify all registered colorizers work
3. **UI test:** Verify dropdowns populate correctly
4. **Worker test:** Verify `create_renderer()` still works
5. **State persistence test:** Verify app state load/save with new structure

## Implementation Steps

1. Create `fractalwonder-compute/src/renderer_factory.rs`
2. Update `fractalwonder-compute/src/lib.rs` exports
3. Delete `fractalwonder-compute/src/render_config.rs`
4. Create `fractalwonder-ui/src/rendering/presentation_config.rs`
5. Refactor `fractalwonder-ui/src/rendering/colorizers.rs` to new structure
6. Update `fractalwonder-ui/src/rendering/mod.rs` exports
7. Update all UI code to use new API (`app.rs`, `app_state.rs`, `components/ui.rs`)
8. Update worker initialization (if needed)
9. Move/update tests
10. Verify build, run tests, manual testing
