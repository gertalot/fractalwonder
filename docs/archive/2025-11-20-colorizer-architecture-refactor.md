# Colorizer Architecture Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Separate compute and UI concerns by moving colorizer registry to UI and creating minimal renderer factory in compute.

**Architecture:** Split RenderConfig into two parts: compute provides renderer factory for workers, UI owns presentation metadata (display names, info providers, colorizers). Single static COLORIZERS registry in UI organized by renderer ID.

**Tech Stack:** Rust, Leptos, WASM, static arrays for zero-cost abstraction

---

## Task 1: Create Renderer Factory in Compute

**Files:**
- Create: `fractalwonder-compute/src/renderer_factory.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Create renderer factory module**

Create `fractalwonder-compute/src/renderer_factory.rs`:

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

**Step 2: Update compute lib.rs exports**

Modify `fractalwonder-compute/src/lib.rs`:

Find this line:
```rust
pub use render_config::{get_color_scheme, get_config, ColorScheme, RenderConfig, RENDER_CONFIGS};
```

Replace with:
```rust
pub use renderer_factory::create_renderer;
```

Add module declaration at top with other `pub mod` declarations:
```rust
pub mod renderer_factory;
```

**Step 3: Update worker.rs to use new factory**

Modify `fractalwonder-compute/src/worker.rs` line 10:

Change from:
```rust
    crate::render_config::create_renderer(renderer_id)
```

To:
```rust
    crate::renderer_factory::create_renderer(renderer_id)
```

**Step 4: Verify compute builds**

Run: `cargo check -p fractalwonder-compute`
Expected: Success (may have warnings about unused render_config module)

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/renderer_factory.rs fractalwonder-compute/src/lib.rs fractalwonder-compute/src/worker.rs
git commit -m "feat(compute): add renderer factory, update worker

- Create renderer_factory.rs with create_renderer function
- Update worker.rs to use new factory
- Export create_renderer from lib.rs

Part of colorizer architecture refactor"
```

---

## Task 2: Create UI Colorizers Registry

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers.rs` (will replace existing)
- Backup: Save current colorizers.rs content for reference

**Step 1: Read current colorizers.rs for reference**

Run: `cat fractalwonder-ui/src/rendering/colorizers.rs > /tmp/old_colorizers.rs`

This saves current implementation for copying colorizer functions.

**Step 2: Write new colorizers.rs with registry**

Replace entire contents of `fractalwonder-ui/src/rendering/colorizers.rs`:

```rust
use fractalwonder_core::{AppData, MandelbrotData, TestImageData};

pub type Colorizer = fn(&AppData) -> (u8, u8, u8, u8);

#[derive(Clone, Copy)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::MandelbrotData;

    #[test]
    fn test_colorizer_registry_has_all_renderers() {
        assert_eq!(COLORIZERS.len(), 2);
        assert!(COLORIZERS.iter().any(|r| r.renderer_id == "mandelbrot"));
        assert!(COLORIZERS.iter().any(|r| r.renderer_id == "test_image"));
    }

    #[test]
    fn test_each_renderer_has_default_colorizer() {
        for renderer_colorizers in COLORIZERS.iter() {
            let has_default = renderer_colorizers.colorizers.iter().any(|c| c.is_default);
            assert!(
                has_default,
                "Renderer {} missing default colorizer",
                renderer_colorizers.renderer_id
            );
        }
    }

    #[test]
    fn test_colorizer_on_circle() {
        let data = AppData::TestImageData(TestImageData::new(true, 0.05));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (255, 0, 0, 255));
    }

    #[test]
    fn test_colorizer_checkerboard_white() {
        let data = AppData::TestImageData(TestImageData::new(true, 5.0));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (255, 255, 255, 255));
    }

    #[test]
    fn test_colorizer_checkerboard_grey() {
        let data = AppData::TestImageData(TestImageData::new(false, 5.0));
        let color = test_image_default_colorizer(&data);
        assert_eq!(color, (204, 204, 204, 255));
    }

    #[test]
    fn test_mandelbrot_default_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 128,
            escaped: true,
        });
        let (r, g, b, a) = mandelbrot_default_colorizer(&data);
        assert_eq!(a, 255);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn test_mandelbrot_fire_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 64,
            escaped: true,
        });
        let (r, _g, b, a) = mandelbrot_fire_colorizer(&data);
        assert_eq!(a, 255);
        assert!(r > b);
    }

    #[test]
    fn test_mandelbrot_opal_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 64,
            escaped: true,
        });
        let (r, _g, b, a) = mandelbrot_opal_colorizer(&data);
        assert_eq!(a, 255);
        assert!(b > r);
    }

    #[test]
    fn test_mandelbrot_set_interior_is_black() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 256,
            escaped: false,
        });

        let (r, g, b, _) = mandelbrot_default_colorizer(&data);
        assert_eq!((r, g, b), (0, 0, 0));

        let (r, g, b, _) = mandelbrot_fire_colorizer(&data);
        assert_eq!((r, g, b), (0, 0, 0));

        let (r, g, b, _) = mandelbrot_opal_colorizer(&data);
        assert_eq!((r, g, b), (0, 0, 0));
    }
}
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui colorizers`
Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers.rs
git commit -m "feat(ui): create unified colorizer registry

- Single COLORIZERS static with all colorizers
- Organized by renderer_id
- Includes all existing colorizer functions
- Add registry tests

Part of colorizer architecture refactor"
```

