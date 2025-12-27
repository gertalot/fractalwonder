# Light Effects Section Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add the "Light Effects" collapsible section to the palette editor with 3D lighting toggle, falloff curve, lighting parameter sliders, and circular light direction control.

**Architecture:** Create two new components (LightingSlider, LightingControl), modify CurveEditor to remove hardcoded label, then integrate everything into PaletteEditor's Light Effects section with proper signal wiring.

**Tech Stack:** Rust, Leptos 0.6, web-sys for canvas/mouse events, Tailwind CSS

---

## Task 1: Create LightingSlider Component

**Files:**
- Create: `fractalwonder-ui/src/components/lighting_slider.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create the component file with basic structure**

Create `fractalwonder-ui/src/components/lighting_slider.rs`:

```rust
//! Reusable slider component for lighting parameters.

use leptos::*;

/// Slider with label and value display for lighting parameters.
#[component]
pub fn LightingSlider(
    /// Label text displayed on the left
    label: &'static str,
    /// Current value signal
    value: Signal<f64>,
    /// Called when value changes
    on_change: Callback<f64>,
    /// Minimum value
    min: f64,
    /// Maximum value
    max: f64,
    /// Step increment
    step: f64,
    /// Decimal places for value display
    #[prop(default = 2)]
    precision: u8,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <div class="text-white text-xs w-20">{label}</div>
            <input
                type="range"
                class="flex-1 accent-white"
                prop:min=min
                prop:max=max
                prop:step=step
                prop:value=move || value.get()
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                        on_change.call(v);
                    }
                }
            />
            <div class="text-white text-xs w-10 text-right">
                {move || format!("{:.prec$}", value.get(), prec = precision as usize)}
            </div>
        </div>
    }
}
```

**Step 2: Register the module**

In `fractalwonder-ui/src/components/mod.rs`, add after line 8 (`mod home_button;`):

```rust
mod lighting_slider;
```

Add to exports after line 26 (`pub use home_button::HomeButton;`):

```rust
pub use lighting_slider::LightingSlider;
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui`

Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/lighting_slider.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(palette-editor): add LightingSlider component"
```

---

## Task 2: Create LightingControl Component - Basic Structure

**Files:**
- Create: `fractalwonder-ui/src/components/lighting_control.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create component with static rendering (no drag yet)**

Create `fractalwonder-ui/src/components/lighting_control.rs`:

```rust
//! Circular light direction control for azimuth and elevation.

use leptos::*;
use std::f64::consts::{FRAC_PI_2, PI};

