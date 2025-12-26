# Transfer Curve Editor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a CurveEditor component to the palette editor for controlling color distribution via transfer curves.

**Architecture:** Canvas-based interactive editor following the GradientEditor pattern. Displays actual cubic spline interpolation (100 sample points). Control points are draggable with first/last locked to x=0/x=1.

**Tech Stack:** Leptos 0.6, web_sys (canvas, mouse events), wasm-bindgen

---

## Task 1: Create CurveEditor Component Shell

**Files:**
- Create: `fractalwonder-ui/src/components/curve_editor.rs`

**Step 1: Create the component file with imports and signature**

```rust
//! Interactive curve editor for transfer and falloff curves.

use crate::rendering::colorizers::{Curve, CurvePoint};
use crate::rendering::get_2d_context;
use leptos::*;
use web_sys::HtmlCanvasElement;

/// Interactive curve editor component.
#[component]
pub fn CurveEditor(
    /// The curve to edit (None when editor closed)
    curve: Signal<Option<Curve>>,
    /// Called when curve changes
    on_change: Callback<Curve>,
    /// Canvas size in logical pixels
    #[prop(default = 320)]
    size: u32,
) -> impl IntoView {
    view! {
        <Show when=move || curve.get().is_some()>
            <div class="bg-white/5 border border-white/10 rounded-lg p-4 space-y-2">
                <div class="text-white/50 text-xs">
                    "Transfer Curve (placeholder)"
                </div>
            </div>
        </Show>
    }
}
```

**Step 2: Run clippy to verify syntax**

Run: `cargo clippy -p fractalwonder-ui --all-targets -- -D warnings 2>&1 | head -30`
Expected: Compiles (unused warnings OK at this stage)

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/curve_editor.rs
git commit -m "feat(curve-editor): add component shell"
```

---

## Task 2: Export CurveEditor from Components Module

**Files:**
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Add module declaration and re-export**

Add after `mod gradient_editor;`:
```rust
mod curve_editor;
```

Add after `pub use gradient_editor::GradientEditor;`:
```rust
pub use curve_editor::CurveEditor;
```

**Step 2: Verify compilation**

Run: `cargo check -p fractalwonder-ui 2>&1 | tail -5`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/mod.rs
git commit -m "feat(curve-editor): export from components module"
```

---

## Task 3: Add Canvas and Drawing Infrastructure

**Files:**
- Modify: `fractalwonder-ui/src/components/curve_editor.rs`

**Step 1: Replace the component with canvas infrastructure**

