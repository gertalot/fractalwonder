# Color Options UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace preset-based color scheme selector with composable controls: palette selection, 3D toggle, smooth toggle, and cycle count adjustment.

**Architecture:** Add `ColorOptions` struct to hold user settings, pass to renderer which builds `ColorSettings` on demand. Toast component shows transient feedback for keyboard-driven changes. Persistence migrates from single `color_scheme_id` to full options struct.

**Tech Stack:** Rust, Leptos 0.6, WebAssembly, Tailwind CSS

---

## Task 1: Add PaletteEntry and palettes() Registry

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Write the failing test**

Add to `mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_returns_11_entries() {
        let palettes = palettes();
        assert_eq!(palettes.len(), 11);
    }

    #[test]
    fn all_palettes_have_unique_ids() {
        let palettes = palettes();
        let ids: Vec<_> = palettes.iter().map(|p| p.id).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len(), "Duplicate palette IDs found");
    }

    #[test]
    fn classic_palette_exists() {
        let palettes = palettes();
        assert!(palettes.iter().any(|p| p.id == "classic"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-ui palettes_returns_11_entries`
Expected: FAIL with "cannot find function `palettes`"

**Step 3: Write minimal implementation**

Add to `mod.rs` before the existing `pub use` statements:

```rust
/// A palette entry with ID, display name, and palette instance.
#[derive(Clone, Debug)]
pub struct PaletteEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub palette: Palette,
}

/// Get all available color palettes.
pub fn palettes() -> Vec<PaletteEntry> {
    vec![
        PaletteEntry { id: "classic", name: "Classic", palette: Palette::ultra_fractal() },
        PaletteEntry { id: "fire", name: "Fire", palette: Palette::fire() },
        PaletteEntry { id: "ocean", name: "Ocean", palette: Palette::ocean() },
        PaletteEntry { id: "electric", name: "Electric", palette: Palette::electric() },
        PaletteEntry { id: "grayscale", name: "Grayscale", palette: Palette::grayscale() },
        PaletteEntry { id: "rainbow", name: "Rainbow", palette: Palette::rainbow() },
        PaletteEntry { id: "neon", name: "Neon", palette: Palette::neon() },
        PaletteEntry { id: "twilight", name: "Twilight", palette: Palette::twilight() },
        PaletteEntry { id: "candy", name: "Candy", palette: Palette::candy() },
        PaletteEntry { id: "inferno", name: "Inferno", palette: Palette::inferno() },
        PaletteEntry { id: "aurora", name: "Aurora", palette: Palette::aurora() },
    ]
}
```

Also add to the pub use section: `pub use self::palettes;` and `pub use self::PaletteEntry;`

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-ui palettes_returns`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add PaletteEntry and palettes() registry"
```

---

## Task 2: Add ColorOptions Struct

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/settings.rs`

**Step 1: Write the failing test**

Add to `settings.rs` tests:

```rust
#[test]
fn color_options_default_values() {
    let options = ColorOptions::default();
    assert_eq!(options.palette_id, "classic");
    assert!(!options.shading_enabled);
    assert!(options.smooth_enabled);
    assert_eq!(options.cycle_count, 32);
}

#[test]
fn color_options_to_color_settings_uses_palette() {
    let mut options = ColorOptions::default();
    options.palette_id = "fire".to_string();
    let settings = options.to_color_settings();
    // Fire palette starts dark, sample at 0 should be near black
    let sample = settings.palette.sample(0.0);
    assert_eq!(sample, [0, 0, 0]);
}

#[test]
fn color_options_to_color_settings_shading() {
    let mut options = ColorOptions::default();
    options.shading_enabled = true;
    let settings = options.to_color_settings();
    assert!(settings.shading.enabled);
}

#[test]
fn color_options_cycle_power_of_two() {
    assert!(ColorOptions::is_valid_cycle_count(1));
    assert!(ColorOptions::is_valid_cycle_count(2));
    assert!(ColorOptions::is_valid_cycle_count(32));
    assert!(ColorOptions::is_valid_cycle_count(128));
    assert!(!ColorOptions::is_valid_cycle_count(3));
    assert!(!ColorOptions::is_valid_cycle_count(0));
    assert!(!ColorOptions::is_valid_cycle_count(256));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-ui color_options_default`
Expected: FAIL with "cannot find value `ColorOptions`"

**Step 3: Write minimal implementation**

Add to `settings.rs` after existing imports:

```rust
use super::palettes;
use serde::{Deserialize, Serialize};
```

Add the struct (before or after `ColorSettings`):

```rust
/// User-configurable color options for the UI.
/// Converted to ColorSettings for rendering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorOptions {
    /// Palette ID (e.g., "classic", "fire").
    pub palette_id: String,
    /// Whether 3D slope shading is enabled.
    pub shading_enabled: bool,
    /// Whether smooth iteration coloring is enabled.
    pub smooth_enabled: bool,
    /// Number of palette cycles (power of 2: 1, 2, 4, ..., 128).
    pub cycle_count: u32,
}

