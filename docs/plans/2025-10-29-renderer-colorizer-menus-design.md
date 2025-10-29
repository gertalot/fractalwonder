# Renderer and Colorizer Selection Menus

**Date:** 2025-10-29
**Status:** Design Approved
**Goal:** Add UI menus for selecting render functions and color schemes with stateful per-renderer configuration and localStorage persistence.

## Overview

Add two dropdown menus to the UI that allow runtime selection of:
1. **Render Function** - computational strategy (Test Image, Mandelbrot)
2. **Color Scheme** - colorization strategy (contextual options based on selected renderer)

Key behaviors:
- Switching colorizers recolorizes from cache (no recomputation)
- Switching renderers triggers full recomputation
- Each renderer remembers its last viewport and color scheme
- All state persists to localStorage across page reloads

## Requirements

### UI Requirements
- Two dropdown menus in bottom bar: `[Info] [Home] [Render Function ▾] [Color Scheme ▾] [Center/Zoom] [Fullscreen]`
- Menu options:
  - **Render Function**: Test Image | Mandelbrot
  - **Color Scheme** (contextual):
    - Test Image → Default | Pastel
    - Mandelbrot → Default | Fire | Opal

### State Requirements
- Per-renderer state persistence:
  - Viewport (center coordinates + zoom)
  - Selected color scheme
- Global state:
  - Currently selected renderer
- Initial state: Test Image + Default color + home viewport
- LocalStorage persistence:
  - Debounced saves for viewport (500ms)
  - Immediate saves for renderer/color scheme changes

### Behavioral Requirements
- **Switching renderer** → restore that renderer's last viewport + color scheme, trigger recompute
- **Switching color scheme** → recolorize only (preserve cache)
- **Viewport changes** (pan/zoom/home) → trigger recompute for current renderer
- **Existing functionality preserved** → all current interactions continue working

## Architecture

### 1. RenderConfig Registry

Single source of truth for all renderers and their colorizers:

```rust
// src/rendering/render_config.rs

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
            ColorScheme { id: "default", display_name: "Default", colorizer: mandelbrot_default_colorizer },
            ColorScheme { id: "fire", display_name: "Fire", colorizer: mandelbrot_fire_colorizer },
            ColorScheme { id: "opal", display_name: "Opal", colorizer: mandelbrot_opal_colorizer },
        ],
        default_color_scheme_id: "default",
        create_renderer: create_mandelbrot_renderer,
        create_info_provider: || Box::new(MandelbrotComputer::new()),
    },
];

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
```

**Design rationale:**
- Declarative configuration makes adding renderers trivial
- UI can iterate RENDER_CONFIGS to build menus automatically
- Factory functions provide trait objects for runtime polymorphism
- Type safety preserved through Rust's type system

### 2. TilingCanvasRenderer Modifications

Change from generic type parameter to trait object storage:

```rust
// src/rendering/tiling_canvas_renderer.rs

pub struct TilingCanvasRenderer<C: CoordFloat> {
    renderer: Box<dyn Renderer<Coord=C, Data=AppData>>,  // Changed from generic R
    colorizer: Colorizer<AppData>,
    tile_size: u32,
    cached_state: Arc<Mutex<CachedState<AppData>>>,
}

impl<C: CoordFloat> TilingCanvasRenderer<C> {
    pub fn new(
        renderer: Box<dyn Renderer<Coord=C, Data=AppData>>,
        colorizer: Colorizer<AppData>,
        tile_size: u32,
    ) -> Self {
        // Existing implementation
    }

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

**Key changes:**
- Trait object (`Box<dyn Renderer>`) enables runtime renderer swapping
- `set_renderer()` invalidates cache (new computation needed)
- `set_colorizer()` preserves cache (recolorize only)
- Removed generic `R` parameter, keeping only `C` for coordinates

### 3. State Management with LocalStorage

```rust
// In App.rs

const STORAGE_KEY: &str = "fractal_wonder_state";
const VIEWPORT_SAVE_DEBOUNCE_MS: i32 = 500;

#[derive(Clone, Serialize, Deserialize)]
struct RendererState {
    viewport: Viewport<f64>,      // Contains center + zoom
    color_scheme_id: String,
}

