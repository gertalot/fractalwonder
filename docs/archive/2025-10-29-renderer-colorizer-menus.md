# Renderer and Colorizer Selection Menus Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add UI menus for selecting render functions and color schemes with stateful per-renderer configuration and localStorage persistence.

**Architecture:** Config-driven registry pattern with trait objects for runtime renderer polymorphism. Single TilingCanvasRenderer instance swaps renderer/colorizer via trait objects. Leptos reactive effects orchestrate state synchronization. LocalStorage provides persistence.

**Tech Stack:** Rust + Leptos 0.6 + WASM, Tailwind CSS, serde for serialization

---

## Task 1: Extend AppData enum with MandelbrotData variant

**Files:**
- Modify: `src/rendering/app_data.rs`

**Step 1: Add MandelbrotData variant to AppData enum**

Edit `src/rendering/app_data.rs`:

```rust
use crate::rendering::computers::mandelbrot::MandelbrotData;
use crate::rendering::computers::test_image::TestImageData;

#[derive(Debug, Clone, Copy)]
pub enum AppData {
    TestImageData(TestImageData),
    MandelbrotData(MandelbrotData),
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS (note: mandelbrot module doesn't exist yet, so this will fail - that's OK, we'll create it next)

**Step 3: Commit**

```bash
git add src/rendering/app_data.rs
git commit -m "feat: add MandelbrotData variant to AppData enum"
```

---

## Task 2: Implement MandelbrotComputer

**Files:**
- Create: `src/rendering/computers/mandelbrot.rs`
- Modify: `src/rendering/computers/mod.rs`

**Step 1: Write test for MandelbrotComputer**

Create `src/rendering/computers/mandelbrot.rs`:

```rust
use crate::rendering::point_compute::ImagePointComputer;
use crate::rendering::points::Point;
use crate::rendering::renderer_info::{RendererInfo, RendererInfoData};
use crate::rendering::viewport::Viewport;

#[derive(Debug, Clone, Copy)]
pub struct MandelbrotData {
    pub iterations: u32,
    pub escaped: bool,
}

#[derive(Debug, Clone)]
pub struct MandelbrotComputer {
    max_iterations: u32,
}