/// Circular picker for light direction.
///
/// Azimuth and elevation are in radians. Display shows degrees.
#[component]
pub fn LightingControl(
    /// Light azimuth in radians (0 = top, clockwise)
    azimuth: Signal<f64>,
    /// Light elevation in radians (0 = horizon, PI/2 = overhead)
    elevation: Signal<f64>,
    /// Called when direction changes (azimuth, elevation) in radians
    on_change: Callback<(f64, f64)>,
) -> impl IntoView {
    let circle_ref = create_node_ref::<leptos::html::Div>();

    // Convert radians to position percentage
    let position = Signal::derive(move || {
        let az = azimuth.get();
        let el = elevation.get();

        // Angle from top, clockwise
        let angle = az - FRAC_PI_2;
        // Radius: center = 0%, edge = 50%
        let radius_pct = (1.0 - el / FRAC_PI_2) * 50.0;

        let x = 50.0 + radius_pct * angle.cos();
        let y = 50.0 + radius_pct * angle.sin();
        (x, y)
    });

    // Display in degrees
    let azimuth_deg = Signal::derive(move || (azimuth.get() * 180.0 / PI).round() as i32);
    let elevation_deg = Signal::derive(move || (elevation.get() * 180.0 / PI).round() as i32);

    view! {
        <div class="bg-white/5 border border-white/10 rounded-lg p-3 space-y-3">
            <div
                node_ref=circle_ref
                class="relative w-full aspect-square bg-white/5 rounded-full border border-white/20 cursor-crosshair"
            >
                // Center dot
                <div class="absolute top-1/2 left-1/2 w-2 h-2 bg-white/30 rounded-full -translate-x-1/2 -translate-y-1/2" />

                // Concentric guide circles
                {[0.25, 0.5, 0.75, 1.0].into_iter().map(|r| {
                    let size = format!("{}%", r * 100.0);
                    view! {
                        <div
                            class="absolute top-1/2 left-1/2 border border-white/10 rounded-full -translate-x-1/2 -translate-y-1/2"
                            style:width=size.clone()
                            style:height=size
                        />
                    }
                }).collect_view()}

                // Light position indicator
                <div
                    class="absolute w-4 h-4 bg-white rounded-full shadow-lg -translate-x-1/2 -translate-y-1/2"
                    style:left=move || format!("{}%", position.get().0)
                    style:top=move || format!("{}%", position.get().1)
                />
            </div>

            // Azimuth/Elevation display
            <div class="grid grid-cols-2 gap-2 text-xs">
                <div class="space-y-0.5">
                    <div class="text-white/70">"Azimuth"</div>
                    <div class="text-white">{move || format!("{}°", azimuth_deg.get())}</div>
                </div>
                <div class="space-y-0.5">
                    <div class="text-white/70">"Elevation"</div>
                    <div class="text-white">{move || format!("{}°", elevation_deg.get())}</div>
                </div>
            </div>

            <div class="text-white/50 text-xs">
                "Drag to adjust light direction"
            </div>
        </div>
    }
}
```

**Step 2: Register the module**

In `fractalwonder-ui/src/components/mod.rs`, add after `mod lighting_slider;`:

```rust
mod lighting_control;
```

Add to exports after `pub use lighting_slider::LightingSlider;`:

```rust
pub use lighting_control::LightingControl;
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui`

Expected: Compiles without errors (warning about unused `on_change` and `circle_ref` is OK for now)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/lighting_control.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(palette-editor): add LightingControl component (static rendering)"
```

---

## Task 3: Add Drag Interaction to LightingControl

**Files:**
- Modify: `fractalwonder-ui/src/components/lighting_control.rs`

**Step 1: Add drag state and mouse handlers**

Replace the entire `lighting_control.rs` with:

