# Gradient Editor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an interactive gradient editor to the palette editor panel with OKLAB-correct rendering, draggable stops and midpoints, zoom, and color picking.

**Architecture:** The GradientEditor component manages local UI state (selection, zoom, drag) and calls `on_change` on mouse release. Canvas rendering uses `Gradient::to_preview_lut()` for OKLAB interpolation. Color utilities for hex↔RGB live in the colorizers module.

**Tech Stack:** Rust, Leptos 0.6, web_sys (canvas, mouse events), wasm-bindgen

---

## Task 1: Add hex↔RGB utility functions

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/color_space.rs`

**Step 1: Write failing tests**

Add to the `#[cfg(test)]` module at the bottom of `color_space.rs`:

```rust
#[test]
fn rgb_to_hex_black() {
    assert_eq!(rgb_to_hex([0, 0, 0]), "#000000");
}

#[test]
fn rgb_to_hex_white() {
    assert_eq!(rgb_to_hex([255, 255, 255]), "#ffffff");
}

#[test]
fn rgb_to_hex_color() {
    assert_eq!(rgb_to_hex([255, 68, 0]), "#ff4400");
}

#[test]
fn hex_to_rgb_valid() {
    assert_eq!(hex_to_rgb("#ff4400"), Some([255, 68, 0]));
}

#[test]
fn hex_to_rgb_no_hash() {
    assert_eq!(hex_to_rgb("ff4400"), Some([255, 68, 0]));
}

#[test]
fn hex_to_rgb_uppercase() {
    assert_eq!(hex_to_rgb("#FF4400"), Some([255, 68, 0]));
}

#[test]
fn hex_to_rgb_invalid_length() {
    assert_eq!(hex_to_rgb("#fff"), None);
}

#[test]
fn hex_to_rgb_invalid_chars() {
    assert_eq!(hex_to_rgb("#gggggg"), None);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-ui rgb_to_hex`
Expected: FAIL with "cannot find function `rgb_to_hex`"

**Step 3: Implement the functions**

Add at the end of `color_space.rs`, before the `#[cfg(test)]` module:

```rust
/// Convert RGB [0-255] to hex string (e.g., [255, 68, 0] → "#ff4400")
pub fn rgb_to_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// Parse hex string to RGB (e.g., "#ff4400" → Some([255, 68, 0]))
/// Handles with/without # prefix, case-insensitive. Returns None on invalid input.
pub fn hex_to_rgb(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui rgb_to_hex hex_to_rgb`
Expected: All 8 tests PASS

**Step 5: Export in module**

Add to `fractalwonder-ui/src/rendering/colorizers/mod.rs`:

```rust
pub use color_space::{hex_to_rgb, rgb_to_hex};
```

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/color_space.rs fractalwonder-ui/src/rendering/colorizers/mod.rs
git commit -m "feat(colorizers): add hex↔RGB conversion utilities"
```

---

## Task 2: Add `to_preview_lut()` to Gradient

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/gradient.rs`

**Step 1: Write failing test**

Add to the `#[cfg(test)]` module in `gradient.rs`:

```rust
#[test]
fn preview_lut_matches_full_lut_at_endpoints() {
    let gradient = Gradient::new(vec![
        ColorStop { position: 0.0, color: [255, 0, 0] },
        ColorStop { position: 1.0, color: [0, 0, 255] },
    ]);
    let preview = gradient.to_preview_lut(100);
    let full = gradient.to_lut();

    assert_eq!(preview.len(), 100);
    assert_eq!(preview[0], full[0], "Start colors should match");
    assert_eq!(preview[99], full[4095], "End colors should match");
}

#[test]
fn preview_lut_samples_correctly() {
    let gradient = Gradient::new(vec![
        ColorStop { position: 0.0, color: [0, 0, 0] },
        ColorStop { position: 1.0, color: [255, 255, 255] },
    ]);
    let preview = gradient.to_preview_lut(256);

    // Midpoint should be roughly middle gray (OKLAB interpolation may differ slightly)
    let mid = preview[128];
    assert!(mid[0] > 100 && mid[0] < 200, "Midpoint R should be mid-range, got {}", mid[0]);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-ui preview_lut`
Expected: FAIL with "no method named `to_preview_lut`"