---

## Task 3: Create Presentation Config in UI

**Files:**
- Create: `fractalwonder-ui/src/rendering/presentation_config.rs`

**Step 1: Create presentation config module**

Create `fractalwonder-ui/src/rendering/presentation_config.rs`:

```rust
use crate::rendering::colorizers::{Colorizer, RendererColorizers, COLORIZERS};
use fractalwonder_compute::RendererInfo;

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

/// Get renderer config by ID
pub fn get_renderer_config(id: &str) -> Option<&'static RendererPresentationConfig> {
    RENDERER_CONFIGS.iter().find(|c| c.id == id)
}

/// Get colorizers for a specific renderer
pub fn get_colorizers_for_renderer(renderer_id: &str) -> Option<&'static RendererColorizers> {
    COLORIZERS.iter().find(|c| c.renderer_id == renderer_id)
}

/// Get specific colorizer by renderer and colorizer ID
pub fn get_colorizer(renderer_id: &str, colorizer_id: &str) -> Option<Colorizer> {
    let renderer_colorizers = get_colorizers_for_renderer(renderer_id)?;
    renderer_colorizers
        .colorizers
        .iter()
        .find(|c| c.id == colorizer_id)
        .map(|c| c.colorizer)
}

/// Get default colorizer ID for a renderer
pub fn get_default_colorizer_id(renderer_id: &str) -> Option<&'static str> {
    let renderer_colorizers = get_colorizers_for_renderer(renderer_id)?;
    renderer_colorizers
        .colorizers
        .iter()
        .find(|c| c.is_default)
        .map(|c| c.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_renderer_config() {
        assert!(get_renderer_config("mandelbrot").is_some());
        assert!(get_renderer_config("test_image").is_some());
        assert!(get_renderer_config("unknown").is_none());
    }

    #[test]
    fn test_get_colorizers_for_renderer() {
        let mandelbrot_colorizers = get_colorizers_for_renderer("mandelbrot").unwrap();
        assert_eq!(mandelbrot_colorizers.colorizers.len(), 3);

        let test_image_colorizers = get_colorizers_for_renderer("test_image").unwrap();
        assert_eq!(test_image_colorizers.colorizers.len(), 2);
    }

    #[test]
    fn test_get_colorizer() {
        assert!(get_colorizer("mandelbrot", "default").is_some());
        assert!(get_colorizer("mandelbrot", "fire").is_some());
        assert!(get_colorizer("mandelbrot", "opal").is_some());
        assert!(get_colorizer("mandelbrot", "unknown").is_none());
        assert!(get_colorizer("unknown", "default").is_none());
    }

    #[test]
    fn test_get_default_colorizer_id() {
        assert_eq!(get_default_colorizer_id("mandelbrot"), Some("default"));
        assert_eq!(get_default_colorizer_id("test_image"), Some("default"));
        assert_eq!(get_default_colorizer_id("unknown"), None);
    }

    #[test]
    fn test_all_renderer_configs_have_colorizers() {
        for config in RENDERER_CONFIGS.iter() {
            assert!(
                get_colorizers_for_renderer(config.id).is_some(),
                "Renderer {} has no colorizers",
                config.id
            );
        }
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p fractalwonder-ui presentation_config`
Expected: All tests pass

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/presentation_config.rs
git commit -m "feat(ui): add presentation config module