#[derive(Serialize, Deserialize)]
struct AppState {
    selected_renderer_id: String,
    renderer_states: HashMap<String, RendererState>,
}

impl AppState {
    fn load() -> Self {
        window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|storage| storage.get_item(STORAGE_KEY).ok().flatten())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_else(Self::default)
    }

    fn save(&self) {
        if let Some(storage) = window().local_storage().ok().flatten() {
            if let Ok(json) = serde_json::to_string(self) {
                let _ = storage.set_item(STORAGE_KEY, &json);
            }
        }
    }

    fn default() -> Self {
        let mut renderer_states = HashMap::new();
        for config in RENDER_CONFIGS {
            let renderer = (config.create_renderer)();
            let natural_bounds = renderer.natural_bounds();
            renderer_states.insert(
                config.id.to_string(),
                RendererState {
                    viewport: Viewport::new(natural_bounds.center(), 1.0),
                    color_scheme_id: config.default_color_scheme_id.to_string(),
                }
            );
        }
        AppState {
            selected_renderer_id: "test_image".to_string(),
            renderer_states,
        }
    }
}
```

**State orchestration in App component:**

```rust
// Initialize from localStorage
let initial_state = AppState::load();
let (selected_renderer_id, set_selected_renderer_id) = create_signal(initial_state.selected_renderer_id.clone());
let (renderer_states, set_renderer_states) = create_signal(initial_state.renderer_states);
let (info_provider, set_info_provider) = create_signal(/* initial info provider */);

// Effect: Renderer selection changed → restore state, swap renderer
create_effect(move |_| {
    let renderer_id = selected_renderer_id.get();
    let config = RENDER_CONFIGS.iter().find(|c| c.id == renderer_id).unwrap();
    let states = renderer_states.get();
    let state = states.get(renderer_id.as_str()).unwrap();

    // Create new renderer and info provider
    let new_renderer = (config.create_renderer)();
    let new_info_provider = (config.create_info_provider)();

    // Find colorizer for restored color scheme
    let colorizer = config.color_schemes
        .iter()
        .find(|cs| cs.id == state.color_scheme_id)
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
        selected_renderer_id: renderer_id.clone(),
        renderer_states: states,
    }.save();
});

// Effect: Viewport changed → save to current renderer's state (debounced)
// ... debounced save logic

// Effect: Color scheme changed → update colorizer only
create_effect(move |_| {
    let color_scheme_id = selected_color_scheme_id.get();
    let renderer_id = selected_renderer_id.get();
    let config = RENDER_CONFIGS.iter().find(|c| c.id == renderer_id).unwrap();

    let colorizer = config.color_schemes
        .iter()
        .find(|cs| cs.id == color_scheme_id)
        .unwrap()
        .colorizer;

    canvas_renderer.update(|cr| {
        cr.set_colorizer(colorizer);  // Cache preserved!
    });

    // Save color scheme to current renderer's state
    set_renderer_states.update(|states| {
        if let Some(state) = states.get_mut(renderer_id.as_str()) {
            state.color_scheme_id = color_scheme_id.clone();
        }
    });

    // Save immediately
    let states = renderer_states.get();
    AppState { selected_renderer_id: renderer_id, renderer_states: states }.save();
});
```

**Design rationale:**
- HashMap stores independent state per renderer
- Leptos effects handle all state synchronization
- Debounced viewport saves reduce localStorage writes during interaction
- Immediate saves for renderer/color changes ensure user selections persist

### 4. UI Components

New reusable dropdown component:

```rust
// src/components/dropdown_menu.rs