**Step 3: Implement the method**

Add to `impl Gradient` in `gradient.rs`:

```rust
/// Generate a LUT sized to the given width for editor preview.
/// Uses the same OKLAB interpolation as `to_lut()`.
pub fn to_preview_lut(&self, width: usize) -> Vec<[u8; 3]> {
    if width == 0 {
        return vec![];
    }
    if self.stops.is_empty() {
        return vec![[0, 0, 0]; width];
    }
    if self.stops.len() == 1 {
        return vec![self.stops[0].color; width];
    }

    // Convert stops to OKLAB
    let oklab_stops: Vec<(f64, (f64, f64, f64))> = self
        .stops
        .iter()
        .map(|stop| {
            let r = srgb_to_linear(stop.color[0] as f64 / 255.0);
            let g = srgb_to_linear(stop.color[1] as f64 / 255.0);
            let b = srgb_to_linear(stop.color[2] as f64 / 255.0);
            (stop.position, linear_rgb_to_oklab(r, g, b))
        })
        .collect();

    (0..width)
        .map(|i| {
            let t = if width == 1 { 0.0 } else { i as f64 / (width - 1) as f64 };
            self.sample_oklab(&oklab_stops, t)
        })
        .collect()
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-ui preview_lut`
Expected: Both tests PASS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/gradient.rs
git commit -m "feat(gradient): add to_preview_lut() for editor canvas"
```

---

## Task 3: Create GradientEditor component skeleton

**Files:**
- Create: `fractalwonder-ui/src/components/gradient_editor.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create the component file**

Create `fractalwonder-ui/src/components/gradient_editor.rs`:

```rust
//! Interactive gradient editor with color stops, midpoints, and zoom.

use crate::rendering::colorizers::Gradient;
use leptos::*;

/// Interactive gradient editor component.
#[component]
pub fn GradientEditor(
    /// The gradient to edit (None when editor closed)
    gradient: Signal<Option<Gradient>>,
    /// Called when gradient changes (on mouse release)
    on_change: Callback<Gradient>,
) -> impl IntoView {
    // Internal state
    let selected_stop = create_rw_signal(None::<usize>);
    let zoom = create_rw_signal(1.0_f64);
    let _is_dragging = create_rw_signal(false);

    view! {
        <Show when=move || gradient.get().is_some()>
            <div class="space-y-2">
                // Zoom controls (hidden at 1x)
                <Show when=move || zoom.get() > 1.0>
                    <div class="flex items-center justify-between px-1">
                        <span class="text-white/50 text-xs">
                            {move || format!("Zoom: {:.1}x", zoom.get())}
                        </span>
                    </div>
                </Show>

                // Placeholder for gradient bar
                <div class="h-8 bg-white/10 rounded border border-white/20">
                    <span class="text-white/50 text-xs p-2">"Gradient bar placeholder"</span>
                </div>

                // Placeholder for color picker
                <Show when=move || selected_stop.get().is_some()>
                    <div class="bg-white/5 border border-white/10 rounded p-2">
                        <span class="text-white/50 text-xs">"Color picker placeholder"</span>
                    </div>
                </Show>
            </div>
        </Show>
    }
}
```

**Step 2: Export in mod.rs**

Add to `fractalwonder-ui/src/components/mod.rs`:

After `mod palette_menu;`:
```rust
mod gradient_editor;
```

After `pub use palette_menu::PaletteMenu;`:
```rust
pub use gradient_editor::GradientEditor;
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(gradient-editor): add component skeleton"
```

---

## Task 4: Integrate GradientEditor into PaletteEditor

**Files:**
- Modify: `fractalwonder-ui/src/components/palette_editor.rs`

**Step 1: Add import**

At the top of `palette_editor.rs`, add to the imports:

```rust
use crate::components::GradientEditor;
use crate::rendering::colorizers::Gradient;
```

**Step 2: Create derived gradient signal**

Inside the `PaletteEditor` component, after the existing derived signals (around line 90), add:

```rust
// Derived: current gradient
let gradient_signal = Signal::derive(move || {
    state.get().map(|s| s.working_palette.gradient.clone())
});

// Callback for gradient changes
let on_gradient_change = Callback::new(move |new_gradient: Gradient| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.gradient = new_gradient;
        }
    });
});
```