- RENDERER_CONFIGS with display names and info providers
- Helper functions: get_renderer_config, get_colorizer, etc.
- Comprehensive tests

Part of colorizer architecture refactor"
```

---

## Task 4: Update UI Rendering Module Exports

**Files:**
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Update mod.rs exports**

Modify `fractalwonder-ui/src/rendering/mod.rs`:

Replace the entire file with:

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

/// Progress information for ongoing renders
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

**Step 2: Verify builds**

Run: `cargo check -p fractalwonder-ui`
Expected: May have errors in app.rs and other files using old API - that's expected

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat(ui): update rendering module exports

- Export new presentation_config functions
- Export colorizers types
- Remove old render_config exports
- Re-export create_renderer from compute

Part of colorizer architecture refactor"
```

---

## Task 5: Update app.rs to Use New API

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Update imports**

In `fractalwonder-ui/src/app.rs`, find the import section (around line 7-9):

Change from:
```rust
use crate::rendering::{
    get_config, AppData, Colorizer, ParallelCanvasRenderer, Viewport, RENDER_CONFIGS,
};
```

To:
```rust
use crate::rendering::{
    get_colorizer, get_colorizers_for_renderer, get_renderer_config, AppData, Colorizer,
    ParallelCanvasRenderer, Viewport, RENDERER_CONFIGS,
};
```

**Step 2: Update initial config lookup (line ~35)**

Change from:
```rust
let initial_config = get_config(&initial_state.selected_renderer_id).unwrap();
```

To:
```rust
let initial_config = get_renderer_config(&initial_state.selected_renderer_id).unwrap();
```

**Step 3: Update renderer info effect (line ~96)**

Change from:
```rust
let config = get_config(&renderer_id).unwrap();
```

To:
```rust
let config = get_renderer_config(&renderer_id).unwrap();
```

**Step 4: Update initial colorizer lookup (line ~45)**

The call to `crate::rendering::get_colorizer` stays the same (function name unchanged, just moved to presentation_config).

**Step 5: Update renderer selection effect (line ~115-125)**

Find the renderer selection effect that gets config. Change from:
```rust
let config = get_config(&new_renderer_id).unwrap();
```

To:
```rust
let config = get_renderer_config(&new_renderer_id).unwrap();
```

**Step 6: Update menu options (line ~200-215)**

Change from:
```rust
let render_function_options = create_memo(move |_| {
    RENDER_CONFIGS
        .iter()
        .map(|c| (c.id.to_string(), c.display_name.to_string()))
        .collect::<Vec<_>>()
});

let color_scheme_options = create_memo(move |_| {
    let renderer_id = selected_renderer_id.get();
    let config = get_config(&renderer_id).unwrap();
    config
        .color_schemes
        .iter()
        .map(|cs| (cs.id.to_string(), cs.display_name.to_string()))
        .collect::<Vec<_>>()
});
```

To:
```rust
let render_function_options = create_memo(move |_| {
    RENDERER_CONFIGS
        .iter()
        .map(|c| (c.id.to_string(), c.display_name.to_string()))
        .collect::<Vec<_>>()
});

let color_scheme_options = create_memo(move |_| {
    let renderer_id = selected_renderer_id.get();
    let colorizers = get_colorizers_for_renderer(&renderer_id).unwrap();
    colorizers
        .colorizers
        .iter()
        .map(|c| (c.id.to_string(), c.display_name.to_string()))
        .collect::<Vec<_>>()
});
```

**Step 7: Verify builds**

Run: `cargo check -p fractalwonder-ui`
Expected: Success (app_state.rs may still have errors)