```rust
//! Interactive curve editor for transfer and falloff curves.

use crate::rendering::colorizers::{Curve, CurvePoint};
use crate::rendering::get_2d_context;
use leptos::*;
use web_sys::HtmlCanvasElement;

/// Interactive curve editor component.
#[component]
pub fn CurveEditor(
    /// The curve to edit (None when editor closed)
    curve: Signal<Option<Curve>>,
    /// Called when curve changes
    on_change: Callback<Curve>,
    /// Canvas size in logical pixels
    #[prop(default = 320)]
    size: u32,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();
    let hover_index = create_rw_signal(None::<usize>);

    // Draw curve when it changes or hover state changes
    create_effect(move |_| {
        let Some(crv) = curve.get() else { return };
        let Some(canvas) = canvas_ref.get() else { return };
        let _ = hover_index.get(); // Track hover for redraw

        draw_curve(&canvas, &crv, size, hover_index.get());
    });

    view! {
        <Show when=move || curve.get().is_some()>
            <div class="bg-white/5 border border-white/10 rounded-lg p-4 space-y-2">
                <div class="text-white/50 text-xs mb-2">"Transfer Curve"</div>
                <canvas
                    node_ref=canvas_ref
                    width=size
                    height=size
                    class="cursor-crosshair rounded"
                    style="width: 100%; height: auto;"
                />
                <div class="text-white/50 text-xs">
                    "Click to add points · Drag to move · Double-click to remove"
                </div>
            </div>
        </Show>
    }
}

/// Draw the curve editor canvas.
fn draw_curve(
    canvas: &HtmlCanvasElement,
    curve: &Curve,
    size: u32,
    hover_index: Option<usize>,
) {
    let Ok(ctx) = get_2d_context(canvas) else { return };
    let size_f = size as f64;

    // Clear canvas
    ctx.clear_rect(0.0, 0.0, size_f, size_f);

    // Background
    ctx.set_fill_style_str("rgba(255, 255, 255, 0.05)");
    ctx.fill_rect(0.0, 0.0, size_f, size_f);

    // Grid (4x4)
    ctx.set_stroke_style_str("rgba(255, 255, 255, 0.1)");
    ctx.set_line_width(1.0);
    for i in 0..=4 {
        let pos = (i as f64 / 4.0) * size_f;
        ctx.begin_path();
        ctx.move_to(pos, 0.0);
        ctx.line_to(pos, size_f);
        ctx.stroke();
        ctx.begin_path();
        ctx.move_to(0.0, pos);
        ctx.line_to(size_f, pos);
        ctx.stroke();
    }

    // Diagonal reference line (dashed)
    ctx.set_stroke_style_str("rgba(255, 255, 255, 0.2)");
    ctx.set_line_dash(&js_sys::Array::of2(&5.0.into(), &5.0.into())).ok();
    ctx.begin_path();
    ctx.move_to(0.0, size_f);
    ctx.line_to(size_f, 0.0);
    ctx.stroke();
    ctx.set_line_dash(&js_sys::Array::new()).ok();

    // Draw actual cubic spline curve (100 sample points)
    ctx.set_stroke_style_str("rgba(255, 255, 255, 0.8)");
    ctx.set_line_width(2.0);
    ctx.begin_path();
    for i in 0..=100 {
        let x = i as f64 / 100.0;
        let y = curve.evaluate(x);
        let canvas_x = x * size_f;
        let canvas_y = (1.0 - y) * size_f; // Invert Y
        if i == 0 {
            ctx.move_to(canvas_x, canvas_y);
        } else {
            ctx.line_to(canvas_x, canvas_y);
        }
    }
    ctx.stroke();

    // Draw control points
    for (i, point) in curve.points.iter().enumerate() {
        let canvas_x = point.x * size_f;
        let canvas_y = (1.0 - point.y) * size_f;
        let radius = if hover_index == Some(i) { 6.0 } else { 5.0 };

        ctx.set_fill_style_str(if hover_index == Some(i) {
            "rgba(255, 255, 255, 1.0)"
        } else {
            "rgba(255, 255, 255, 0.9)"
        });
        ctx.begin_path();
        ctx.arc(canvas_x, canvas_y, radius, 0.0, std::f64::consts::TAU).ok();
        ctx.fill();

        ctx.set_stroke_style_str("rgba(0, 0, 0, 0.5)");
        ctx.set_line_width(2.0);
        ctx.stroke();
    }
}
```

**Step 2: Verify compilation**

Run: `cargo clippy -p fractalwonder-ui --all-targets -- -D warnings 2>&1 | head -30`
Expected: Compiles (may have unused `on_change` warning)

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/curve_editor.rs
git commit -m "feat(curve-editor): add canvas drawing with grid, curve, and control points"
```

---

## Task 4: Add Mouse Interaction - Hit Detection and Hover

**Files:**
- Modify: `fractalwonder-ui/src/components/curve_editor.rs`

**Step 1: Add hit detection helper function after draw_curve**

```rust
/// Find the index of a control point near the given canvas coordinates.
/// Returns None if no point is within the hit radius (10 pixels).
fn find_point_at(curve: &Curve, canvas_x: f64, canvas_y: f64, size: f64) -> Option<usize> {
    const HIT_RADIUS: f64 = 10.0;

    for (i, point) in curve.points.iter().enumerate() {
        let px = point.x * size;
        let py = (1.0 - point.y) * size;
        let dist = ((canvas_x - px).powi(2) + (canvas_y - py).powi(2)).sqrt();
        if dist < HIT_RADIUS {
            return Some(i);
        }
    }
    None
}