```rust
//! Circular light direction control for azimuth and elevation.

use leptos::*;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Circular picker for light direction.
///
/// Azimuth and elevation are in radians. Display shows degrees.
#[component]
pub fn LightingControl(
    /// Light azimuth in radians (0 = top, clockwise)
    azimuth: Signal<f64>,
    /// Light elevation in radians (0 = horizon, PI/2 = overhead)
    elevation: Signal<f64>,
    /// Called when direction changes (azimuth, elevation) in radians
    on_change: Callback<(f64, f64)>,
) -> impl IntoView {
    let circle_ref = create_node_ref::<leptos::html::Div>();
    let is_dragging = create_rw_signal(false);

    // Calculate azimuth/elevation from mouse position
    let calculate_from_mouse = move |client_x: i32, client_y: i32| {
        let Some(circle) = circle_ref.get() else { return };
        let rect = circle.get_bounding_client_rect();

        let center_x = rect.left() + rect.width() / 2.0;
        let center_y = rect.top() + rect.height() / 2.0;
        let radius = rect.width() / 2.0;

        let dx = (client_x as f64 - center_x) / radius;
        let dy = (client_y as f64 - center_y) / radius;

        // Azimuth: atan2 + 90° offset (so 0 = top)
        let mut new_azimuth = dy.atan2(dx) + FRAC_PI_2;
        if new_azimuth < 0.0 {
            new_azimuth += TAU;
        }

        // Elevation: center = PI/2, edge = 0
        let distance = (dx * dx + dy * dy).sqrt().min(1.0);
        let new_elevation = FRAC_PI_2 * (1.0 - distance);

        on_change.call((new_azimuth, new_elevation));
    };

    // Document-level mouse handlers
    create_effect(move |_| {
        let window = web_sys::window().expect("window");
        let document = window.document().expect("document");

        let mousemove_closure =
            Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                if is_dragging.get() {
                    calculate_from_mouse(e.client_x(), e.client_y());
                }
            });

        let mouseup_closure =
            Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
                is_dragging.set(false);
            });

        let _ = document.add_event_listener_with_callback(
            "mousemove",
            mousemove_closure.as_ref().unchecked_ref(),
        );
        let _ = document.add_event_listener_with_callback(
            "mouseup",
            mouseup_closure.as_ref().unchecked_ref(),
        );

        mousemove_closure.forget();
        mouseup_closure.forget();
    });

    // Convert radians to position percentage
    let position = Signal::derive(move || {
        let az = azimuth.get();
        let el = elevation.get();

        let angle = az - FRAC_PI_2;
        let radius_pct = (1.0 - el / FRAC_PI_2) * 50.0;

        let x = 50.0 + radius_pct * angle.cos();
        let y = 50.0 + radius_pct * angle.sin();
        (x, y)
    });

    // Display in degrees
    let azimuth_deg = Signal::derive(move || (azimuth.get() * 180.0 / PI).round() as i32);
    let elevation_deg = Signal::derive(move || (elevation.get() * 180.0 / PI).round() as i32);

    view! {
        <div class="bg-white/5 border border-white/10 rounded-lg p-3 space-y-3">
            <div
                node_ref=circle_ref
                class="relative w-full aspect-square bg-white/5 rounded-full border border-white/20 cursor-crosshair"
                on:mousedown=move |e| {
                    e.prevent_default();
                    is_dragging.set(true);
                    calculate_from_mouse(e.client_x(), e.client_y());
                }
            >
                // Center dot
                <div class="absolute top-1/2 left-1/2 w-2 h-2 bg-white/30 rounded-full -translate-x-1/2 -translate-y-1/2" />

                // Concentric guide circles
                {[0.25, 0.5, 0.75, 1.0].into_iter().map(|r| {
                    let size = format!("{}%", r * 100.0);
                    view! {
                        <div
                            class="absolute top-1/2 left-1/2 border border-white/10 rounded-full -translate-x-1/2 -translate-y-1/2"
                            style:width=size.clone()
                            style:height=size
                        />
                    }
                }).collect_view()}

                // Light position indicator
                <div
                    class="absolute w-4 h-4 bg-white rounded-full shadow-lg -translate-x-1/2 -translate-y-1/2"
                    style:left=move || format!("{}%", position.get().0)
                    style:top=move || format!("{}%", position.get().1)
                />
            </div>

            // Azimuth/Elevation display
            <div class="grid grid-cols-2 gap-2 text-xs">
                <div class="space-y-0.5">
                    <div class="text-white/70">"Azimuth"</div>
                    <div class="text-white">{move || format!("{}°", azimuth_deg.get())}</div>
                </div>
                <div class="space-y-0.5">
                    <div class="text-white/70">"Elevation"</div>
                    <div class="text-white">{move || format!("{}°", elevation_deg.get())}</div>
                </div>
            </div>

            <div class="text-white/50 text-xs">
                "Drag to adjust light direction"
            </div>
        </div>
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check --package fractalwonder-ui`

Expected: Compiles without errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/lighting_control.rs
git commit -m "feat(palette-editor): add drag interaction to LightingControl"
```

---

## Task 4: Remove Hardcoded Label from CurveEditor

**Files:**
- Modify: `fractalwonder-ui/src/components/curve_editor.rs`

**Step 1: Remove the label div**

In `fractalwonder-ui/src/components/curve_editor.rs`, find line 118:

```rust
<div class="text-white/50 text-xs mb-2">"Transfer Curve"</div>
```

Delete this line.

**Step 2: Verify compilation**

Run: `cargo check --package fractalwonder-ui`

Expected: Compiles without errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/curve_editor.rs
git commit -m "refactor(curve-editor): remove hardcoded label for reusability"
```

---

## Task 5: Add Light Effects Section to PaletteEditor - Signals

**Files:**
- Modify: `fractalwonder-ui/src/components/palette_editor.rs`

**Step 1: Add imports**

At the top of `palette_editor.rs`, update the imports from:

```rust
use crate::components::{
    CollapsibleSection, ConfirmDialog, CurveEditor, EditMode, GradientEditor, PaletteEditorState,
};
use crate::rendering::colorizers::{Curve, Gradient, Palette};
```

To:

```rust
use crate::components::{
    CollapsibleSection, ConfirmDialog, CurveEditor, EditMode, GradientEditor, LightingControl,
    LightingSlider, PaletteEditorState,
};
use crate::rendering::colorizers::{Curve, Gradient, Palette};
```

**Step 2: Add collapsible section state**

After line 39 (`let palette_expanded = create_rw_signal(true);`), add:

```rust
let light_effects_expanded = create_rw_signal(true);
```

**Step 3: Add shading_enabled signal**

After line 91 (the `smooth_enabled` signal), add:

```rust
let shading_enabled = Signal::derive(move || {
    state
        .get()
        .map(|s| s.working_palette.shading_enabled)
        .unwrap_or(false)
});
```

**Step 4: Add falloff curve signal and callback**

After line 120 (the `on_transfer_curve_change` callback), add:

```rust
// Derived: falloff curve
let falloff_curve_signal = Signal::derive(move || {
    state
        .get()
        .map(|s| s.working_palette.falloff_curve.clone())
});

// Callback for falloff curve changes
let on_falloff_curve_change = Callback::new(move |new_curve: Curve| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.falloff_curve = new_curve;
        }
    });
});
```

**Step 5: Add lighting parameter signals**

After the falloff curve callback, add:

```rust
// Derived: lighting parameters
let ambient = Signal::derive(move || {
    state.get().map(|s| s.working_palette.lighting.ambient).unwrap_or(0.0)
});
let diffuse = Signal::derive(move || {
    state.get().map(|s| s.working_palette.lighting.diffuse).unwrap_or(0.0)
});
let specular = Signal::derive(move || {
    state.get().map(|s| s.working_palette.lighting.specular).unwrap_or(0.0)
});
let shininess = Signal::derive(move || {
    state.get().map(|s| s.working_palette.lighting.shininess).unwrap_or(1.0)
});
let strength = Signal::derive(move || {
    state.get().map(|s| s.working_palette.lighting.strength).unwrap_or(0.0)
});
let azimuth = Signal::derive(move || {
    state.get().map(|s| s.working_palette.lighting.azimuth).unwrap_or(0.0)
});
let elevation = Signal::derive(move || {
    state.get().map(|s| s.working_palette.lighting.elevation).unwrap_or(0.0)
});
```

**Step 6: Add lighting parameter callbacks**

After the lighting parameter signals, add:

```rust
// Callbacks for lighting parameters
let on_ambient_change = Callback::new(move |value: f64| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.lighting.ambient = value;
        }
    });
});
let on_diffuse_change = Callback::new(move |value: f64| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.lighting.diffuse = value;
        }
    });
});
let on_specular_change = Callback::new(move |value: f64| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.lighting.specular = value;
        }
    });
});
let on_shininess_change = Callback::new(move |value: f64| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.lighting.shininess = value;
        }
    });
});
let on_strength_change = Callback::new(move |value: f64| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.lighting.strength = value;
        }
    });
});
let on_direction_change = Callback::new(move |(az, el): (f64, f64)| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.lighting.azimuth = az;
            s.working_palette.lighting.elevation = el;
        }
    });
});
```

**Step 7: Verify compilation**

Run: `cargo check --package fractalwonder-ui`