**Step 8: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat(ui): update app.rs to use new presentation API

- Use get_renderer_config instead of get_config
- Use get_colorizers_for_renderer for menu options
- Update imports

Part of colorizer architecture refactor"
```

---

## Task 6: Update app_state.rs

**Files:**
- Modify: `fractalwonder-ui/src/state/app_state.rs`

**Step 1: Update imports (line 1)**

Change from:
```rust
use fractalwonder_compute::RENDER_CONFIGS;
```

To:
```rust
use crate::rendering::{get_default_colorizer_id, RENDERER_CONFIGS};
```

**Step 2: Update Default impl (line 45-55)**

Change from:
```rust
for config in RENDER_CONFIGS.iter() {
    // Use default viewport - app will compute natural bounds on first render
    let default_viewport = Viewport::new(fractalwonder_core::Point::new(0.0, 0.0), 1.0);

    renderer_states.insert(
        config.id.to_string(),
        RendererState {
            viewport: default_viewport,
            color_scheme_id: config.default_color_scheme_id.to_string(),
        },
    );
}
```

To:
```rust
for config in RENDERER_CONFIGS.iter() {
    // Use default viewport - app will compute natural bounds on first render
    let default_viewport = Viewport::new(fractalwonder_core::Point::new(0.0, 0.0), 1.0);

    let default_colorizer_id = get_default_colorizer_id(config.id)
        .expect("All renderers must have a default colorizer");

    renderer_states.insert(
        config.id.to_string(),
        RendererState {
            viewport: default_viewport,
            color_scheme_id: default_colorizer_id.to_string(),
        },
    );
}
```

**Step 3: Verify builds**

Run: `cargo check -p fractalwonder-ui`
Expected: Success (parallel_canvas_renderer.rs may still have error)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/state/app_state.rs
git commit -m "feat(ui): update app_state to use new API

- Import from rendering module
- Use get_default_colorizer_id
- Use RENDERER_CONFIGS

Part of colorizer architecture refactor"
```

---

## Task 7: Update parallel_canvas_renderer.rs

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs`

**Step 1: Update renderer creation (line ~79-81)**

Find this code:
```rust
let config = fractalwonder_compute::get_config(&renderer_id)
    .ok_or_else(|| JsValue::from_str(&format!("Unknown renderer: {}", renderer_id)))?;
let renderer = (config.create_renderer)();
```

Replace with:
```rust
let renderer = fractalwonder_compute::create_renderer(&renderer_id)
    .ok_or_else(|| JsValue::from_str(&format!("Unknown renderer: {}", renderer_id)))?;
```

**Step 2: Verify builds**

Run: `cargo check -p fractalwonder-ui`
Expected: Success - all compile errors resolved!

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_canvas_renderer.rs
git commit -m "feat(ui): update parallel_canvas_renderer to use factory

- Use create_renderer directly from compute
- Remove dependency on RenderConfig

Part of colorizer architecture refactor"
```

---

## Task 8: Delete Old render_config.rs from Compute

**Files:**
- Delete: `fractalwonder-compute/src/render_config.rs`
- Modify: `fractalwonder-compute/src/lib.rs`

**Step 1: Remove render_config module declaration**

In `fractalwonder-compute/src/lib.rs`, find and delete this line:
```rust
pub mod render_config;
```

**Step 2: Delete the file**

Run: `rm fractalwonder-compute/src/render_config.rs`

**Step 3: Verify everything builds**

Run: `cargo check --workspace`
Expected: Success - no errors, no warnings about unused module

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/render_config.rs fractalwonder-compute/src/lib.rs
git commit -m "refactor: remove old render_config from compute

- Delete render_config.rs entirely
- Remove module declaration from lib.rs
- Clean separation: compute has no UI concerns