/// Convert mouse event to canvas-relative coordinates.
fn mouse_to_canvas(e: &web_sys::MouseEvent, canvas: &HtmlCanvasElement, size: u32) -> (f64, f64) {
    let rect = canvas.get_bounding_client_rect();
    let scale_x = size as f64 / rect.width();
    let scale_y = size as f64 / rect.height();
    let x = (e.client_x() as f64 - rect.left()) * scale_x;
    let y = (e.client_y() as f64 - rect.top()) * scale_y;
    (x.clamp(0.0, size as f64), y.clamp(0.0, size as f64))
}
```

**Step 2: Add hover handling to the canvas element in the view**

Replace the `<canvas .../>` element with:

```rust
<canvas
    node_ref=canvas_ref
    width=size
    height=size
    class="cursor-crosshair rounded"
    style="width: 100%; height: auto;"
    on:mousemove=move |e| {
        let Some(canvas) = canvas_ref.get() else { return };
        let Some(crv) = curve.get() else { return };
        let (x, y) = mouse_to_canvas(&e, &canvas, size);
        hover_index.set(find_point_at(&crv, x, y, size as f64));
    }
    on:mouseleave=move |_| {
        hover_index.set(None);
    }
/>
```

**Step 3: Verify compilation**

Run: `cargo clippy -p fractalwonder-ui --all-targets -- -D warnings 2>&1 | head -30`
Expected: Compiles

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/curve_editor.rs
git commit -m "feat(curve-editor): add hit detection and hover highlighting"
```

---

## Task 5: Add Point Dragging

**Files:**
- Modify: `fractalwonder-ui/src/components/curve_editor.rs`

**Step 1: Add drag state signals after hover_index**

```rust
let drag_index = create_rw_signal(None::<usize>);
let is_dragging = create_rw_signal(false);
```

**Step 2: Add drag handlers**

Add after the helper functions:

```rust
/// Clamp a curve value to valid range.
fn clamp_point(x: f64, y: f64, index: usize, point_count: usize) -> (f64, f64) {
    let x = if index == 0 {
        0.0 // First point locked to x=0
    } else if index == point_count - 1 {
        1.0 // Last point locked to x=1
    } else {
        x.clamp(0.0, 1.0)
    };
    (x, y.clamp(0.0, 1.0))
}
```

**Step 3: Add mousedown handler to canvas**

Add to the canvas element:

```rust
on:mousedown=move |e| {
    let Some(canvas) = canvas_ref.get() else { return };
    let Some(crv) = curve.get() else { return };
    let (x, y) = mouse_to_canvas(&e, &canvas, size);

    if let Some(idx) = find_point_at(&crv, x, y, size as f64) {
        // Start dragging existing point
        e.prevent_default();
        is_dragging.set(true);
        drag_index.set(Some(idx));
    }
}
```

**Step 4: Add document-level mouse handlers for drag**

Add after the draw effect:

```rust
// Document-level mouse handlers for drag
create_effect(move |_| {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");

    let size_copy = size;
    let canvas_ref_copy = canvas_ref;

    let mousemove_closure = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        if !is_dragging.get() {
            return;
        }
        let Some(idx) = drag_index.get() else { return };
        let Some(canvas) = canvas_ref_copy.get() else { return };
        let Some(mut crv) = curve.get() else { return };

        let (canvas_x, canvas_y) = mouse_to_canvas(&e, &canvas, size_copy);
        let x = canvas_x / size_copy as f64;
        let y = 1.0 - (canvas_y / size_copy as f64); // Invert Y

        let (x, y) = clamp_point(x, y, idx, crv.points.len());

        if idx < crv.points.len() {
            crv.points[idx] = CurvePoint { x, y };
            // Re-sort by x (maintains curve validity)
            crv.points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
            on_change.call(crv);
        }
    });

    let mouseup_closure = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
        is_dragging.set(false);
        drag_index.set(None);
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
```

**Step 5: Verify compilation**