**Step 3: Add GradientEditor to the Palette section**

In the view, find the `CollapsibleSection` for "Palette" (around line 333). After the closing `</div>` of the checkboxes div (around line 370), add:

```rust
// Gradient editor
<GradientEditor
    gradient=gradient_signal
    on_change=on_gradient_change
/>
```

**Step 4: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 5: Visual test in browser**

Open http://localhost:8080, open palette editor. The "Palette" section should show the placeholder text "Gradient bar placeholder".

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/components/palette_editor.rs
git commit -m "feat(palette-editor): integrate GradientEditor component"
```

---

## Task 5: Implement canvas gradient bar rendering

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Add imports for canvas**

At the top of `gradient_editor.rs`:

```rust
use crate::rendering::canvas_utils::get_2d_context;
use crate::rendering::colorizers::Gradient;
use leptos::*;
use wasm_bindgen::Clamped;
use web_sys::{HtmlCanvasElement, ImageData};
```

**Step 2: Add canvas ref and drawing effect**

Replace the entire component implementation with:

```rust
/// Interactive gradient editor component.
#[component]
pub fn GradientEditor(
    /// The gradient to edit (None when editor closed)
    gradient: Signal<Option<Gradient>>,
    /// Called when gradient changes (on mouse release)
    on_change: Callback<Gradient>,
) -> impl IntoView {
    // Internal state
    let selected_stop = create_rw_signal(None::<usize>);
    let zoom = create_rw_signal(1.0_f64);
    let _is_dragging = create_rw_signal(false);

    // Canvas ref
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Draw gradient when it changes
    create_effect(move |_| {
        let Some(grad) = gradient.get() else { return };
        let Some(canvas) = canvas_ref.get() else { return };

        let canvas_el: &HtmlCanvasElement = &canvas;
        let width = canvas_el.width() as usize;
        let height = canvas_el.height() as usize;

        if width == 0 || height == 0 {
            return;
        }

        // Generate OKLAB-interpolated colors
        let lut = grad.to_preview_lut(width);

        // Convert to RGBA pixels (repeat each column for full height)
        let mut pixels = vec![0u8; width * height * 4];
        for x in 0..width {
            let [r, g, b] = lut[x];
            for y in 0..height {
                let idx = (y * width + x) * 4;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = 255;
            }
        }

        // Draw to canvas
        if let Ok(ctx) = get_2d_context(canvas_el) {
            if let Ok(image_data) = ImageData::new_with_u8_clamped_array_and_sh(
                Clamped(&pixels),
                width as u32,
                height as u32,
            ) {
                let _ = ctx.put_image_data(&image_data, 0.0, 0.0);
            }
        }
    });

    view! {
        <Show when=move || gradient.get().is_some()>
            <div class="space-y-2">
                // Zoom controls
                <div class="flex items-center justify-between px-1">
                    <div class="text-white/50 text-xs">
                        {move || if zoom.get() > 1.0 {
                            format!("Zoom: {:.1}x", zoom.get())
                        } else {
                            String::new()
                        }}
                    </div>
                    <div class="flex items-center gap-1">
                        <button
                            class="p-1 rounded hover:bg-white/10 text-white disabled:opacity-30 \
                                   disabled:cursor-not-allowed transition-colors"
                            prop:disabled=move || zoom.get() <= 1.0
                            on:click=move |_| zoom.update(|z| *z = (*z / 1.2).max(1.0))
                        >
                            <ZoomOutIcon />
                        </button>
                        <button
                            class="p-1 rounded hover:bg-white/10 text-white disabled:opacity-30 \
                                   disabled:cursor-not-allowed transition-colors"
                            prop:disabled=move || zoom.get() >= 10.0
                            on:click=move |_| zoom.update(|z| *z = (*z * 1.2).min(10.0))
                        >
                            <ZoomInIcon />
                        </button>
                    </div>
                </div>

                // Scrollable gradient container
                <div
                    class="overflow-x-auto overflow-y-visible"
                    style=move || format!("max-width: 100%;")
                >
                    <div
                        class="relative"
                        style=move || format!("width: {}%;", zoom.get() * 100.0)
                    >
                        // Color stops placeholder (above bar)
                        <div class="relative h-6 mb-1">
                            // Stops will go here
                        </div>

                        // Gradient bar (canvas)
                        <canvas
                            node_ref=canvas_ref
                            class="w-full rounded border border-white/20 cursor-crosshair"
                            width="320"
                            height="32"
                            style="height: 32px;"
                        />
                    </div>
                </div>

                // Color picker panel
                <Show when=move || selected_stop.get().is_some()>
                    <div class="bg-white/5 border border-white/10 rounded p-2">
                        <span class="text-white/50 text-xs">"Color picker placeholder"</span>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

// Zoom icons
#[component]
fn ZoomOutIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"/>
            <path d="m21 21-4.3-4.3"/>
            <path d="M8 11h6"/>
        </svg>
    }
}