Expected: Compiles with warnings about unused variables (they'll be used in next task)

**Step 8: Commit**

```bash
git add fractalwonder-ui/src/components/palette_editor.rs
git commit -m "feat(palette-editor): add signals and callbacks for Light Effects section"
```

---

## Task 6: Add Light Effects Section to PaletteEditor - View

**Files:**
- Modify: `fractalwonder-ui/src/components/palette_editor.rs`

**Step 1: Add labels before curve editors in Palette section**

Find the GradientEditor and CurveEditor in the Palette section (around line 403-413). Update to add labels:

```rust
// Gradient editor
<div class="text-white/50 text-xs px-1">"Color Gradient"</div>
<GradientEditor
    gradient=gradient_signal
    on_change=on_gradient_change
/>

// Transfer curve editor
<div class="text-white/50 text-xs px-1">"Transfer Curve"</div>
<CurveEditor
    curve=transfer_curve_signal
    on_change=on_transfer_curve_change
/>
```

**Step 2: Add Light Effects section after Palette section**

After the closing `</CollapsibleSection>` for Palette (around line 414), add:

```rust
// Light Effects Section
<CollapsibleSection title="Light Effects" expanded=light_effects_expanded>
    // 3D Lighting toggle
    <div class="space-y-1">
        <label class="flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 \
                      cursor-pointer transition-colors">
            <input
                type="checkbox"
                class="w-3.5 h-3.5 rounded accent-white"
                prop:checked=move || shading_enabled.get()
                on:change=move |ev| {
                    let checked = event_target_checked(&ev);
                    state.update(|opt| {
                        if let Some(s) = opt {
                            s.working_palette.shading_enabled = checked;
                        }
                    });
                }
            />
            <span class="text-white text-sm">"3D Lighting"</span>
        </label>
    </div>

    // Conditional content when 3D enabled
    <Show when=move || shading_enabled.get()>
        // Falloff Curve
        <div class="text-white/50 text-xs px-1">"3D Falloff Curve"</div>
        <CurveEditor
            curve=falloff_curve_signal
            on_change=on_falloff_curve_change
        />

        // Lighting Parameters
        <div class="text-white/50 text-xs px-1">"Lighting Parameters"</div>
        <div class="space-y-2">
            <LightingSlider
                label="Ambient"
                value=ambient
                on_change=on_ambient_change
                min=0.0
                max=1.0
                step=0.01
                precision=2
            />
            <LightingSlider
                label="Diffuse"
                value=diffuse
                on_change=on_diffuse_change
                min=0.0
                max=1.0
                step=0.01
                precision=2
            />
            <LightingSlider
                label="Specular"
                value=specular
                on_change=on_specular_change
                min=0.0
                max=1.0
                step=0.01
                precision=2
            />
            <LightingSlider
                label="Shininess"
                value=shininess
                on_change=on_shininess_change
                min=1.0
                max=128.0
                step=1.0
                precision=0
            />
            <LightingSlider
                label="Strength"
                value=strength
                on_change=on_strength_change
                min=0.0
                max=2.0
                step=0.01
                precision=2
            />
        </div>

        // Light Direction
        <div class="text-white/50 text-xs px-1">"Light Direction"</div>
        <LightingControl
            azimuth=azimuth
            elevation=elevation
            on_change=on_direction_change
        />
    </Show>
</CollapsibleSection>
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder-ui`

Expected: Compiles without errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/palette_editor.rs
git commit -m "feat(palette-editor): add Light Effects section with full UI"
```

---

## Task 7: Run Full Test Suite

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Expected: No errors or warnings

**Step 3: Run tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: All tests pass

**Step 4: Build WASM**

Run: `cargo check --package fractalwonder-ui --target wasm32-unknown-unknown`

Expected: Compiles without errors

**Step 5: Commit any formatting fixes**

```bash
git add -A
git commit -m "style: format code" --allow-empty
```

---

## Task 8: Visual Testing

**Step 1: Verify trunk is running**

Ensure `trunk serve` is running on http://localhost:8080

**Step 2: Open palette editor**

Navigate to the app and open the palette editor

**Step 3: Verify Light Effects section**

Check:
- [ ] "Light Effects" collapsible section appears after "Palette" section
- [ ] "3D Lighting" checkbox toggles visibility of sub-controls
- [ ] Falloff Curve editor appears when 3D enabled
- [ ] All 5 sliders appear with correct labels and ranges
- [ ] LightingControl circular picker appears
- [ ] Dragging the light position updates azimuth/elevation display
- [ ] Changes are reflected in the fractal render

**Step 4: Compare with prototype**

Open prototype at `docs/ux-palette-editor/` (`npm run dev`) and compare layout

**Step 5: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix(palette-editor): visual adjustments for Light Effects section"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Create LightingSlider component | lighting_slider.rs, mod.rs |
| 2 | Create LightingControl (static) | lighting_control.rs, mod.rs |
| 3 | Add drag to LightingControl | lighting_control.rs |
| 4 | Remove CurveEditor label | curve_editor.rs |
| 5 | Add signals to PaletteEditor | palette_editor.rs |
| 6 | Add Light Effects view | palette_editor.rs |
| 7 | Run test suite | - |
| 8 | Visual testing | - |