#[component]
pub fn DropdownMenu<F>(
    label: String,
    options: Vec<(String, String)>,  // (id, display_name)
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
                <span class="text-sm">{label}</span>
                <span class="text-xs opacity-70">"▾"</span>
            </button>

            {move || is_open.get().then(|| view! {
                <div class="absolute bottom-full mb-2 left-0 min-w-40 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg overflow-hidden">
                    <For
                        each=move || options.clone()
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

**Integration in UI component:**

```rust
// src/components/ui.rs

<div class="flex items-center gap-2">
    <InfoButton /* existing */ />
    <HomeButton on_click=on_home_click />

    <DropdownMenu
        label="Render Function".to_string()
        options=render_function_options
        selected_id=selected_renderer_id
        on_select=on_renderer_select
    />

    <DropdownMenu
        label="Color Scheme".to_string()
        options=color_scheme_options  // Dynamically filtered
        selected_id=selected_color_scheme_id
        on_select=on_color_scheme_select
    />
</div>
```

**Design rationale:**
- Reuses existing UI styling (dark theme, backdrop blur, hover effects)
- Dropdown opens upward (consistent with InfoButton)
- Selected item highlighted
- Auto-closes on selection
- Generic component for reuse

### 5. New Implementations

#### MandelbrotComputer

```rust
// src/rendering/computers/mandelbrot.rs

pub struct MandelbrotComputer {
    max_iterations: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct MandelbrotData {
    pub iterations: u32,
    pub escaped: bool,
}

impl ImagePointComputer for MandelbrotComputer {
    type Coord = f64;
    type Data = MandelbrotData;

    fn compute(&self, point: Point<f64>) -> MandelbrotData {
        let c = Complex::new(*point.x(), *point.y());
        let mut z = Complex::new(0.0, 0.0);

        for i in 0..self.max_iterations {
            if z.norm_sqr() > 4.0 {
                return MandelbrotData { iterations: i, escaped: true };
            }
            z = z * z + c;
        }

        MandelbrotData {
            iterations: self.max_iterations,
            escaped: false,
        }
    }
}

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

#### AppData Extension

```rust
// src/rendering/app_data.rs

#[derive(Debug, Clone, Copy)]
pub enum AppData {
    TestImageData(TestImageData),
    MandelbrotData(MandelbrotData),
}
```

#### Colorizers

```rust
// src/rendering/colorizers.rs

// Test Image - Default (existing)
pub fn test_image_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => test_image_data_to_rgba(d),
        _ => (0, 0, 0, 255),
    }
}

// Test Image - Pastel (new)
pub fn test_image_pastel_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::TestImageData(d) => {
            // Soft pastel palette based on checkerboard + circle distance
            // Implementation details in code
        }
        _ => (0, 0, 0, 255),
    }
}

// Mandelbrot - Default (new)
pub fn mandelbrot_default_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            // Grayscale gradient based on iteration count
            // Implementation details in code
        }
        _ => (0, 0, 0, 255),
    }
}

// Mandelbrot - Fire (new)
pub fn mandelbrot_fire_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            // Red → Orange → Yellow gradient
            // Implementation details in code
        }
        _ => (0, 0, 0, 255),
    }
}

// Mandelbrot - Opal (new)
pub fn mandelbrot_opal_colorizer(data: &AppData) -> (u8, u8, u8, u8) {
    match data {
        AppData::MandelbrotData(d) => {
            // Blue → Cyan → White iridescent effect
            // Implementation details in code
        }
        _ => (0, 0, 0, 255),
    }
}
```

## Implementation Notes

### Cache Invalidation Rules

| Event | Action | Cache |
|-------|--------|-------|
| Change renderer | Recompute | Cleared |
| Change color scheme | Recolorize | Preserved |
| Change viewport (pan/zoom) | Recompute | Cleared (viewport mismatch) |
| Click home button | Recompute | Cleared (viewport change) |

### Future Extensions

This design supports future enhancements:
- Add `custom_params` field to `RendererState` for per-renderer parameters (max iterations, checkerboard size, etc.)
- UI controls for custom parameters
- More renderers (Lyapunov, Julia sets, map views)
- More colorizers per renderer

### Testing Strategy

- Unit tests for each colorizer function
- Unit tests for MandelbrotComputer
- Browser tests for menu interactions
- Integration tests for state persistence (localStorage mocking)
- Manual testing of all renderer × colorizer combinations

## Summary

This design provides:
- ✅ Clean separation of concerns (computation vs colorization)
- ✅ Extensible architecture (add renderers via registry)
- ✅ Optimal performance (cache-aware colorizer swapping)
- ✅ Excellent UX (stateful per-renderer preferences, localStorage persistence)
- ✅ Type-safe runtime polymorphism (trait objects)
- ✅ Consistent with existing codebase patterns