#[component]
fn ZoomInIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"/>
            <path d="m21 21-4.3-4.3"/>
            <path d="M11 8v6"/>
            <path d="M8 11h6"/>
        </svg>
    }
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 4: Visual test in browser**

Open http://localhost:8080, open palette editor. The gradient bar should display the actual OKLAB-interpolated gradient colors.

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs
git commit -m "feat(gradient-editor): implement canvas gradient bar with OKLAB"
```

---

## Task 6: Render color stop markers

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Add stop rendering**

Inside the view, replace the empty `div.relative.h-6.mb-1` with:

```rust
// Color stops (squares above gradient bar)
<div class="relative h-6 mb-1">
    <For
        each=move || {
            gradient.get()
                .map(|g| g.stops.iter().enumerate()
                    .map(|(i, s)| (i, s.position, s.color))
                    .collect::<Vec<_>>())
                .unwrap_or_default()
        }
        key=|(i, _, _)| *i
        children=move |(index, position, color)| {
            let is_selected = move || selected_stop.get() == Some(index);
            let color_hex = format!(
                "#{:02x}{:02x}{:02x}",
                color[0], color[1], color[2]
            );

            view! {
                <div
                    class="absolute top-0 w-3 h-3 cursor-ew-resize transition-shadow"
                    style=move || format!(
                        "left: {}%; transform: translateX(-50%); \
                         background-color: {}; \
                         border: 1px solid rgba(255, 255, 255, 0.3); \
                         box-shadow: {};",
                        position * 100.0,
                        color_hex,
                        if is_selected() {
                            "0 0 6px 2px rgba(255, 255, 255, 0.7)"
                        } else {
                            "none"
                        }
                    )
                    on:click=move |e| {
                        e.stop_propagation();
                        selected_stop.set(Some(index));
                    }
                />
            }
        }
    />
</div>
```

**Step 2: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 3: Visual test in browser**

Open palette editor. Color stop squares should appear above the gradient bar at their correct positions. Clicking a stop should show the glow effect.

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs
git commit -m "feat(gradient-editor): render color stop markers with selection glow"
```

---

## Task 7: Implement color stop dragging

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Add drag state and container ref**

Add after the existing signals:

```rust
let is_dragging = create_rw_signal(false);
let drag_index = create_rw_signal(None::<usize>);
let container_ref = create_node_ref::<leptos::html::Div>();
```

**Step 2: Add drag handlers**

Add these closures after the signals:

```rust
// Handle drag start on a stop
let start_drag = move |index: usize, e: web_sys::MouseEvent| {
    e.prevent_default();
    is_dragging.set(true);
    drag_index.set(Some(index));
    selected_stop.set(Some(index));
};

// Handle mouse move during drag
let handle_mouse_move = move |e: web_sys::MouseEvent| {
    if !is_dragging.get() {
        return;
    }
    let Some(index) = drag_index.get() else { return };
    let Some(container) = container_ref.get() else { return };
    let Some(mut grad) = gradient.get() else { return };

    let rect = container.get_bounding_client_rect();
    let x = e.client_x() as f64 - rect.left();
    let width = rect.width();
    let position = (x / width).clamp(0.0, 1.0);

    // Update stop position
    if index < grad.stops.len() {
        grad.stops[index].position = position;
        // Don't call on_change yet - wait for release
    }
};

// Handle drag end
let end_drag = move |_: web_sys::MouseEvent| {
    if is_dragging.get() {
        is_dragging.set(false);
        if let Some(grad) = gradient.get() {
            // Sort stops by position and call on_change
            let mut sorted = grad.clone();
            sorted.stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
            on_change.call(sorted);
        }
    }
    drag_index.set(None);
};
```