Part of colorizer architecture refactor"
```

---

## Task 9: Run Full Test Suite

**Files:**
- None (verification only)

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: No warnings or errors

**Step 3: Format code**

Run: `cargo fmt --all`

**Step 4: Build release**

Run: `cargo build --workspace --release`
Expected: Success

**Step 5: Commit if formatting changed**

```bash
git add -A
git commit -m "style: format code after refactor"
```

---

## Task 10: Manual Testing

**Files:**
- None (manual verification)

**Step 1: Start dev server**

Run: `trunk serve`
Expected: Server starts, app loads at http://localhost:8080

**Step 2: Test renderer switching**

1. Open browser to http://localhost:8080
2. Click renderer dropdown
3. Verify both "Test Image" and "Mandelbrot" appear
4. Switch between renderers
5. Expected: Renders correctly, viewport state persists per renderer

**Step 3: Test colorizer switching**

1. Select "Mandelbrot" renderer
2. Click color scheme dropdown
3. Verify "Default", "Fire", "Opal" appear
4. Switch between color schemes
5. Expected: Colors change immediately

**Step 4: Test state persistence**

1. Change renderer to "Mandelbrot"
2. Change color scheme to "Fire"
3. Zoom in/pan around
4. Refresh page
5. Expected: Renderer, color scheme, and viewport all restore correctly

**Step 5: Test adding new colorizer**

Edit `fractalwonder-ui/src/rendering/colorizers.rs`:

Add to mandelbrot colorizers array:
```rust
ColorizerInfo {
    id: "grayscale_inverted",
    display_name: "Inverted",
    is_default: false,
    colorizer: mandelbrot_inverted_colorizer,
},
```

Add function:
```rust
fn mandelbrot_inverted_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (255, 255, 255, 255);
            }
            let normalized = (d.iterations as f64 / 256.0).min(1.0);
            let intensity = ((1.0 - normalized) * 255.0) as u8;
            (intensity, intensity, intensity, 255)
        }
        _ => (0, 0, 0, 255),
    }
}
```

Reload browser. Expected: "Inverted" appears in dropdown, works when selected.

**Step 6: Revert test changes**

Run: `git checkout fractalwonder-ui/src/rendering/colorizers.rs`

---

## Task 11: Final Verification and Documentation

**Files:**
- Update: `docs/plans/2025-11-20-colorizer-architecture-refactor-design.md`

**Step 1: Update design doc status**

In the design doc, change:
```
**Status:** Design - Ready for Implementation
```

To:
```
**Status:** Implemented ✅
```

**Step 2: Run final full build**

Run: `cargo build --workspace --all-targets --all-features`
Expected: Success

**Step 3: Run WASM tests**

Run: `wasm-pack test --headless --chrome`
Expected: All tests pass

**Step 4: Commit status update**

```bash
git add docs/plans/2025-11-20-colorizer-architecture-refactor-design.md
git commit -m "docs: mark colorizer refactor as implemented"
```

**Step 5: Create summary commit**

```bash
git log --oneline HEAD~11..HEAD > /tmp/refactor_commits.txt
git commit --allow-empty -m "refactor: colorizer architecture complete

Summary of changes:
- Separated compute and UI concerns
- Compute: renderer_factory with create_renderer
- UI: presentation_config + unified COLORIZERS registry
- All tests passing, manual testing verified

Benefits:
- Adding colorizers: edit one file (colorizers.rs)
- Clean separation: compute has no UI concerns
- Type-safe, zero runtime overhead
- Single source of truth for colorizers

See design: docs/plans/2025-11-20-colorizer-architecture-refactor-design.md

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Completion Checklist

- [ ] Renderer factory created in compute
- [ ] Unified colorizer registry in UI
- [ ] Presentation config created
- [ ] All UI code migrated to new API
- [ ] Old render_config deleted from compute
- [ ] All tests passing
- [ ] Clippy clean
- [ ] Manual testing verified
- [ ] Documentation updated
- [ ] Clean git history with atomic commits

## Success Criteria

1. ✅ `cargo test --workspace` - all pass
2. ✅ `cargo clippy --workspace` - no warnings
3. ✅ `trunk serve` - app loads and functions
4. ✅ Can switch renderers and colorizers
5. ✅ State persists across refresh
6. ✅ Adding new colorizer only touches `colorizers.rs`
7. ✅ Compute crate has no UI concerns
8. ✅ All commits are atomic and well-described