Run: `cargo clippy -p fractalwonder-ui --all-targets -- -D warnings 2>&1 | head -30`
Expected: Compiles

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/components/curve_editor.rs
git commit -m "feat(curve-editor): add point dragging with boundary constraints"
```

---

## Task 6: Add Point Creation and Deletion

**Files:**
- Modify: `fractalwonder-ui/src/components/curve_editor.rs`

**Step 1: Modify mousedown to add points on empty area click**

Update the mousedown handler to add point creation:

```rust
on:mousedown=move |e| {
    let Some(canvas) = canvas_ref.get() else { return };
    let Some(crv) = curve.get() else { return };
    let (canvas_x, canvas_y) = mouse_to_canvas(&e, &canvas, size);

    if let Some(idx) = find_point_at(&crv, canvas_x, canvas_y, size as f64) {
        // Start dragging existing point
        e.prevent_default();
        is_dragging.set(true);
        drag_index.set(Some(idx));
    } else {
        // Add new point at click position
        let x = canvas_x / size as f64;
        let y = 1.0 - (canvas_y / size as f64);
        let mut new_curve = crv.clone();
        new_curve.points.push(CurvePoint { x, y });
        new_curve.points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
        on_change.call(new_curve);
    }
}
```

**Step 2: Add double-click handler to delete points**

Add to the canvas element:

```rust
on:dblclick=move |e| {
    let Some(canvas) = canvas_ref.get() else { return };
    let Some(crv) = curve.get() else { return };
    let (canvas_x, canvas_y) = mouse_to_canvas(&e, &canvas, size);

    if let Some(idx) = find_point_at(&crv, canvas_x, canvas_y, size as f64) {
        // Don't delete if only 2 points remain
        if crv.points.len() <= 2 {
            return;
        }
        // Don't delete first or last point
        if idx == 0 || idx == crv.points.len() - 1 {
            return;
        }
        let mut new_curve = crv.clone();
        new_curve.points.remove(idx);
        on_change.call(new_curve);
    }
}
```

**Step 3: Verify compilation**

Run: `cargo clippy -p fractalwonder-ui --all-targets -- -D warnings 2>&1 | head -30`
Expected: Compiles

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/curve_editor.rs
git commit -m "feat(curve-editor): add point creation and deletion"
```

---

## Task 7: Integrate into PaletteEditor

**Files:**
- Modify: `fractalwonder-ui/src/components/palette_editor.rs`

**Step 1: Add CurveEditor import**

Update the imports at the top:

```rust
use crate::components::{
    CollapsibleSection, ConfirmDialog, CurveEditor, EditMode, GradientEditor, PaletteEditorState,
};
use crate::rendering::colorizers::{Curve, Gradient, Palette};
```

**Step 2: Add transfer curve signal and callback**

Add after `on_gradient_change`:

```rust
// Derived: current transfer curve
let transfer_curve_signal = Signal::derive(move || {
    state.get().map(|s| s.working_palette.transfer_curve.clone())
});

// Callback for transfer curve changes
let on_transfer_curve_change = Callback::new(move |new_curve: Curve| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.transfer_curve = new_curve;
        }
    });
});
```

**Step 3: Add CurveEditor to the view**

Add after `<GradientEditor ... />` inside the Palette CollapsibleSection:

```rust
// Transfer curve editor
<CurveEditor
    curve=transfer_curve_signal
    on_change=on_transfer_curve_change
/>
```

**Step 4: Verify compilation**

Run: `cargo clippy -p fractalwonder-ui --all-targets -- -D warnings 2>&1 | head -30`
Expected: Compiles

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/palette_editor.rs
git commit -m "feat(curve-editor): integrate into palette editor"
```

---

## Task 8: Test in Browser

**Files:**
- None (manual testing)

**Step 1: Verify trunk is running**

Ensure `trunk serve` is running on http://localhost:8080

**Step 2: Open palette editor**

1. Open http://localhost:8080 in browser
2. Click the palette button in the bottom control bar
3. Expand the "Palette" section if collapsed

**Step 3: Verify curve editor appears**

Expected: Transfer Curve editor visible below gradient editor with:
- Grid (4x4 subdivisions)
- Dashed diagonal reference line
- Smooth cubic curve through control points
- White control point circles

**Step 4: Test interactions**

- Click empty area → New point added
- Drag point → Point moves (first/last locked to edges)
- Double-click middle point → Point deleted
- Hover over point → Point enlarges

**Step 5: Verify curve changes persist**

1. Modify the curve
2. Click "Apply"
3. Re-open palette editor
4. Verify curve changes were saved

**Step 6: Commit confirmation**

```bash
git add -A
git commit -m "feat(curve-editor): complete transfer curve editor implementation"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Component shell | `curve_editor.rs` (create) |
| 2 | Module export | `mod.rs` (modify) |
| 3 | Canvas drawing | `curve_editor.rs` (modify) |
| 4 | Hit detection & hover | `curve_editor.rs` (modify) |
| 5 | Point dragging | `curve_editor.rs` (modify) |
| 6 | Add/delete points | `curve_editor.rs` (modify) |
| 7 | PaletteEditor integration | `palette_editor.rs` (modify) |
| 8 | Browser testing | Manual |