**Step 3: Attach handlers to the stop elements**

Update the stop `div` to include mousedown:

```rust
on:mousedown=move |e| start_drag(index, e)
```

**Step 4: Add document-level mouse handlers**

Add an effect to manage document-level listeners:

```rust
// Document-level mouse handlers for drag
create_effect(move |_| {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");

    let mousemove_closure = Closure::<dyn Fn(web_sys::MouseEvent)>::new(handle_mouse_move);
    let mouseup_closure = Closure::<dyn Fn(web_sys::MouseEvent)>::new(end_drag);

    let _ = document.add_event_listener_with_callback(
        "mousemove",
        mousemove_closure.as_ref().unchecked_ref(),
    );
    let _ = document.add_event_listener_with_callback(
        "mouseup",
        mouseup_closure.as_ref().unchecked_ref(),
    );

    // Leak closures (they live for app lifetime)
    mousemove_closure.forget();
    mouseup_closure.forget();
});
```

**Step 5: Update container to use ref**

Change the container div to use the ref:

```rust
<div
    node_ref=container_ref
    class="relative"
    style=move || format!("width: {}%;", zoom.get() * 100.0)
>
```

**Step 6: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 7: Test in browser**

Drag a color stop. It should move, and the gradient should update when you release.

**Step 8: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs
git commit -m "feat(gradient-editor): implement color stop dragging"
```

---

## Task 8: Implement click-to-add stops

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Add click handler to canvas**

Add this closure after the drag handlers:

```rust
// Handle click on gradient bar to add a stop
let handle_bar_click = move |e: web_sys::MouseEvent| {
    let Some(container) = container_ref.get() else { return };
    let Some(mut grad) = gradient.get() else { return };

    let rect = container.get_bounding_client_rect();
    let x = e.client_x() as f64 - rect.left();
    let width = rect.width();
    let position = (x / width).clamp(0.0, 1.0);

    // Sample color from gradient at this position
    let lut = grad.to_preview_lut(1000);
    let lut_index = ((position * 999.0) as usize).min(999);
    let color = lut[lut_index];

    // Add new stop
    grad.stops.push(crate::rendering::colorizers::ColorStop { position, color });
    grad.stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

    // Update midpoints array to match new stop count
    let new_midpoint_count = grad.stops.len().saturating_sub(1);
    grad.midpoints.resize(new_midpoint_count, 0.5);

    // Find the index of the new stop and select it
    let new_index = grad.stops.iter().position(|s| s.position == position);
    selected_stop.set(new_index);

    on_change.call(grad);
};
```

**Step 2: Attach handler to canvas**

Update the canvas element:

```rust
<canvas
    node_ref=canvas_ref
    class="w-full rounded border border-white/20 cursor-crosshair"
    width="320"
    height="32"
    style="height: 32px;"
    on:click=handle_bar_click
/>
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 4: Test in browser**