impl MandelbrotComputer {
    pub fn new() -> Self {
        Self { max_iterations: 256 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandelbrot_point_in_set() {
        let computer = MandelbrotComputer::new();
        let point = Point::new(0.0, 0.0); // Origin is in Mandelbrot set
        let result = computer.compute(point);
        assert!(!result.escaped);
        assert_eq!(result.iterations, 256);
    }

    #[test]
    fn test_mandelbrot_point_outside_set() {
        let computer = MandelbrotComputer::new();
        let point = Point::new(2.0, 2.0); // Far outside set
        let result = computer.compute(point);
        assert!(result.escaped);
        assert!(result.iterations < 256);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractal_wonder --lib rendering::computers::mandelbrot::tests`
Expected: FAIL with "compute method not found"

**Step 3: Implement ImagePointComputer trait**

Add to `src/rendering/computers/mandelbrot.rs` after `impl MandelbrotComputer`:

```rust
impl ImagePointComputer for MandelbrotComputer {
    type Coord = f64;
    type Data = MandelbrotData;

    fn compute(&self, point: Point<f64>) -> MandelbrotData {
        let cx = *point.x();
        let cy = *point.y();

        let mut zx = 0.0;
        let mut zy = 0.0;

        for i in 0..self.max_iterations {
            let zx_sq = zx * zx;
            let zy_sq = zy * zy;

            if zx_sq + zy_sq > 4.0 {
                return MandelbrotData {
                    iterations: i,
                    escaped: true,
                };
            }

            let new_zx = zx_sq - zy_sq + cx;
            let new_zy = 2.0 * zx * zy + cy;

            zx = new_zx;
            zy = new_zy;
        }

        MandelbrotData {
            iterations: self.max_iterations,
            escaped: false,
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractal_wonder --lib rendering::computers::mandelbrot::tests`
Expected: PASS (2 tests)

**Step 5: Implement RendererInfo trait**

Add to `src/rendering/computers/mandelbrot.rs`:

```rust
impl RendererInfo for MandelbrotComputer {
    type Coord = f64;

    fn info(&self, viewport: &Viewport<f64>) -> RendererInfoData {
        RendererInfoData {
            name: "Mandelbrot".to_string(),
            center_display: format!("{:.6}, {:.6}", viewport.center.x(), viewport.center.y()),
            zoom_display: format!("{:.2e}", viewport.zoom),
            custom_params: vec![
                ("Max Iterations".to_string(), self.max_iterations.to_string())
            ],
            render_time_ms: None,
        }
    }
}
```

**Step 6: Export module**

Edit `src/rendering/computers/mod.rs`:

```rust
pub mod mandelbrot;
pub mod test_image;

pub use mandelbrot::{MandelbrotComputer, MandelbrotData};
pub use test_image::{TestImageComputer, TestImageData};
```

**Step 7: Verify everything compiles**

Run: `cargo check`
Expected: SUCCESS

**Step 8: Run all tests**

Run: `cargo test --package fractal_wonder --lib rendering::computers`
Expected: All tests PASS

**Step 9: Commit**

```bash
git add src/rendering/computers/mandelbrot.rs src/rendering/computers/mod.rs
git commit -m "feat: implement MandelbrotComputer with ImagePointComputer and RendererInfo traits"
```

---

## Task 3: Add new colorizers

**Files:**
- Modify: `src/rendering/colorizers.rs`

**Step 1: Write tests for new colorizers**

Add to end of `src/rendering/colorizers.rs` (in the test module):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_image_pastel_colorizer() {
        let data = AppData::TestImageData(TestImageData {
            checkerboard: true,
            circle_distance: 0.5,
        });
        let (r, g, b, a) = test_image_pastel_colorizer(&data);
        assert_eq!(a, 255); // Always opaque
        assert!(r < 255 && g < 255 && b < 255); // Not pure white
    }

    #[test]
    fn test_mandelbrot_default_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 128,
            escaped: true,
        });
        let (r, g, b, a) = mandelbrot_default_colorizer(&data);
        assert_eq!(a, 255); // Always opaque
        assert_eq!(r, g); // Grayscale
        assert_eq!(g, b);
    }

    #[test]
    fn test_mandelbrot_fire_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 64,
            escaped: true,
        });
        let (r, g, b, a) = mandelbrot_fire_colorizer(&data);
        assert_eq!(a, 255); // Always opaque
        assert!(r > b); // Fire has more red than blue
    }

    #[test]
    fn test_mandelbrot_opal_colorizer() {
        let data = AppData::MandelbrotData(MandelbrotData {
            iterations: 64,
            escaped: true,
        });
        let (r, g, b, a) = mandelbrot_opal_colorizer(&data);
        assert_eq!(a, 255); // Always opaque
        assert!(b > r); // Opal has more blue than red
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

**Step 2: Run tests to verify they fail**

Run: `cargo test --package fractal_wonder --lib rendering::colorizers::tests`
Expected: FAIL with "function not found" errors

**Step 3: Implement test_image_pastel_colorizer**

Add to `src/rendering/colorizers.rs` after `test_image_default_colorizer`:

```rust
pub fn test_image_pastel_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => {
            let base_hue = if d.checkerboard { 200.0 } else { 50.0 }; // Blue vs Yellow
            let lightness = 0.7 + (d.circle_distance.sin() * 0.2); // 0.5-0.9 range
            let saturation = 0.4; // Pastel = low saturation

            // HSL to RGB conversion (simplified)
            let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
            let h_prime = base_hue / 60.0;
            let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());

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

**Step 4: Implement mandelbrot_default_colorizer**

Add to `src/rendering/colorizers.rs`:

```rust
pub fn mandelbrot_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255); // Set interior is black
            }

            // Grayscale gradient based on iteration count
            let normalized = (d.iterations as f64 / 256.0).min(1.0);
            let intensity = (normalized * 255.0) as u8;

            (intensity, intensity, intensity, 255)
        }
        _ => (0, 0, 0, 255),
    }
}
```

**Step 5: Implement mandelbrot_fire_colorizer**

Add to `src/rendering/colorizers.rs`:

```rust
pub fn mandelbrot_fire_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255); // Set interior is black
            }

            // Fire gradient: Black -> Red -> Orange -> Yellow -> White
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
```

**Step 6: Implement mandelbrot_opal_colorizer**

Add to `src/rendering/colorizers.rs`:

```rust
pub fn mandelbrot_opal_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            if !d.escaped {
                return (0, 0, 0, 255); // Set interior is black
            }

            // Opal gradient: Black -> Deep Blue -> Cyan -> White
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
```

**Step 7: Run tests to verify they pass**

Run: `cargo test --package fractal_wonder --lib rendering::colorizers::tests`
Expected: All tests PASS

**Step 8: Commit**

```bash
git add src/rendering/colorizers.rs
git commit -m "feat: add pastel colorizer for test image and three colorizers for mandelbrot (default, fire, opal)"
```

---

## Task 4: Modify TilingCanvasRenderer to use trait objects

**Files:**
- Modify: `src/rendering/tiling_canvas_renderer.rs`

**Step 1: Change struct to use trait object**

Edit `src/rendering/tiling_canvas_renderer.rs`. Change the struct definition from:

```rust
pub struct TilingCanvasRenderer<R: Renderer> {
    renderer: R,
    // ...
}
```

To:

```rust
pub struct TilingCanvasRenderer<C: CoordFloat> {
    renderer: Box<dyn Renderer<Coord=C, Data=AppData>>,
    colorizer: Colorizer<AppData>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<AppData>>>,
}
```

**Step 2: Update CachedState to use AppData**

Change `CachedState` from:

```rust
struct CachedState<R: Renderer> {
    viewport: Option<Viewport<R::Coord>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<R::Data>,
}
```

To:

```rust
struct CachedState<C: CoordFloat> {
    viewport: Option<Viewport<C>>,
    canvas_size: Option<(u32, u32)>,
    data: Vec<AppData>,
}
```

**Step 3: Update impl blocks**

Change all `impl<R: Renderer>` to `impl<C: CoordFloat>` and update method signatures accordingly.

Update `new` method:

```rust
impl<C: CoordFloat> TilingCanvasRenderer<C> {
    pub fn new(
        renderer: Box<dyn Renderer<Coord=C, Data=AppData>>,
        colorizer: Colorizer<AppData>,
        tile_size: u32,
    ) -> Self {
        Self {
            renderer,
            colorizer,
            tile_size,
            cached_state: Arc::new(Mutex::new(CachedState {
                viewport: None,
                canvas_size: None,
                data: Vec::new(),
            })),
        }
    }
}
```

**Step 4: Add set_renderer and set_colorizer methods**

Add after the `new` method:

```rust
impl<C: CoordFloat> TilingCanvasRenderer<C> {
    // ... existing new() method ...

    pub fn set_renderer(&mut self, renderer: Box<dyn Renderer<Coord=C, Data=AppData>>) {
        self.renderer = renderer;
        self.clear_cache();
    }

    pub fn set_colorizer(&mut self, colorizer: Colorizer<AppData>) {
        self.colorizer = colorizer;
        // Cache preserved!
    }

    fn clear_cache(&mut self) {
        let mut cache = self.cached_state.lock().unwrap();
        cache.viewport = None;
        cache.canvas_size = None;
        cache.data.clear();
    }
}
```

**Step 5: Remove or update with_colorizer method**

The existing `with_colorizer` method that returns a new instance can be removed or kept for backwards compatibility. For simplicity, remove it since we now have `set_colorizer`.

**Step 6: Update all type references**

Throughout the file, replace references to `R::Coord` with `C` and `R::Data` with `AppData`.

**Step 7: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS (note: App.rs will fail because it still uses old API - we'll fix that later)

**Step 8: Run tests**

Run: `cargo test --package fractal_wonder --lib rendering::tiling_canvas_renderer`
Expected: Tests may need updates - fix any that fail due to API changes

**Step 9: Commit**

```bash
git add src/rendering/tiling_canvas_renderer.rs
git commit -m "refactor: change TilingCanvasRenderer to use trait objects instead of generic type parameter"
```

---

## Task 5: Create RenderConfig registry

**Files:**
- Create: `src/rendering/render_config.rs`
- Modify: `src/rendering/mod.rs`

**Step 1: Create render_config module**

Create `src/rendering/render_config.rs`:

```rust
use crate::rendering::{
    AppData, AppDataRenderer, Colorizer, MandelbrotComputer, PixelRenderer, Renderer,
    TestImageComputer,
};
use crate::rendering::colorizers::{
    mandelbrot_default_colorizer, mandelbrot_fire_colorizer, mandelbrot_opal_colorizer,
    test_image_default_colorizer, test_image_pastel_colorizer,
};
use crate::rendering::renderer_info::RendererInfo;

pub struct ColorScheme {
    pub id: &'static str,
    pub display_name: &'static str,
    pub colorizer: Colorizer<AppData>,
}

pub struct RenderConfig {
    pub id: &'static str,
    pub display_name: &'static str,
    pub color_schemes: Vec<ColorScheme>,
    pub default_color_scheme_id: &'static str,
    pub create_renderer: fn() -> Box<dyn Renderer<Coord=f64, Data=AppData>>,
    pub create_info_provider: fn() -> Box<dyn RendererInfo<Coord=f64>>,
}

fn create_test_image_renderer() -> Box<dyn Renderer<Coord=f64, Data=AppData>> {
    let computer = TestImageComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));
    Box::new(app_renderer)
}

fn create_mandelbrot_renderer() -> Box<dyn Renderer<Coord=f64, Data=AppData>> {
    let computer = MandelbrotComputer::new();
    let pixel_renderer = PixelRenderer::new(computer);
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::MandelbrotData(*d));
    Box::new(app_renderer)
}

pub static RENDER_CONFIGS: &[RenderConfig] = &[
    RenderConfig {
        id: "test_image",
        display_name: "Test Image",
        color_schemes: vec![
            ColorScheme {
                id: "default",
                display_name: "Default",
                colorizer: test_image_default_colorizer,
            },
            ColorScheme {
                id: "pastel",
                display_name: "Pastel",
                colorizer: test_image_pastel_colorizer,
            },
        ],
        default_color_scheme_id: "default",
        create_renderer: create_test_image_renderer,
        create_info_provider: || Box::new(TestImageComputer::new()),
    },
    RenderConfig {
        id: "mandelbrot",
        display_name: "Mandelbrot",
        color_schemes: vec![
            ColorScheme {
                id: "default",
                display_name: "Default",
                colorizer: mandelbrot_default_colorizer,
            },
            ColorScheme {
                id: "fire",
                display_name: "Fire",
                colorizer: mandelbrot_fire_colorizer,
            },
            ColorScheme {
                id: "opal",
                display_name: "Opal",
                colorizer: mandelbrot_opal_colorizer,
            },
        ],
        default_color_scheme_id: "default",
        create_renderer: create_mandelbrot_renderer,
        create_info_provider: || Box::new(MandelbrotComputer::new()),
    },
];

pub fn get_config(id: &str) -> Option<&'static RenderConfig> {
    RENDER_CONFIGS.iter().find(|c| c.id == id)
}

pub fn get_color_scheme<'a>(config: &'a RenderConfig, scheme_id: &str) -> Option<&'a ColorScheme> {
    config.color_schemes.iter().find(|cs| cs.id == scheme_id)
}
```

**Step 2: Export from rendering module**

Edit `src/rendering/mod.rs`, add:

```rust
pub mod render_config;
pub use render_config::{RenderConfig, ColorScheme, RENDER_CONFIGS, get_config, get_color_scheme};
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS (or compilation errors for Vec initialization in const context - see next step)

**Step 4: Fix const Vec issue if needed**

If you get errors about `Vec` in const context, change the `color_schemes` field type from `Vec<ColorScheme>` to `&'static [ColorScheme]` and update the definitions to use arrays:

```rust
pub struct RenderConfig {
    // ...
    pub color_schemes: &'static [ColorScheme],
    // ...
}

pub static RENDER_CONFIGS: &[RenderConfig] = &[
    RenderConfig {
        id: "test_image",
        display_name: "Test Image",
        color_schemes: &[
            ColorScheme { id: "default", display_name: "Default", colorizer: test_image_default_colorizer },
            ColorScheme { id: "pastel", display_name: "Pastel", colorizer: test_image_pastel_colorizer },
        ],
        // ...
    },
    // ...
];
```

**Step 5: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add src/rendering/render_config.rs src/rendering/mod.rs
git commit -m "feat: add RenderConfig registry for declarative renderer and colorizer configuration"
```

---

## Task 6: Create DropdownMenu component

**Files:**
- Create: `src/components/dropdown_menu.rs`
- Modify: `src/components/mod.rs`

**Step 1: Create dropdown_menu component**

Create `src/components/dropdown_menu.rs`:

```rust
use leptos::*;

#[component]
pub fn DropdownMenu<F>(
    label: String,
    options: Signal<Vec<(String, String)>>, // (id, display_name)
    selected_id: Signal<String>,
    on_select: F,
) -> impl IntoView
where
    F: Fn(String) + 'static,
{
    let (is_open, set_is_open) = create_signal(false);

    view! {
        <div class="relative">
            <button
                class="text-white hover:text-gray-200 hover:bg-white/10 rounded-lg px-3 py-2 transition-colors flex items-center gap-2"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                <span class="text-sm">{label.clone()}</span>
                <span class="text-xs opacity-70">"▾"</span>
            </button>

            {move || is_open.get().then(|| view! {
                <div class="absolute bottom-full mb-2 left-0 min-w-40 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg overflow-hidden">
                    <For
                        each=move || options.get()
                        key=|(id, _)| id.clone()
                        children=move |(id, name)| {
                            let is_selected = move || selected_id.get() == id;
                            let id_clone = id.clone();
                            view! {
                                <button
                                    class=move || format!(
                                        "w-full text-left px-4 py-2 text-sm transition-colors {}",
                                        if is_selected() {
                                            "bg-white/20 text-white"
                                        } else {
                                            "text-gray-300 hover:bg-white/10 hover:text-white"
                                        }
                                    )
                                    on:click=move |_| {
                                        on_select(id_clone.clone());
                                        set_is_open.set(false);
                                    }
                                >
                                    {name}
                                </button>
                            }
                        }
                    />
                </div>
            })}
        </div>
    }
}
```

**Step 2: Export from components module**

Edit `src/components/mod.rs`, add:

```rust
pub mod dropdown_menu;
pub use dropdown_menu::DropdownMenu;
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add src/components/dropdown_menu.rs src/components/mod.rs
git commit -m "feat: add DropdownMenu component for renderer and colorizer selection"
```

---

## Task 7: Add localStorage state persistence

**Files:**
- Create: `src/state/mod.rs`
- Create: `src/state/app_state.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml` (add serde dependency if not present)

**Step 1: Add serde dependencies**

Edit `Cargo.toml`, ensure these dependencies exist:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Step 2: Create app_state module**

Create `src/state/mod.rs`:

```rust
pub mod app_state;
pub use app_state::{AppState, RendererState};
```

Create `src/state/app_state.rs`:

```rust
use crate::rendering::{Viewport, RENDER_CONFIGS};
use leptos::window;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const STORAGE_KEY: &str = "fractal_wonder_state";

#[derive(Clone, Serialize, Deserialize)]
pub struct RendererState {
    pub viewport: Viewport<f64>,
    pub color_scheme_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub selected_renderer_id: String,
    pub renderer_states: HashMap<String, RendererState>,
}

impl AppState {
    pub fn load() -> Self {
        window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|storage| storage.get_item(STORAGE_KEY).ok().flatten())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_else(Self::default)
    }

    pub fn save(&self) {
        if let Some(storage) = window().local_storage().ok().flatten() {
            if let Ok(json) = serde_json::to_string(self) {
                let _ = storage.set_item(STORAGE_KEY, &json);
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        let mut renderer_states = HashMap::new();

        for config in RENDER_CONFIGS.iter() {
            let renderer = (config.create_renderer)();
            let natural_bounds = renderer.natural_bounds();

            renderer_states.insert(
                config.id.to_string(),
                RendererState {
                    viewport: Viewport::new(natural_bounds.center(), 1.0),
                    color_scheme_id: config.default_color_scheme_id.to_string(),
                },
            );
        }

        AppState {
            selected_renderer_id: "test_image".to_string(),
            renderer_states,
        }
    }
}
```

**Step 3: Make Viewport serializable**

Edit `src/rendering/viewport.rs`, add serde derives:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Viewport<T> {
    pub center: Point<T>,
    pub zoom: f64,
}
```

**Step 4: Make Point serializable**

Edit `src/rendering/points.rs`, add serde derives:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point<T> {
    x: T,
    y: T,
}
```

**Step 5: Export state module**

Edit `src/lib.rs`, add:

```rust
pub mod state;
```

**Step 6: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS

**Step 7: Run tests**

Run: `cargo test`
Expected: All tests PASS

**Step 8: Commit**

```bash
git add src/state/ src/rendering/viewport.rs src/rendering/points.rs Cargo.toml src/lib.rs
git commit -m "feat: add AppState with localStorage persistence for renderer and viewport state"
```

---

## Task 8: Refactor App.rs to use new architecture

**Files:**
- Modify: `src/app.rs`

**Step 1: Update imports**

Edit `src/app.rs`, update imports:

```rust
use crate::components::dropdown_menu::DropdownMenu;
use crate::components::interactive_canvas::InteractiveCanvas;
use crate::components::ui::UI;
use crate::hooks::fullscreen::toggle_fullscreen;
use crate::hooks::ui_visibility::use_ui_visibility;
use crate::rendering::{
    get_color_scheme, get_config, renderer_info::RendererInfoData, AppData, Colorizer,
    TilingCanvasRenderer, Viewport, RENDER_CONFIGS,
};
use crate::state::{AppState, RendererState};
use leptos::*;
use std::time::Duration;
```

**Step 2: Replace App component implementation**

Replace the entire `App` component with:

```rust
#[component]
pub fn App() -> impl IntoView {
    // ========== Load state from localStorage ==========
    let initial_state = AppState::load();

    let (selected_renderer_id, set_selected_renderer_id) =
        create_signal(initial_state.selected_renderer_id.clone());
    let (renderer_states, set_renderer_states) = create_signal(initial_state.renderer_states);

    // Get initial config
    let initial_config = get_config(&initial_state.selected_renderer_id).unwrap();
    let initial_renderer_state = initial_state
        .renderer_states
        .get(&initial_state.selected_renderer_id)
        .unwrap();

    // ========== Create initial renderer ==========
    let initial_renderer = (initial_config.create_renderer)();
    let initial_colorizer = get_color_scheme(
        initial_config,
        &initial_renderer_state.color_scheme_id,
    )
    .unwrap()
    .colorizer;

    let natural_bounds = initial_renderer.natural_bounds();
    let (viewport, set_viewport) = create_signal(initial_renderer_state.viewport.clone());

    // ========== Canvas renderer with cache ==========
    let canvas_renderer = create_rw_signal(TilingCanvasRenderer::new(
        initial_renderer,
        initial_colorizer,
        128,
    ));

    // ========== RendererInfo for UI display ==========
    let initial_info_provider = (initial_config.create_info_provider)();
    let (info_provider, set_info_provider) =
        create_signal(initial_info_provider as Box<dyn crate::rendering::renderer_info::RendererInfo<Coord = f64>>);
    let (render_time_ms, set_render_time_ms) = create_signal(None::<f64>);
    let (renderer_info, set_renderer_info) =
        create_signal(info_provider.get_untracked().info(&viewport.get_untracked()));

    // ========== Effect: Update renderer info when viewport or render time changes ==========
    create_effect(move |_| {
        let vp = viewport.get();
        let provider = info_provider.get();
        let mut info = provider.info(&vp);
        info.render_time_ms = render_time_ms.get();
        set_renderer_info.set(info);
    });

    // ========== Effect: Renderer selection changed ==========
    create_effect(move |_| {
        let renderer_id = selected_renderer_id.get();
        let config = get_config(&renderer_id).unwrap();
        let states = renderer_states.get();
        let state = states.get(&renderer_id).unwrap();

        // Create new renderer and info provider
        let new_renderer = (config.create_renderer)();
        let new_info_provider = (config.create_info_provider)();

        // Find colorizer for restored color scheme
        let colorizer = get_color_scheme(config, &state.color_scheme_id)
            .unwrap()
            .colorizer;

        // Update canvas renderer (invalidates cache)
        canvas_renderer.update(|cr| {
            cr.set_renderer(new_renderer);
            cr.set_colorizer(colorizer);
        });

        // Restore viewport
        set_viewport.set(state.viewport.clone());

        // Update info provider
        set_info_provider.set(new_info_provider);

        // Save immediately
        AppState {
            selected_renderer_id: renderer_id,
            renderer_states: states,
        }
        .save();
    });

    // ========== Effect: Viewport changed (save debounced) ==========
    let (viewport_save_trigger, set_viewport_save_trigger) = create_signal(());

    create_effect(move |_| {
        viewport.get();
        set_timeout(
            move || {
                set_viewport_save_trigger.update(|_| {});
            },
            Duration::from_millis(500),
        );
    });

    create_effect(move |_| {
        viewport_save_trigger.get();
        let vp = viewport.get();
        let renderer_id = selected_renderer_id.get();

        set_renderer_states.update(|states| {
            if let Some(state) = states.get_mut(&renderer_id) {
                state.viewport = vp;
            }
        });

        let states = renderer_states.get();
        AppState {
            selected_renderer_id: renderer_id,
            renderer_states: states,
        }
        .save();
    });

    // ========== Derived signal: Selected color scheme ID ==========
    let selected_color_scheme_id = create_memo(move |_| {
        let renderer_id = selected_renderer_id.get();
        let states = renderer_states.get();
        states
            .get(&renderer_id)
            .map(|s| s.color_scheme_id.clone())
            .unwrap_or_default()
    });

    // ========== Effect: Color scheme changed ==========
    let (color_scheme_change_trigger, set_color_scheme_change_trigger) = create_signal(());

    let on_color_scheme_select = move |scheme_id: String| {
        let renderer_id = selected_renderer_id.get();
        let config = get_config(&renderer_id).unwrap();

        let colorizer = get_color_scheme(config, &scheme_id).unwrap().colorizer;

        canvas_renderer.update(|cr| {
            cr.set_colorizer(colorizer);
        });

        set_renderer_states.update(|states| {
            if let Some(state) = states.get_mut(&renderer_id) {
                state.color_scheme_id = scheme_id.clone();
            }
        });

        let states = renderer_states.get();
        AppState {
            selected_renderer_id: renderer_id,
            renderer_states: states,
        }
        .save();

        set_color_scheme_change_trigger.update(|_| {});
    };

    // ========== UI menu options ==========
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

    // ========== UI visibility and callbacks ==========
    let ui_visibility = use_ui_visibility();

    let on_home_click = move || {
        let renderer_id = selected_renderer_id.get();
        let config = get_config(&renderer_id).unwrap();
        let renderer = (config.create_renderer)();
        let natural_bounds = renderer.natural_bounds();
        set_viewport.set(Viewport::new(natural_bounds.center(), 1.0));
    };

    let on_fullscreen_click = move || {
        toggle_fullscreen();
    };

    view! {
        <div class="relative w-screen h-screen overflow-hidden bg-black">
            <InteractiveCanvas
                canvas_renderer=canvas_renderer
                viewport=viewport
                set_viewport=set_viewport
                set_render_time_ms=set_render_time_ms
                natural_bounds=natural_bounds
            />
            <UI
                info=renderer_info
                is_visible=ui_visibility.is_visible
                set_is_hovering=ui_visibility.set_is_hovering
                on_home_click=on_home_click
                on_fullscreen_click=on_fullscreen_click
                render_function_options=render_function_options
                selected_renderer_id=Signal::derive(move || selected_renderer_id.get())
                on_renderer_select=move |id: String| set_selected_renderer_id.set(id)
                color_scheme_options=color_scheme_options
                selected_color_scheme_id=Signal::derive(move || selected_color_scheme_id.get())
                on_color_scheme_select=on_color_scheme_select
            />
        </div>
    }
}
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compilation errors in UI component (missing props) - we'll fix that next

**Step 4: Don't commit yet**

We need to update the UI component first before this will compile.

---

## Task 9: Update UI component to include dropdown menus

**Files:**
- Modify: `src/components/ui.rs`

**Step 1: Update UI component signature**

Edit `src/components/ui.rs`, update the component signature to include new props:

```rust
#[component]
pub fn UI(
    info: Signal<RendererInfoData>,
    is_visible: Signal<bool>,
    set_is_hovering: WriteSignal<bool>,
    on_home_click: impl Fn() + 'static + Copy,
    on_fullscreen_click: impl Fn() + 'static + Copy,
    render_function_options: Signal<Vec<(String, String)>>,
    selected_renderer_id: Signal<String>,
    on_renderer_select: impl Fn(String) + 'static + Copy,
    color_scheme_options: Signal<Vec<(String, String)>>,
    selected_color_scheme_id: Signal<String>,
    on_color_scheme_select: impl Fn(String) + 'static + Copy,
) -> impl IntoView {
    // ... existing implementation
}
```

**Step 2: Update the bottom bar layout**

Find the bottom bar section and update it to include the dropdown menus:

```rust
<div class="flex items-center gap-2">
    <InfoButton
        is_open=is_info_open
        set_is_open=set_is_info_open
    />
    <HomeButton on_click=on_home_click />

    <DropdownMenu
        label="Render Function".to_string()
        options=render_function_options
        selected_id=selected_renderer_id
        on_select=on_renderer_select
    />

    <DropdownMenu
        label="Color Scheme".to_string()
        options=color_scheme_options
        selected_id=selected_color_scheme_id
        on_select=on_color_scheme_select
    />
</div>
```

**Step 3: Add DropdownMenu import**

Add to imports at top of file:

```rust
use crate::components::dropdown_menu::DropdownMenu;
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: SUCCESS

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 6: Commit App.rs and UI.rs together**

```bash
git add src/app.rs src/components/ui.rs
git commit -m "feat: integrate renderer/colorizer selection menus with localStorage persistence"
```

---

## Task 10: Manual browser testing

**Step 1: Start dev server**

Run: `trunk serve`

**Step 2: Open browser**

Navigate to: `http://localhost:8080`

**Step 3: Test render function menu**

- Click "Render Function" dropdown
- Verify it shows "Test Image" and "Mandelbrot" options
- Select "Mandelbrot"
- Verify it renders the Mandelbrot set
- Verify the info display shows "Mandelbrot"

**Step 4: Test color scheme menu**

- With Test Image selected, verify Color Scheme shows "Default" and "Pastel"
- Select "Pastel"
- Verify colors change without recomputing
- Switch to Mandelbrot
- Verify Color Scheme now shows "Default", "Fire", and "Opal"
- Try each color scheme

**Step 5: Test per-renderer state persistence**

- Select Test Image, zoom in, pan around
- Select Mandelbrot
- Verify it goes to Mandelbrot's home position
- Zoom/pan in Mandelbrot
- Switch back to Test Image
- Verify it returns to where you left Test Image

**Step 6: Test localStorage persistence**

- Select a renderer, color scheme, and zoom/pan position
- Refresh the page (F5)
- Verify everything is restored correctly

**Step 7: Test existing functionality**

- Verify home button works
- Verify pan/zoom interactions work
- Verify fullscreen button works
- Verify info popover works

**Step 8: Document any issues**

If any issues found, create GitHub issues or fix immediately.

---

## Task 11: Format and lint

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

**Step 3: Fix any warnings**

Address any warnings or errors from Clippy.

**Step 4: Commit if changes made**

```bash
git add -u
git commit -m "chore: format code and fix clippy warnings"
```

---

## Task 12: Final verification

**Step 1: Clean build**

Run: `cargo clean && cargo build`
Expected: SUCCESS

**Step 2: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests PASS

**Step 3: Build release version**

Run: `trunk build --release`
Expected: SUCCESS

**Step 4: Final manual test in release build**

Serve the release build and verify all functionality works.

---

## Completion

All tasks complete! The implementation adds:
- ✅ Two UI dropdown menus for renderer and colorizer selection
- ✅ Mandelbrot fractal renderer with three colorizers
- ✅ Pastel colorizer for test image
- ✅ Per-renderer state persistence (viewport + color scheme)
- ✅ LocalStorage integration for state persistence across page reloads
- ✅ Config-driven registry architecture
- ✅ Trait object-based renderer swapping
- ✅ Cache-aware colorizer swapping (no recomputation)
- ✅ All existing functionality preserved

**Next steps:**
- Consider adding max_iterations parameter UI control for Mandelbrot
- Consider adding more renderers (Julia sets, Lyapunov diagrams)
- Consider adding more colorizers