impl Default for ColorOptions {
    fn default() -> Self {
        Self {
            palette_id: "classic".to_string(),
            shading_enabled: false,
            smooth_enabled: true,
            cycle_count: 32,
        }
    }
}

impl ColorOptions {
    /// Valid cycle counts: powers of 2 from 1 to 128.
    pub fn is_valid_cycle_count(n: u32) -> bool {
        n > 0 && n <= 128 && n.is_power_of_two()
    }

    /// Double cycle count (max 128).
    pub fn cycle_up(&mut self) {
        if self.cycle_count < 128 {
            self.cycle_count *= 2;
        }
    }

    /// Halve cycle count (min 1).
    pub fn cycle_down(&mut self) {
        if self.cycle_count > 1 {
            self.cycle_count /= 2;
        }
    }

    /// Convert to ColorSettings for rendering.
    pub fn to_color_settings(&self) -> ColorSettings {
        let palette = palettes()
            .into_iter()
            .find(|p| p.id == self.palette_id)
            .map(|p| p.palette)
            .unwrap_or_else(Palette::ultra_fractal);

        ColorSettings {
            palette,
            cycle_count: self.cycle_count as f64,
            shading: if self.shading_enabled {
                ShadingSettings::enabled()
            } else {
                ShadingSettings::disabled()
            },
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package fractalwonder-ui color_options`
Expected: PASS (all 4 tests)

**Step 5: Export from mod.rs**

Edit `fractalwonder-ui/src/rendering/colorizers/mod.rs` to add to pub use:

```rust
pub use settings::ColorOptions;
```

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/settings.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add ColorOptions struct with to_color_settings()"
```

---

## Task 3: Update PersistedState for ColorOptions

**Files:**
- Modify: `fractalwonder-ui/src/hooks/persistence.rs`

**Step 1: Write the failing test**

Add to `persistence.rs` tests:

```rust
#[test]
fn persisted_state_with_color_options_roundtrips() {
    use crate::rendering::colorizers::ColorOptions;

    let viewport = fractalwonder_core::Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 64);
    let mut options = ColorOptions::default();
    options.palette_id = "fire".to_string();
    options.shading_enabled = true;
    options.cycle_count = 64;

    let state = PersistedState::new(viewport.clone(), "mandelbrot".to_string(), options.clone());

    let encoded = encode_state(&state).expect("encoding should succeed");
    let decoded = decode_state(&encoded).expect("decoding should succeed");

    assert_eq!(decoded.color_options.palette_id, "fire");
    assert!(decoded.color_options.shading_enabled);
    assert_eq!(decoded.color_options.cycle_count, 64);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --package fractalwonder-ui persisted_state_with_color_options`
Expected: FAIL (color_options field doesn't exist)

**Step 3: Write minimal implementation**

Update imports at top of `persistence.rs`:

```rust
use crate::rendering::colorizers::ColorOptions;
```

Update `PersistedState` struct:

```rust
/// State persisted to localStorage between sessions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedState {
    /// Current viewport (center, width, height with arbitrary precision)
    pub viewport: Viewport,
    /// Selected fractal configuration ID
    pub config_id: String,
    /// Color options (palette, shading, smooth, cycles)
    #[serde(default)]
    pub color_options: ColorOptions,
    /// Schema version for future migrations
    version: u32,
}
```

Remove the old `color_scheme_id` field and related code (`default_color_scheme` fn, `DEFAULT_COLOR_SCHEME` const).

Update `PersistedState` impl:

```rust
impl PersistedState {
    const CURRENT_VERSION: u32 = 3;  // Bump version

    pub fn new(viewport: Viewport, config_id: String, color_options: ColorOptions) -> Self {
        Self {
            viewport,
            config_id,
            color_options,
            version: Self::CURRENT_VERSION,
        }
    }

    /// Create state with default color options.
    pub fn with_defaults(viewport: Viewport, config_id: String) -> Self {
        Self::new(viewport, config_id, ColorOptions::default())
    }
}
```

Update version checks in `load_from_local_storage()` and `decode_state()`:

```rust
// Accept v1, v2, v3 (migration handled by serde default)
if state.version >= 1 && state.version <= PersistedState::CURRENT_VERSION {
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-ui persisted_state_with_color_options`
Expected: PASS

**Step 5: Update existing tests**

Update `color_scheme_id_persists_through_encode_decode` test to use new API:

```rust
#[test]
fn color_options_persist_through_encode_decode() {
    let viewport = fractalwonder_core::Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 64);
    let mut options = ColorOptions::default();
    options.palette_id = "fire".to_string();
    let state = PersistedState::new(viewport, "mandelbrot".to_string(), options);

    let encoded = encode_state(&state).expect("encoding should succeed");
    let decoded = decode_state(&encoded).expect("decoding should succeed");

    assert_eq!(decoded.color_options.palette_id, "fire");
}
```

Remove `default_color_scheme_is_classic` test (no longer relevant).

**Step 6: Run all persistence tests**

Run: `cargo test --package fractalwonder-ui persistence`
Expected: PASS

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/hooks/persistence.rs
git commit -m "feat(persistence): update PersistedState to use ColorOptions"
```

---

## Task 4: Create Toast Component

**Files:**
- Create: `fractalwonder-ui/src/components/toast.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create the toast component file**

Create `fractalwonder-ui/src/components/toast.rs`:

```rust
//! Toast notification component for transient feedback.

use leptos::*;

/// Toast notification that appears briefly then fades out.
/// Only shows when UI panel is hidden.
#[component]
pub fn Toast(
    /// Message to display (None = hidden)
    message: Signal<Option<String>>,
    /// Whether UI panel is visible (toast hidden when true)
    ui_visible: Signal<bool>,
) -> impl IntoView {
    // Track visibility with fade animation
    let (is_visible, set_is_visible) = create_signal(false);
    let (display_message, set_display_message) = create_signal(String::new());

    // Handle message changes
    create_effect(move |_| {
        if let Some(msg) = message.get() {
            // Don't show if UI is visible
            if ui_visible.get_untracked() {
                return;
            }

            set_display_message.set(msg);
            set_is_visible.set(true);

            // Auto-hide after 1.5 seconds
            set_timeout(
                move || {
                    set_is_visible.set(false);
                },
                std::time::Duration::from_millis(1500),
            );
        }
    });

    view! {
        <div
            class=move || format!(
                "fixed bottom-12 left-1/2 -translate-x-1/2 z-50 \
                 px-4 py-2 rounded-lg \
                 bg-black/80 text-white text-sm font-medium \
                 transition-opacity duration-300 \
                 pointer-events-none {}",
                if is_visible.get() { "opacity-100" } else { "opacity-0" }
            )
        >
            {move || display_message.get()}
        </div>
    }
}
```

**Step 2: Export from mod.rs**

Add to `fractalwonder-ui/src/components/mod.rs`:

```rust
mod toast;
```

And add to the `pub use` section:

```rust
pub use toast::Toast;
```

**Step 3: Verify it compiles**

Run: `cargo check --package fractalwonder-ui`
Expected: PASS (no errors)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/toast.rs
git add fractalwonder-ui/src/components/mod.rs
git commit -m "feat(components): add Toast notification component"
```

---

## Task 5: Add OptionsMenu Component

**Files:**
- Create: `fractalwonder-ui/src/components/options_menu.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create the options menu component**

Create `fractalwonder-ui/src/components/options_menu.rs`:

```rust
//! Options dropdown menu with grouped sections for Effects and Cycles.

use leptos::*;

#[component]
pub fn OptionsMenu(
    /// 3D shading enabled state
    shading_enabled: Signal<bool>,
    /// Callback when 3D is toggled
    on_shading_toggle: Callback<()>,
    /// Smooth iteration enabled state
    smooth_enabled: Signal<bool>,
    /// Callback when smooth is toggled
    on_smooth_toggle: Callback<()>,
    /// Current cycle count
    cycle_count: Signal<u32>,
    /// Callback to increase cycles
    on_cycle_up: Callback<()>,
    /// Callback to decrease cycles
    on_cycle_down: Callback<()>,
) -> impl IntoView {
    let (is_open, set_is_open) = create_signal(false);

    view! {
        <div class="relative">
            <button
                class="text-white hover:text-gray-200 hover:bg-white/10 rounded-lg px-3 py-2 transition-colors flex items-center gap-2"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                <span class="text-sm">"Options"</span>
                <span class="text-xs opacity-70">"▾"</span>
            </button>

            {move || is_open.get().then(|| view! {
                <div class="absolute bottom-full mb-2 left-0 min-w-48 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg overflow-hidden">
                    // Effects section
                    <div class="px-3 py-2 text-xs text-gray-400 uppercase tracking-wider border-b border-gray-800">
                        "Effects"
                    </div>
                    <button
                        class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center justify-between"
                        on:click=move |_| {
                            on_shading_toggle.call(());
                        }
                    >
                        <span class="flex items-center gap-2">
                            <span class=move || if shading_enabled.get() { "opacity-100" } else { "opacity-30" }>
                                {move || if shading_enabled.get() { "☑" } else { "☐" }}
                            </span>
                            "3D"
                        </span>
                        <span class="text-xs text-gray-500">"[3]"</span>
                    </button>
                    <button
                        class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center justify-between"
                        on:click=move |_| {
                            on_smooth_toggle.call(());
                        }
                    >
                        <span class="flex items-center gap-2">
                            <span class=move || if smooth_enabled.get() { "opacity-100" } else { "opacity-30" }>
                                {move || if smooth_enabled.get() { "☑" } else { "☐" }}
                            </span>
                            "Smooth"
                        </span>
                        <span class="text-xs text-gray-500">"[S]"</span>
                    </button>

                    // Cycles section
                    <div class="px-3 py-2 text-xs text-gray-400 uppercase tracking-wider border-t border-b border-gray-800">
                        "Cycles"
                    </div>
                    <div class="px-4 py-2 text-sm text-gray-300 flex items-center justify-between">
                        <div class="flex items-center gap-3">
                            <button
                                class="text-gray-400 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed"
                                on:click=move |_| on_cycle_down.call(())
                                disabled=move || cycle_count.get() <= 1
                            >
                                "◀"
                            </button>
                            <span class="min-w-8 text-center font-mono">{move || cycle_count.get()}</span>
                            <button
                                class="text-gray-400 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed"
                                on:click=move |_| on_cycle_up.call(())
                                disabled=move || cycle_count.get() >= 128
                            >
                                "▶"
                            </button>
                        </div>
                        <span class="text-xs text-gray-500">"[↑↓]"</span>
                    </div>
                </div>
            })}
        </div>
    }
}
```

**Step 2: Export from mod.rs**

Add to `fractalwonder-ui/src/components/mod.rs`:

```rust
mod options_menu;
```

And add to the `pub use` section:

```rust
pub use options_menu::OptionsMenu;
```

**Step 3: Verify it compiles**

Run: `cargo check --package fractalwonder-ui`
Expected: PASS

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/options_menu.rs
git add fractalwonder-ui/src/components/mod.rs
git commit -m "feat(components): add OptionsMenu with Effects and Cycles sections"
```

---

## Task 6: Update UIPanel with Palette and Options Menus

**Files:**
- Modify: `fractalwonder-ui/src/components/ui_panel.rs`

**Step 1: Update imports**

Add to imports:

```rust
use crate::components::OptionsMenu;
```

**Step 2: Update component props**

Replace the colorizer-related props with new color options props:

```rust
#[component]
pub fn UIPanel(
    /// Current viewport in fractal space
    viewport: Signal<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Calculated precision bits
    precision_bits: Signal<usize>,
    /// Callback when home button is clicked
    on_home_click: Callback<()>,
    /// Palette options (id, display_name)
    palette_options: Signal<Vec<(String, String)>>,
    /// Currently selected palette ID
    selected_palette_id: Signal<String>,
    /// Callback when palette is selected
    on_palette_select: Callback<String>,
    /// 3D shading enabled
    shading_enabled: Signal<bool>,
    /// Callback to toggle 3D
    on_shading_toggle: Callback<()>,
    /// Smooth iteration enabled
    smooth_enabled: Signal<bool>,
    /// Callback to toggle smooth
    on_smooth_toggle: Callback<()>,
    /// Cycle count
    cycle_count: Signal<u32>,
    /// Callback to increase cycles
    on_cycle_up: Callback<()>,
    /// Callback to decrease cycles
    on_cycle_down: Callback<()>,
    /// Render progress signal
    render_progress: Signal<RwSignal<RenderProgress>>,
    /// UI visibility signal (from parent)
    is_visible: ReadSignal<bool>,
    /// Set hovering state (from parent)
    set_is_hovering: WriteSignal<bool>,
    /// Callback to cancel current render
    on_cancel: Callback<()>,
    /// X-ray mode enabled state
    xray_enabled: ReadSignal<bool>,
    /// Callback to toggle x-ray mode
    set_xray_enabled: WriteSignal<bool>,
) -> impl IntoView {
```

**Step 3: Update the view**

Replace the Colors dropdown with Palette and Options menus in the left section:

```rust
// Left section: info button, home button, and menus
<div class="flex items-center space-x-2">
    <InfoButton
        is_open=is_info_open
        set_is_open=set_is_info_open
        xray_enabled=xray_enabled
        set_xray_enabled=set_xray_enabled
    />
    <HomeButton on_click=on_home_click />
    <DropdownMenu
        label="Palette".to_string()
        options=palette_options
        selected_id=selected_palette_id
        on_select=move |id| on_palette_select.call(id)
    />
    <OptionsMenu
        shading_enabled=shading_enabled
        on_shading_toggle=on_shading_toggle
        smooth_enabled=smooth_enabled
        on_smooth_toggle=on_smooth_toggle
        cycle_count=cycle_count
        on_cycle_up=on_cycle_up
        on_cycle_down=on_cycle_down
    />
</div>
```

**Step 4: Verify it compiles**

Run: `cargo check --package fractalwonder-ui`
Expected: FAIL (App.rs needs updating - that's Task 7)

**Step 5: Commit (partial - will complete after Task 7)**

Wait for Task 7 before committing.

---

## Task 7: Update App.rs with ColorOptions State

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Update imports**

Add/update imports:

```rust
use crate::components::{CircularProgress, InteractiveCanvas, Toast, UIPanel};
use crate::hooks::{
    load_state, save_state, use_hashchange_listener, use_ui_visibility, PersistedState,
};
use crate::rendering::colorizers::{palettes, ColorOptions};
use crate::rendering::RenderProgress;
```

**Step 2: Replace color scheme state with color options**

Replace the color scheme signals:

```rust
// Load persisted state from localStorage (if any)
let persisted = load_state();

// Extract persisted values before moving into closures
let initial_config_id = persisted
    .as_ref()
    .map(|s| s.config_id.clone())
    .unwrap_or_else(|| "mandelbrot".to_string());
let initial_color_options = persisted
    .as_ref()
    .map(|s| s.color_options.clone())
    .unwrap_or_default();
let persisted_viewport = persisted.map(|s| s.viewport);

// ... existing viewport setup code ...

// Color options state
let (color_options, set_color_options) = create_signal(initial_color_options);

// Derive individual signals for UI components
let palette_id = create_memo(move |_| color_options.get().palette_id.clone());
let shading_enabled = create_memo(move |_| color_options.get().shading_enabled);
let smooth_enabled = create_memo(move |_| color_options.get().smooth_enabled);
let cycle_count = create_memo(move |_| color_options.get().cycle_count);

// Palette options for dropdown
let palette_options = Signal::derive(move || {
    palettes()
        .iter()
        .map(|p| (p.id.to_string(), p.name.to_string()))
        .collect::<Vec<_>>()
});

// Toast message signal
let (toast_message, set_toast_message) = create_signal::<Option<String>>(None);
```

**Step 3: Update persistence effect**

Update the save effect:

```rust
// Persist state to localStorage when viewport or color options change
create_effect(move |_| {
    let vp = viewport.get();
    let config_id = selected_config_id.get();
    let options = color_options.get();

    // Skip saving if viewport hasn't been initialized yet
    if vp.width.to_f64() == 4.0 && vp.height.to_f64() == 3.0 {
        return;
    }

    let state = PersistedState::new(vp, config_id, options);
    save_state(&state);
});
```

**Step 4: Update keyboard handler**

Replace the keyboard handler with new shortcuts:

```rust
let keyboard_handler = store_value::<Option<Closure<dyn FnMut(web_sys::KeyboardEvent)>>>(None);

create_effect(move |_| {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    let handler = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
        // Skip if typing in an input field
        if let Some(target) = e.target() {
            if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                let tag = element.tag_name().to_lowercase();
                if tag == "input" || tag == "textarea" {
                    return;
                }
            }
        }

        match e.key().as_str() {
            "x" | "X" => {
                // Toggle x-ray mode
                set_xray_enabled.update(|v| {
                    *v = !*v;
                    let msg = if *v { "X-ray: On" } else { "X-ray: Off" };
                    set_toast_message.set(Some(msg.to_string()));
                });
            }
            "d" | "D" => {
                // Subdivide quadtree (only when x-ray enabled)
                if xray_enabled.get_untracked() {
                    set_subdivide_trigger.update(|v| *v = v.wrapping_add(1));
                }
            }
            "3" => {
                // Toggle 3D shading
                set_color_options.update(|opts| {
                    opts.shading_enabled = !opts.shading_enabled;
                    let msg = if opts.shading_enabled { "3D: On" } else { "3D: Off" };
                    set_toast_message.set(Some(msg.to_string()));
                });
            }
            "s" | "S" => {
                // Toggle smooth iteration
                set_color_options.update(|opts| {
                    opts.smooth_enabled = !opts.smooth_enabled;
                    let msg = if opts.smooth_enabled { "Smooth: On" } else { "Smooth: Off" };
                    set_toast_message.set(Some(msg.to_string()));
                });
            }
            "ArrowLeft" => {
                // Previous palette
                let opts = palettes();
                let current_id = color_options.get_untracked().palette_id;
                let current_idx = opts.iter().position(|p| p.id == current_id).unwrap_or(0);
                let new_idx = if current_idx == 0 { opts.len() - 1 } else { current_idx - 1 };
                let new_palette = &opts[new_idx];
                set_color_options.update(|o| o.palette_id = new_palette.id.to_string());
                set_toast_message.set(Some(format!("Palette: {}", new_palette.name)));
            }
            "ArrowRight" => {
                // Next palette
                let opts = palettes();
                let current_id = color_options.get_untracked().palette_id;
                let current_idx = opts.iter().position(|p| p.id == current_id).unwrap_or(0);
                let new_idx = (current_idx + 1) % opts.len();
                let new_palette = &opts[new_idx];
                set_color_options.update(|o| o.palette_id = new_palette.id.to_string());
                set_toast_message.set(Some(format!("Palette: {}", new_palette.name)));
            }
            "ArrowUp" => {
                // Increase cycle count
                set_color_options.update(|opts| {
                    opts.cycle_up();
                    set_toast_message.set(Some(format!("Cycles: {}", opts.cycle_count)));
                });
            }
            "ArrowDown" => {
                // Decrease cycle count
                set_color_options.update(|opts| {
                    opts.cycle_down();
                    set_toast_message.set(Some(format!("Cycles: {}", opts.cycle_count)));
                });
            }
            _ => {}
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    if let Some(window) = web_sys::window() {
        let _ = window
            .add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref());
    }

    keyboard_handler.set_value(Some(handler));

    on_cleanup(move || {
        keyboard_handler.with_value(|handler_opt| {
            if let Some(handler) = handler_opt {
                if let Some(window) = web_sys::window() {
                    let _ = window.remove_event_listener_with_callback(
                        "keydown",
                        handler.as_ref().unchecked_ref(),
                    );
                }
            }
        });
        keyboard_handler.set_value(None);
    });
});
```

**Step 5: Update hashchange listener**

```rust
use_hashchange_listener(move |state| {
    let size = canvas_size.get_untracked();
    if size.0 > 0 && size.1 > 0 {
        let fitted = fit_viewport_to_canvas(&state.viewport, size);
        set_viewport.set(fitted);
        set_color_options.set(state.color_options.clone());
        log::info!("Restored viewport and color options from URL hash change");
    }
});
```

**Step 6: Update view with new props**

Remove the `on_color_schemes` callback and update InteractiveCanvas:

```rust
<InteractiveCanvas
    viewport=viewport.into()
    on_viewport_change=on_viewport_change
    config=config.into()
    on_resize=on_resize
    on_progress_signal=on_progress_signal
    cancel_trigger=cancel_trigger
    subdivide_trigger=subdivide_trigger
    xray_enabled=xray_enabled
    color_options=color_options.into()
/>
```

Update UIPanel:

```rust
<UIPanel
    viewport=viewport.into()
    config=config.into()
    precision_bits=precision_bits.into()
    on_home_click=on_home_click
    palette_options=palette_options
    selected_palette_id=Signal::derive(move || palette_id.get())
    on_palette_select=Callback::new(move |id: String| {
        let name = palettes().iter().find(|p| p.id == id).map(|p| p.name).unwrap_or("Unknown");
        set_color_options.update(|o| o.palette_id = id);
        set_toast_message.set(Some(format!("Palette: {}", name)));
    })
    shading_enabled=Signal::derive(move || shading_enabled.get())
    on_shading_toggle=Callback::new(move |_| {
        set_color_options.update(|opts| {
            opts.shading_enabled = !opts.shading_enabled;
            let msg = if opts.shading_enabled { "3D: On" } else { "3D: Off" };
            set_toast_message.set(Some(msg.to_string()));
        });
    })
    smooth_enabled=Signal::derive(move || smooth_enabled.get())
    on_smooth_toggle=Callback::new(move |_| {
        set_color_options.update(|opts| {
            opts.smooth_enabled = !opts.smooth_enabled;
            let msg = if opts.smooth_enabled { "Smooth: On" } else { "Smooth: Off" };
            set_toast_message.set(Some(msg.to_string()));
        });
    })
    cycle_count=Signal::derive(move || cycle_count.get())
    on_cycle_up=Callback::new(move |_| {
        set_color_options.update(|opts| {
            opts.cycle_up();
            set_toast_message.set(Some(format!("Cycles: {}", opts.cycle_count)));
        });
    })
    on_cycle_down=Callback::new(move |_| {
        set_color_options.update(|opts| {
            opts.cycle_down();
            set_toast_message.set(Some(format!("Cycles: {}", opts.cycle_count)));
        });
    })
    render_progress=render_progress.into()
    is_visible=ui_visibility.is_visible
    set_is_hovering=ui_visibility.set_is_hovering
    on_cancel=on_cancel
    xray_enabled=xray_enabled
    set_xray_enabled=set_xray_enabled
/>
<Toast
    message=Signal::derive(move || toast_message.get())
    ui_visible=ui_visibility.is_visible.into()
/>
<CircularProgress
    progress=render_progress.into()
    is_ui_visible=ui_visibility.is_visible
/>
```

**Step 7: Verify it compiles**

Run: `cargo check --package fractalwonder-ui`
Expected: FAIL (InteractiveCanvas needs updating - Task 8)

---

## Task 8: Update InteractiveCanvas for ColorOptions

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Update imports**

```rust
use crate::rendering::colorizers::ColorOptions;
```

**Step 2: Update component props**

Replace color scheme props with color options:

```rust
#[component]
pub fn InteractiveCanvas(
    /// Current viewport in fractal space (read-only)
    viewport: Signal<Viewport>,
    /// Callback fired when user interaction ends with a new viewport
    on_viewport_change: Callback<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Callback fired when canvas dimensions change
    #[prop(optional)]
    on_resize: Option<Callback<(u32, u32)>>,
    /// Callback fired with progress signal when renderer is created
    #[prop(optional)]
    on_progress_signal: Option<Callback<RwSignal<crate::rendering::RenderProgress>>>,
    /// Signal that triggers render cancellation when incremented
    #[prop(optional)]
    cancel_trigger: Option<ReadSignal<u32>>,
    /// Signal that triggers quadtree subdivision when incremented
    #[prop(optional)]
    subdivide_trigger: Option<ReadSignal<u32>>,
    /// X-ray mode enabled signal
    #[prop(optional)]
    xray_enabled: Option<ReadSignal<bool>>,
    /// Color options signal
    #[prop(optional)]
    color_options: Option<Signal<ColorOptions>>,
) -> impl IntoView {
```

**Step 3: Remove old color scheme handling**

Remove these sections:
- `on_color_schemes` prop and callback
- `selected_color_scheme` prop
- The effect that watches `selected_color_scheme`

**Step 4: Add color options effect**

Replace with new effect:

```rust
// Watch for color options changes - update renderer and recolorize
if let Some(options_signal) = color_options {
    create_effect(move |prev: Option<ColorOptions>| {
        let options = options_signal.get();

        renderer.with_value(|r| r.set_color_options(&options));

        // Recolorize when options change (not on initial mount)
        if prev.is_some() && prev.as_ref() != Some(&options) {
            renderer.with_value(|r| r.recolorize());
        }

        options
    });
}
```

**Step 5: Verify it compiles**

Run: `cargo check --package fractalwonder-ui`
Expected: FAIL (ParallelRenderer needs `set_color_options` - Task 9)

---

## Task 9: Update ParallelRenderer for ColorOptions

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Update imports**

Add to imports:

```rust
use crate::rendering::colorizers::ColorOptions;
```

**Step 2: Add smooth_enabled to state**

Add new field to `ParallelRenderer` struct:

```rust
/// Whether smooth iteration is enabled
smooth_enabled: Rc<Cell<bool>>,
```

Initialize in `new()`:

```rust
let smooth_enabled: Rc<Cell<bool>> = Rc::new(Cell::new(true));
```

And add to the returned struct.

**Step 3: Add set_color_options method**

Add new method:

```rust
/// Set color options from UI.
pub fn set_color_options(&self, options: &ColorOptions) {
    let settings = options.to_color_settings();
    *self.settings.borrow_mut() = settings;
    self.smooth_enabled.set(options.smooth_enabled);
}
```

**Step 4: Update colorization to respect smooth_enabled**

Update `on_tile_complete` closure in `new()` to check smooth_enabled:

```rust
let smooth_enabled_clone = Rc::clone(&smooth_enabled);
let on_tile_complete = move |result: TileResult| {
    if let Some(ctx) = ctx_clone.borrow().as_ref() {
        let xray = xray_clone.get();
        let s = settings_clone.borrow();
        let col = colorizer_clone.borrow();
        let use_smooth = smooth_enabled_clone.get();

        let pixels: Vec<u8> = result
            .data
            .iter()
            .flat_map(|d| {
                if use_smooth {
                    colorize_with_palette(d, &s, &col, xray)
                } else {
                    colorize_discrete(d, &s, xray)
                }
            })
            .collect();

        // ... rest unchanged
    }
};
```

**Step 5: Add colorize_discrete function**

Add to `fractalwonder-ui/src/rendering/colorizers/mod.rs`:

```rust
/// Colorize using discrete iteration count (no smooth interpolation).
pub fn colorize_discrete(
    data: &ComputeData,
    settings: &ColorSettings,
    xray_enabled: bool,
) -> [u8; 4] {
    // Handle xray mode for glitched pixels
    if xray_enabled {
        if let ComputeData::Mandelbrot(m) = data {
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

    match data {
        ComputeData::Mandelbrot(m) => {
            if !m.escaped {
                return [0, 0, 0, 255];
            }
            if m.max_iterations == 0 {
                return [0, 0, 0, 255];
            }
            let normalized = m.iterations as f64 / m.max_iterations as f64;
            let t = (normalized * settings.cycle_count).fract();
            let [r, g, b] = settings.palette.sample(t);
            [r, g, b, 255]
        }
        ComputeData::TestImage(_) => [128, 128, 128, 255],
    }
}
```

**Step 6: Update recolorize to respect smooth_enabled**

Update `recolorize()`:

```rust
pub fn recolorize(&self) {
    let settings = self.settings.borrow();
    let colorizer = self.colorizer.borrow();
    let ctx_ref = self.canvas_ctx.borrow();
    let Some(ctx) = ctx_ref.as_ref() else {
        return;
    };
    let use_smooth = self.smooth_enabled.get();

    // Compute zoom level from stored viewport
    let zoom_level = if let Some(ref viewport) = *self.current_viewport.borrow() {
        let reference_width = self
            .config
            .default_viewport(viewport.precision_bits())
            .width;
        reference_width.to_f64() / viewport.width.to_f64()
    } else {
        1.0
    };

    for result in self.tile_results.borrow().iter() {
        let pixels = if use_smooth {
            colorizer.run_pipeline(
                &result.data,
                &settings,
                result.tile.width as usize,
                result.tile.height as usize,
                zoom_level,
            )
        } else {
            // Discrete coloring - no pipeline, just direct color mapping
            result
                .data
                .iter()
                .map(|d| colorize_discrete(d, &settings, self.xray_enabled.get()))
                .collect()
        };
        let pixel_bytes: Vec<u8> = pixels.into_iter().flatten().collect();
        let _ = draw_pixels_to_canvas(
            ctx,
            &pixel_bytes,
            result.tile.width,
            result.tile.x as f64,
            result.tile.y as f64,
        );
    }
}
```

**Step 7: Verify it compiles**

Run: `cargo check --package fractalwonder-ui`
Expected: PASS

**Step 8: Run all tests**

Run: `cargo test --package fractalwonder-ui`
Expected: PASS

**Step 9: Commit all UI changes**

```bash
git add fractalwonder-ui/src/components/ui_panel.rs
git add fractalwonder-ui/src/components/interactive_canvas.rs
git add fractalwonder-ui/src/app.rs
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git add fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(ui): wire ColorOptions through UI, renderer, and persistence"
```

---

## Task 10: Run Quality Checks

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 4: Check WASM build**

Run: `cargo check --package fractalwonder-ui --target wasm32-unknown-unknown`
Expected: PASS

**Step 5: Manual browser test**

With `trunk serve` running:
1. Open http://localhost:8080
2. Test palette switching with left/right arrows
3. Test cycle count with up/down arrows
4. Test 3D toggle with '3' key
5. Test smooth toggle with 's' key
6. Verify toast appears only when UI hidden
7. Verify settings persist after page reload

**Step 6: Final commit**

```bash
git add -A
git commit -m "chore: format and lint fixes"
```

---

## Task 11: Clean Up Old Preset System (Optional)

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/presets.rs`
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

If presets are no longer used anywhere:

**Step 1: Remove presets.rs**

Delete `fractalwonder-ui/src/rendering/colorizers/presets.rs`

**Step 2: Update mod.rs**

Remove:
```rust
pub mod presets;
pub use presets::{presets, ColorSchemePreset};
```

**Step 3: Update parallel_renderer.rs**

Remove `color_scheme_presets()` method if unused.

**Step 4: Verify nothing breaks**

Run: `cargo test --workspace`
Expected: PASS

**Step 5: Commit cleanup**

```bash
git add -A
git commit -m "refactor(colorizers): remove unused preset system"
```