Click on the gradient bar. A new stop should appear with the correct sampled color.

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs
git commit -m "feat(gradient-editor): click to add stops with sampled color"
```

---

## Task 9: Implement double-click to delete stops

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Add double-click handler to stops**

Update the stop div to handle double-click:

```rust
on:dblclick=move |e| {
    e.stop_propagation();
    let Some(mut grad) = gradient.get() else { return };

    // Silently ignore if only 2 stops remain
    if grad.stops.len() <= 2 {
        return;
    }

    // Remove the stop
    if index < grad.stops.len() {
        grad.stops.remove(index);
        // Update midpoints
        let new_midpoint_count = grad.stops.len().saturating_sub(1);
        grad.midpoints.resize(new_midpoint_count, 0.5);

        selected_stop.set(None);
        on_change.call(grad);
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 3: Test in browser**

Double-click a stop to delete it. Should work unless only 2 stops remain.

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs
git commit -m "feat(gradient-editor): double-click to delete stops"
```

---

## Task 10: Implement color picker panel

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Add imports for color utilities**

Add at the top:

```rust
use crate::rendering::colorizers::{hex_to_rgb, rgb_to_hex, ColorStop, Gradient};
```

**Step 2: Replace color picker placeholder**

Replace the color picker `<Show>` block with:

```rust
// Color picker panel (shown when stop selected)
<Show when=move || selected_stop.get().is_some()>
    {move || {
        let index = selected_stop.get().unwrap();
        let grad = gradient.get();
        let stop = grad.as_ref().and_then(|g| g.stops.get(index));

        if let Some(stop) = stop {
            let color = stop.color;
            let hex = rgb_to_hex(color);

            view! {
                <div class="bg-white/5 border border-white/10 rounded p-2 space-y-2">
                    <div class="flex items-center gap-2">
                        // Native color picker
                        <input
                            type="color"
                            value=hex.clone()
                            class="w-12 h-8 rounded cursor-pointer bg-transparent"
                            on:change=move |e| {
                                let value = event_target_value(&e);
                                if let Some(rgb) = hex_to_rgb(&value) {
                                    let Some(mut grad) = gradient.get() else { return };
                                    if let Some(stop) = grad.stops.get_mut(index) {
                                        stop.color = rgb;
                                        on_change.call(grad);
                                    }
                                }
                            }
                        />
                        // Hex input
                        <input
                            type="text"
                            value=hex
                            class="flex-1 bg-white/5 border border-white/20 rounded px-2 py-1 \
                                   text-white text-xs outline-none focus:border-white/40"
                            on:change=move |e| {
                                let value = event_target_value(&e);
                                if let Some(rgb) = hex_to_rgb(&value) {
                                    let Some(mut grad) = gradient.get() else { return };
                                    if let Some(stop) = grad.stops.get_mut(index) {
                                        stop.color = rgb;
                                        on_change.call(grad);
                                    }
                                }
                            }
                        />
                    </div>
                </div>
            }.into_view()
        } else {
            view! {}.into_view()
        }
    }}
</Show>
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 4: Test in browser**

Select a stop. The color picker panel should appear with the native color input and hex field. Changes should update the gradient.

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs
git commit -m "feat(gradient-editor): implement color picker panel"
```

---

## Task 11: Render midpoint diamonds

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Add midpoint rendering**

Inside the stops div, after the `<For>` for stops, add:

```rust
// Midpoint diamonds (between stops)
<For
    each=move || {
        gradient.get()
            .map(|g| {
                let sorted_stops: Vec<_> = g.stops.iter()
                    .enumerate()
                    .map(|(i, s)| (i, s.position))
                    .collect();
                let midpoints: Vec<_> = g.midpoints.iter().copied().collect();

                (0..sorted_stops.len().saturating_sub(1))
                    .map(|i| {
                        let left_pos = sorted_stops[i].1;
                        let right_pos = sorted_stops.get(i + 1).map(|s| s.1).unwrap_or(1.0);
                        let midpoint_val = midpoints.get(i).copied().unwrap_or(0.5);
                        let display_pos = left_pos + (right_pos - left_pos) * midpoint_val;
                        (i, display_pos, left_pos, right_pos)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }
    key=|(i, _, _, _)| *i
    children=move |(index, display_pos, _left_pos, _right_pos)| {
        view! {
            <div
                class="absolute top-0 w-2.5 h-2.5 bg-white/80 cursor-ew-resize \
                       border border-white/50"
                style=move || format!(
                    "left: {}%; transform: translateX(-50%) rotate(45deg); margin-top: 1px;",
                    display_pos * 100.0
                )
            />
        }
    }
/>
```

**Step 2: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 3: Test in browser**

Midpoint diamonds should appear between each pair of stops.

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs
git commit -m "feat(gradient-editor): render midpoint diamonds"
```

---

## Task 12: Implement midpoint dragging

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Add midpoint drag state**

Add after other signals:

```rust
let dragging_midpoint = create_rw_signal(None::<usize>);
```

**Step 2: Add midpoint drag handlers**

Add these closures:

```rust
// Start dragging a midpoint
let start_midpoint_drag = move |index: usize, e: web_sys::MouseEvent| {
    e.prevent_default();
    e.stop_propagation();
    is_dragging.set(true);
    dragging_midpoint.set(Some(index));
};

// Update midpoint position during drag
let update_midpoint = move |e: &web_sys::MouseEvent| {
    let Some(index) = dragging_midpoint.get() else { return };
    let Some(container) = container_ref.get() else { return };
    let Some(grad) = gradient.get() else { return };

    if index >= grad.stops.len().saturating_sub(1) {
        return;
    }

    // Get the positions of the two stops this midpoint is between
    let left_pos = grad.stops[index].position;
    let right_pos = grad.stops.get(index + 1).map(|s| s.position).unwrap_or(1.0);

    let rect = container.get_bounding_client_rect();
    let x = e.client_x() as f64 - rect.left();
    let width = rect.width();
    let click_pos = (x / width).clamp(0.0, 1.0);

    // Calculate midpoint value (0-1 relative to segment)
    let segment_width = right_pos - left_pos;
    if segment_width.abs() < 0.001 {
        return;
    }

    let midpoint_val = ((click_pos - left_pos) / segment_width).clamp(0.05, 0.95);

    // Store temporarily - will apply on release
    // For now we just track position visually
};

// End midpoint drag
let end_midpoint_drag = move |e: web_sys::MouseEvent| {
    let Some(index) = dragging_midpoint.get() else {
        return;
    };
    let Some(container) = container_ref.get() else { return };
    let Some(mut grad) = gradient.get() else { return };

    if index >= grad.midpoints.len() {
        dragging_midpoint.set(None);
        is_dragging.set(false);
        return;
    }

    let left_pos = grad.stops[index].position;
    let right_pos = grad.stops.get(index + 1).map(|s| s.position).unwrap_or(1.0);

    let rect = container.get_bounding_client_rect();
    let x = e.client_x() as f64 - rect.left();
    let width = rect.width();
    let click_pos = (x / width).clamp(0.0, 1.0);

    let segment_width = right_pos - left_pos;
    if segment_width.abs() >= 0.001 {
        let midpoint_val = ((click_pos - left_pos) / segment_width).clamp(0.05, 0.95);
        grad.midpoints[index] = midpoint_val;
        on_change.call(grad);
    }

    dragging_midpoint.set(None);
    is_dragging.set(false);
};
```

**Step 3: Update mouse handlers**

Modify the `handle_mouse_move` and `end_drag` to also handle midpoint dragging, or update the document-level handlers.

**Step 4: Add mousedown to midpoint diamonds**

Update the midpoint div:

```rust
on:mousedown=move |e| start_midpoint_drag(index, e)
```

**Step 5: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: No errors

**Step 6: Test in browser**

Drag midpoint diamonds. The gradient blend should adjust.

**Step 7: Commit**

```bash
git add fractalwonder-ui/src/components/gradient_editor.rs
git commit -m "feat(gradient-editor): implement midpoint dragging"
```

---

## Task 13: Final cleanup and testing

**Files:**
- Modify: `fractalwonder-ui/src/components/gradient_editor.rs`

**Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Run fmt**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 4: Full browser test**

Test all features:
- [ ] Gradient bar displays with correct OKLAB colors
- [ ] Zoom +/- buttons work
- [ ] Horizontal scroll when zoomed
- [ ] Click stops to select (glow appears)
- [ ] Drag stops to reposition
- [ ] Click gradient bar to add stop (correct sampled color)
- [ ] Double-click stop to delete (min 2 enforced)
- [ ] Color picker appears when stop selected
- [ ] Native color input works
- [ ] Hex input works
- [ ] Midpoint diamonds display between stops
- [ ] Drag midpoints to adjust blend
- [ ] Changes mark editor as dirty
- [ ] Apply saves changes
- [ ] Cancel discards changes

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat(gradient-editor): complete gradient editor implementation"
```

---

## Summary

| Task | Description |
|------|-------------|
| 1 | Add hex↔RGB utility functions |
| 2 | Add `to_preview_lut()` to Gradient |
| 3 | Create GradientEditor component skeleton |
| 4 | Integrate into PaletteEditor |
| 5 | Canvas gradient bar with OKLAB |
| 6 | Render color stop markers |
| 7 | Implement stop dragging |
| 8 | Click-to-add stops |
| 9 | Double-click to delete |
| 10 | Color picker panel |
| 11 | Render midpoint diamonds |
| 12 | Midpoint dragging |
| 13 | Final cleanup and testing |
