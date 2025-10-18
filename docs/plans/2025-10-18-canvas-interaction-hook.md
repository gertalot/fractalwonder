# Canvas Interaction Hook Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Create a generic, framework-agnostic canvas interaction hook (`use_canvas_interaction`) that provides real-time pan/zoom preview with pixel-perfect ImageData transformations, designed for canvases where full re-renders are expensive (minutes/hours).

**Architecture:** Single Leptos hook managing event listeners (pointer, wheel), ImageData snapshot, requestAnimationFrame preview loop, and 1.5s debounce timeout. Returns transformation as both discrete values (offset_x, offset_y, zoom_factor) and a 2D affine matrix for maximum consumer flexibility.

**Tech Stack:** Leptos 0.6 (signals, effects, NodeRef), web-sys (Canvas APIs, ImageData, requestAnimationFrame), wasm-bindgen (closures)

---

## Task 1: Create hooks module and basic structure

**Files:**
- Create: `src/hooks/mod.rs`
- Create: `src/hooks/use_canvas_interaction.rs`
- Modify: `src/lib.rs:1-4`

**Step 1: Write the failing test**

Test file: `src/hooks/use_canvas_interaction.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let matrix = build_transform_matrix((0.0, 0.0), 1.0, None);
        assert_eq!(matrix, [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ]);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_identity_matrix`
Expected: FAIL with "cannot find function `build_transform_matrix`"

**Step 3: Create basic module structure**

File: `src/hooks/mod.rs`
```rust
pub mod use_canvas_interaction;
```

File: `src/hooks/use_canvas_interaction.rs`
```rust
use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Transformation result returned when interaction ends
#[derive(Debug, Clone, PartialEq)]
pub struct TransformResult {
    pub offset_x: f64,
    pub offset_y: f64,
    pub zoom_factor: f64,
    pub matrix: [[f64; 3]; 3],
}

/// Handle returned by the hook
pub struct InteractionHandle {
    pub is_interacting: ReadSignal<bool>,
    pub reset: Box<dyn Fn()>,
}

/// Builds a 2D affine transformation matrix from offset, zoom, and optional zoom center
fn build_transform_matrix(
    offset: (f64, f64),
    zoom: f64,
    zoom_center: Option<(f64, f64)>,
) -> [[f64; 3]; 3] {
    let mut matrix = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];

    // Apply scale (zoom)
    matrix[0][0] = zoom;
    matrix[1][1] = zoom;

    // Apply translation
    if let Some((cx, cy)) = zoom_center {
        // Translate to zoom center, scale, translate back
        matrix[0][2] = offset.0 + cx * (1.0 - zoom);
        matrix[1][2] = offset.1 + cy * (1.0 - zoom);
    } else {
        matrix[0][2] = offset.0;
        matrix[1][2] = offset.1;
    }

    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let matrix = build_transform_matrix((0.0, 0.0), 1.0, None);
        assert_eq!(matrix, [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ]);
    }
}
```

**Step 4: Add hooks module to lib.rs**

File: `src/lib.rs` (modify lines 1-4)
```rust
mod app;
mod components;
pub mod hooks;
pub mod rendering;
```

**Step 5: Run test to verify it passes**

Run: `cargo test test_identity_matrix`
Expected: PASS

**Step 6: Commit**

```bash
git add src/hooks/mod.rs src/hooks/use_canvas_interaction.rs src/lib.rs
git commit -m "feat: add hooks module with transform matrix builder"
```

---

## Task 2: Add matrix transformation tests

**Files:**
- Modify: `src/hooks/use_canvas_interaction.rs:50-end` (add tests)

**Step 1: Write the failing tests**

Add to test module in `src/hooks/use_canvas_interaction.rs`:

```rust
#[test]
fn test_translation_matrix() {
    let matrix = build_transform_matrix((100.0, 50.0), 1.0, None);
    assert_eq!(matrix, [
        [1.0, 0.0, 100.0],
        [0.0, 1.0, 50.0],
        [0.0, 0.0, 1.0],
    ]);
}

#[test]
fn test_zoom_matrix_no_center() {
    let matrix = build_transform_matrix((0.0, 0.0), 2.0, None);
    assert_eq!(matrix, [
        [2.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [0.0, 0.0, 1.0],
    ]);
}

#[test]
fn test_zoom_matrix_with_center() {
    let matrix = build_transform_matrix((0.0, 0.0), 2.0, Some((100.0, 100.0)));
    // Zoom 2x centered at (100, 100)
    // Translation should be 100*(1-2) = -100 for both x and y
    assert_eq!(matrix, [
        [2.0, 0.0, -100.0],
        [0.0, 2.0, -100.0],
        [0.0, 0.0, 1.0],
    ]);
}

#[test]
fn test_combined_transform() {
    let matrix = build_transform_matrix((50.0, 30.0), 1.5, Some((200.0, 150.0)));
    // offset + center*(1-zoom)
    // x: 50 + 200*(1-1.5) = 50 + 200*(-0.5) = 50 - 100 = -50
    // y: 30 + 150*(1-1.5) = 30 + 150*(-0.5) = 30 - 75 = -45
    assert_eq!(matrix, [
        [1.5, 0.0, -50.0],
        [0.0, 1.5, -45.0],
        [0.0, 0.0, 1.0],
    ]);
}
```

**Step 2: Run tests to verify current behavior**

Run: `cargo test build_transform_matrix`
Expected: Tests should pass if matrix math is correct, fail if wrong

**Step 3: Fix implementation if tests fail**

If any tests fail, adjust `build_transform_matrix` logic until all pass.

**Step 4: Run all tests to verify**

Run: `cargo test`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/hooks/use_canvas_interaction.rs
git commit -m "test: add comprehensive matrix transformation tests"
```

---

## Task 3: Add web-sys features for animation and events

**Files:**
- Modify: `Cargo.toml:17-29` (add web-sys features)

**Step 1: Add required web-sys features**

File: `Cargo.toml` (modify web-sys features section)
```toml
[dependencies.web-sys]
version = "0.3"
features = [
  "Window",
  "Document",
  "HtmlCanvasElement",
  "CanvasRenderingContext2d",
  "ImageData",
  "MouseEvent",
  "EventTarget",
  "CssStyleDeclaration",
  "Element",
  "HtmlElement",
  "PointerEvent",
  "WheelEvent",
  "Performance",
]
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "deps: add web-sys features for pointer and wheel events"
```

---

## Task 4: Implement core hook with interaction state

**Files:**
- Modify: `src/hooks/use_canvas_interaction.rs:25-50` (add hook implementation)

**Step 1: Write test for hook creation**

Add to test module:
```rust
#[cfg(test)]
mod browser_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_hook_creates_handle() {
        let canvas_ref = create_node_ref::<leptos::html::Canvas>();
        let callback_fired = create_rw_signal(false);

        let handle = use_canvas_interaction(
            canvas_ref,
            move |_result| {
                callback_fired.set(true);
            },
        );

        assert_eq!(handle.is_interacting.get(), false);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --target wasm32-unknown-unknown`
Expected: FAIL with "cannot find function `use_canvas_interaction`"

**Step 3: Implement core hook structure**

Add before the tests in `src/hooks/use_canvas_interaction.rs`:

```rust
const INTERACTION_TIMEOUT_MS: i32 = 1500;
const ZOOM_SENSITIVITY: f64 = 0.0005;

pub fn use_canvas_interaction<F>(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    on_interaction_end: F,
) -> InteractionHandle
where
    F: Fn(TransformResult) + 'static,
{
    // Interaction state signals
    let is_dragging = create_rw_signal(false);
    let is_zooming = create_rw_signal(false);
    let is_interacting = create_memo(move |_| is_dragging.get() || is_zooming.get());

    // Stored state (non-reactive)
    let initial_image_data = store_value::<Option<ImageData>>(None);
    let drag_start = store_value::<Option<(f64, f64)>>(None);
    let accumulated_offset = store_value((0.0, 0.0));
    let accumulated_zoom = store_value(1.0);
    let zoom_center = store_value::<Option<(f64, f64)>>(None);
    let animation_frame_id = store_value::<Option<i32>>(None);
    let timeout_handle = store_value::<Option<TimeoutHandle>>(None);

    // Reset function
    let reset = {
        let is_dragging = is_dragging.clone();
        let is_zooming = is_zooming.clone();
        Box::new(move || {
            is_dragging.set(false);
            is_zooming.set(false);
            initial_image_data.set_value(None);
            drag_start.set_value(None);
            accumulated_offset.set_value((0.0, 0.0));
            accumulated_zoom.set_value(1.0);
            zoom_center.set_value(None);
            animation_frame_id.set_value(None);
            timeout_handle.set_value(None);
        })
    };

    InteractionHandle {
        is_interacting: is_interacting.into(),
        reset,
    }
}
```

**Step 4: Add TimeoutHandle type**

Add near top of file after imports:
```rust
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};

type TimeoutHandle = UseTimeoutFnReturn;
```

**Step 5: Run test to verify it passes**

Run: `cargo test --target wasm32-unknown-unknown`
Expected: PASS (hook creates but doesn't do anything yet)

**Step 6: Commit**

```bash
git add src/hooks/use_canvas_interaction.rs
git commit -m "feat: implement core hook structure with state management"
```

---

## Task 5: Implement preview rendering loop

**Files:**
- Modify: `src/hooks/use_canvas_interaction.rs` (add preview loop)

**Step 1: Add helper to capture ImageData**

Add before `use_canvas_interaction` function:

```rust
fn capture_canvas_image_data(canvas: &HtmlCanvasElement) -> Result<ImageData, JsValue> {
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("Failed to get 2D context"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    context.get_image_data(0.0, 0.0, canvas.width() as f64, canvas.height() as f64)
}
```

**Step 2: Add preview render function**

Add before `use_canvas_interaction` function:

```rust
fn render_preview(
    canvas: &HtmlCanvasElement,
    image_data: &ImageData,
    offset: (f64, f64),
    zoom: f64,
    zoom_center: Option<(f64, f64)>,
) -> Result<(), JsValue> {
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("Failed to get 2D context"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    // Clear canvas
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    // Apply transformation matrix
    let matrix = build_transform_matrix(offset, zoom, zoom_center);
    context.set_transform(
        matrix[0][0],
        matrix[1][0],
        matrix[0][1],
        matrix[1][1],
        matrix[0][2],
        matrix[1][2],
    )?;

    // Draw the transformed image
    context.put_image_data(image_data, 0.0, 0.0)?;

    // Reset transform
    context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)?;

    Ok(())
}
```

**Step 3: Add animation loop setup in hook**

Inside `use_canvas_interaction`, add after state declarations:

```rust
// Animation loop for preview rendering
let start_preview_loop = {
    let canvas_ref = canvas_ref.clone();
    let initial_image_data = initial_image_data.clone();
    let accumulated_offset = accumulated_offset.clone();
    let accumulated_zoom = accumulated_zoom.clone();
    let zoom_center = zoom_center.clone();
    let is_interacting = is_interacting.clone();
    let animation_frame_id = animation_frame_id.clone();

    move || {
        if animation_frame_id.get_value().is_some() {
            return; // Already running
        }

        let canvas_ref = canvas_ref.clone();
        let initial_image_data = initial_image_data.clone();
        let accumulated_offset = accumulated_offset.clone();
        let accumulated_zoom = accumulated_zoom.clone();
        let zoom_center = zoom_center.clone();
        let is_interacting = is_interacting.clone();
        let animation_frame_id = animation_frame_id.clone();

        let render_frame = Closure::wrap(Box::new(move || {
            if !is_interacting.get() {
                animation_frame_id.set_value(None);
                return;
            }

            if let Some(canvas) = canvas_ref.get() {
                if let Some(image_data) = initial_image_data.get_value() {
                    let offset = accumulated_offset.get_value();
                    let zoom = accumulated_zoom.get_value();
                    let center = zoom_center.get_value();

                    let _ = render_preview(&canvas, &image_data, offset, zoom, center);
                }
            }

            // Continue loop
            if is_interacting.get() {
                let window = web_sys::window().unwrap();
                let id = window
                    .request_animation_frame(render_frame.as_ref().unchecked_ref())
                    .unwrap();
                animation_frame_id.set_value(Some(id));
            }
        }) as Box<dyn Fn()>);

        let window = web_sys::window().unwrap();
        let id = window
            .request_animation_frame(render_frame.as_ref().unchecked_ref())
            .unwrap();
        animation_frame_id.set_value(Some(id));

        render_frame.forget();
    }
};
```

**Step 4: Check compilation**

Run: `cargo check`
Expected: May have errors with closure lifetime - this is expected, we'll fix in next step

**Step 5: Fix using RequestAnimationFrame from leptos-use**

Replace the animation loop with leptos-use helper:

```rust
use leptos_use::use_raf_fn;

// Inside use_canvas_interaction, replace start_preview_loop with:
let should_render = create_memo(move |_| {
    is_interacting.get() && initial_image_data.get_value().is_some()
});

use_raf_fn(move |_| {
    if !should_render.get() {
        return;
    }

    if let Some(canvas) = canvas_ref.get() {
        if let Some(image_data) = initial_image_data.get_value() {
            let offset = accumulated_offset.get_value();
            let zoom = accumulated_zoom.get_value();
            let center = zoom_center.get_value();

            let _ = render_preview(&canvas, &image_data, offset, zoom, center);
        }
    }
});
```

**Step 6: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 7: Commit**

```bash
git add src/hooks/use_canvas_interaction.rs
git commit -m "feat: add preview rendering loop with RAF"
```

---

## Task 6: Implement pointer drag handling

**Files:**
- Modify: `src/hooks/use_canvas_interaction.rs` (add event handlers)

**Step 1: Add interaction start helper**

Add inside `use_canvas_interaction`, before the return statement:

```rust
let start_interaction = {
    let canvas_ref = canvas_ref.clone();
    let initial_image_data = initial_image_data.clone();
    let accumulated_offset = accumulated_offset.clone();
    let accumulated_zoom = accumulated_zoom.clone();
    let zoom_center = zoom_center.clone();

    move || {
        if let Some(canvas) = canvas_ref.get() {
            if let Ok(image_data) = capture_canvas_image_data(&canvas) {
                initial_image_data.set_value(Some(image_data));
                accumulated_offset.set_value((0.0, 0.0));
                accumulated_zoom.set_value(1.0);
                zoom_center.set_value(None);
            }
        }
    }
};
```

**Step 2: Add pointerdown handler**

Add event listener setup before return:

```rust
use leptos::ev::{pointerdown, pointermove, pointerup};

// Pointer down handler
let on_pointer_down = {
    let is_dragging = is_dragging.clone();
    let drag_start = drag_start.clone();
    let start_interaction = start_interaction.clone();

    move |ev: web_sys::PointerEvent| {
        ev.prevent_default();

        start_interaction();
        is_dragging.set(true);
        drag_start.set_value(Some((ev.client_x() as f64, ev.client_y() as f64)));
    }
};

// Set up event listener
create_effect({
    let canvas_ref = canvas_ref.clone();
    move |_| {
        if let Some(canvas) = canvas_ref.get() {
            let _ = canvas.add_event_listener_with_callback(
                "pointerdown",
                on_pointer_down.as_ref().unchecked_ref(),
            );
        }
    }
});
```

**Step 3: Add pointermove handler**

Add after pointerdown:

```rust
// Pointer move handler
let on_pointer_move = {
    let is_dragging = is_dragging.clone();
    let drag_start = drag_start.clone();
    let accumulated_offset = accumulated_offset.clone();

    move |ev: web_sys::PointerEvent| {
        if !is_dragging.get() {
            return;
        }

        if let Some(start) = drag_start.get_value() {
            let current_x = ev.client_x() as f64;
            let current_y = ev.client_y() as f64;
            let offset = (current_x - start.0, current_y - start.1);
            accumulated_offset.set_value(offset);
        }
    }
};
```

**Step 4: Add pointerup handler**

Add after pointermove:

```rust
// Pointer up handler
let on_pointer_up = {
    let is_dragging = is_dragging.clone();

    move |_ev: web_sys::PointerEvent| {
        is_dragging.set(false);
        // Timeout will be started in next task
    }
};
```

**Step 5: Check compilation**

Run: `cargo check`
Expected: Errors about event listener setup - we'll use leptos event handlers instead

**Step 6: Fix using Leptos event directives**

Replace event listener setup with leptos approach - we'll attach these in Task 8 when integrating with components. For now, store the closures:

```rust
// Store event handlers for consumer to attach
let pointer_handlers = store_value((
    on_pointer_down,
    on_pointer_move,
    on_pointer_up,
));
```

**Step 7: Verify compilation**

Run: `cargo check`
Expected: No errors (handlers stored but not yet attached)

**Step 8: Commit**

```bash
git add src/hooks/use_canvas_interaction.rs
git commit -m "feat: add pointer drag event handlers"
```

---

## Task 7: Implement wheel zoom handling

**Files:**
- Modify: `src/hooks/use_canvas_interaction.rs` (add wheel handler)

**Step 1: Add wheel event handler**

Add after pointer handlers:

```rust
// Wheel handler for zoom
let on_wheel = {
    let canvas_ref = canvas_ref.clone();
    let is_zooming = is_zooming.clone();
    let is_dragging = is_dragging.clone();
    let start_interaction = start_interaction.clone();
    let accumulated_zoom = accumulated_zoom.clone();
    let zoom_center = zoom_center.clone();

    move |ev: web_sys::WheelEvent| {
        ev.prevent_default();

        // Start interaction if not already started
        if !is_dragging.get() && !is_zooming.get() {
            start_interaction();
        }

        is_zooming.set(true);

        // Calculate zoom factor from wheel delta
        let delta = ev.delta_y();
        let zoom_multiplier = (-delta * ZOOM_SENSITIVITY).exp();
        let current_zoom = accumulated_zoom.get_value();
        accumulated_zoom.set_value(current_zoom * zoom_multiplier);

        // Store zoom center (pointer position relative to canvas)
        if let Some(canvas) = canvas_ref.get() {
            let rect = canvas.get_bounding_client_rect();
            let x = ev.client_x() as f64 - rect.left();
            let y = ev.client_y() as f64 - rect.top();
            zoom_center.set_value(Some((x, y)));
        }

        // Timeout will be restarted (next task)
    }
};
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/hooks/use_canvas_interaction.rs
git commit -m "feat: add wheel zoom event handler"
```

---

## Task 8: Implement interaction timeout and end callback

**Files:**
- Modify: `src/hooks/use_canvas_interaction.rs` (add timeout logic)

**Step 1: Add stop interaction handler**

Add before return statement:

```rust
let stop_interaction = {
    let is_dragging = is_dragging.clone();
    let is_zooming = is_zooming.clone();
    let accumulated_offset = accumulated_offset.clone();
    let accumulated_zoom = accumulated_zoom.clone();
    let zoom_center = zoom_center.clone();
    let initial_image_data = initial_image_data.clone();
    let on_interaction_end = on_interaction_end.clone();

    move || {
        // Don't stop if still dragging
        if is_dragging.get() {
            return;
        }

        is_zooming.set(false);

        // Build final result
        let offset = accumulated_offset.get_value();
        let zoom = accumulated_zoom.get_value();
        let center = zoom_center.get_value();
        let matrix = build_transform_matrix(offset, zoom, center);

        let result = TransformResult {
            offset_x: offset.0,
            offset_y: offset.1,
            zoom_factor: zoom,
            matrix,
        };

        // Clear state
        initial_image_data.set_value(None);
        accumulated_offset.set_value((0.0, 0.0));
        accumulated_zoom.set_value(1.0);
        zoom_center.set_value(None);

        // Fire callback
        on_interaction_end(result);
    }
};
```

**Step 2: Add timeout using leptos-use**

Add after stop_interaction:

```rust
let UseTimeoutFnReturn { start: start_timeout, stop: stop_timeout, .. } =
    use_timeout_fn(stop_interaction, INTERACTION_TIMEOUT_MS);

// Restart timeout on any interaction
create_effect(move |_| {
    if is_interacting.get() {
        stop_timeout();
    }
});

// Start timeout when interaction becomes inactive
create_effect(move |_| {
    if !is_dragging.get() && is_zooming.get() {
        start_timeout();
    }
});
```

**Step 3: Update pointer up to trigger timeout**

Modify `on_pointer_up`:

```rust
let on_pointer_up = {
    let is_dragging = is_dragging.clone();
    let start_timeout = start_timeout.clone();

    move |_ev: web_sys::PointerEvent| {
        is_dragging.set(false);
        start_timeout();
    }
};
```

**Step 4: Update wheel handler to restart timeout**

Modify end of `on_wheel`:

```rust
// ... existing wheel code ...

        // Restart timeout
        stop_timeout();
        start_timeout();
    }
};
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Errors about closure cloning - need to restructure

**Step 6: Fix by restructuring timeout integration**

The leptos-use timeout needs to be restructured. Replace timeout code with manual approach:

```rust
use wasm_bindgen::JsCast;

// Manual timeout tracking
let timeout_id = store_value::<Option<i32>>(None);

let restart_timeout = {
    let timeout_id = timeout_id.clone();
    let stop_interaction = stop_interaction.clone();

    move || {
        // Clear existing timeout
        if let Some(id) = timeout_id.get_value() {
            web_sys::window().unwrap().clear_timeout_with_handle(id);
        }

        // Set new timeout
        let callback = Closure::once(move || {
            stop_interaction();
        });

        let id = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                INTERACTION_TIMEOUT_MS,
            )
            .unwrap();

        callback.forget();
        timeout_id.set_value(Some(id));
    }
};
```

**Step 7: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 8: Commit**

```bash
git add src/hooks/use_canvas_interaction.rs
git commit -m "feat: implement interaction timeout and end callback"
```

---

## Task 9: Export handler functions in InteractionHandle

**Files:**
- Modify: `src/hooks/use_canvas_interaction.rs` (update InteractionHandle)

**Step 1: Update InteractionHandle structure**

Modify the struct definition:

```rust
pub struct InteractionHandle {
    pub is_interacting: ReadSignal<bool>,
    pub on_pointer_down: Box<dyn Fn(web_sys::PointerEvent)>,
    pub on_pointer_move: Box<dyn Fn(web_sys::PointerEvent)>,
    pub on_pointer_up: Box<dyn Fn(web_sys::PointerEvent)>,
    pub on_wheel: Box<dyn Fn(web_sys::WheelEvent)>,
    pub reset: Box<dyn Fn()>,
}
```

**Step 2: Update hook return statement**

Modify the return at end of `use_canvas_interaction`:

```rust
InteractionHandle {
    is_interacting: is_interacting.into(),
    on_pointer_down: Box::new(on_pointer_down),
    on_pointer_move: Box::new(on_pointer_move),
    on_pointer_up: Box::new(on_pointer_up),
    on_wheel: Box::new(on_wheel),
    reset,
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src/hooks/use_canvas_interaction.rs
git commit -m "feat: export event handlers in InteractionHandle"
```

---

## Task 10: Create example integration in test_image component

**Files:**
- Modify: `src/components/test_image.rs` (add hook usage example)

**Step 1: Read current test_image component**

Run: `cargo check` then examine structure

**Step 2: Add use_canvas_interaction integration**

Add to component (example structure):

```rust
use crate::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};

// Inside component function:
let canvas_ref = create_node_ref::<leptos::html::Canvas>();

let handle = use_canvas_interaction(
    canvas_ref,
    move |result: TransformResult| {
        log::info!(
            "Interaction ended: offset=({}, {}), zoom={}, matrix={:?}",
            result.offset_x,
            result.offset_y,
            result.zoom_factor,
            result.matrix
        );
        // TODO: Trigger full re-render with transformation
    },
);

view! {
    <canvas
        node_ref=canvas_ref
        on:pointerdown=move |ev| (handle.on_pointer_down)(ev)
        on:pointermove=move |ev| (handle.on_pointer_move)(ev)
        on:pointerup=move |ev| (handle.on_pointer_up)(ev)
        on:wheel=move |ev| (handle.on_wheel)(ev)
        style="border: 1px solid black; cursor: grab;"
    />
    <Show when=move || handle.is_interacting.get()>
        <div>"Interacting..."</div>
    </Show>
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 4: Manual browser test**

Run: `trunk serve`
Navigate to: `http://localhost:8080`
Test: Drag canvas, use mouse wheel, verify console logs
Expected: Interaction logs appear after 1.5s of inactivity

**Step 5: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: integrate canvas interaction hook in test component"
```

---

## Task 11: Add documentation and usage example

**Files:**
- Create: `docs/canvas-interaction-hook.md`
- Modify: `src/hooks/use_canvas_interaction.rs` (add rustdoc)

**Step 1: Add rustdoc to public API**

Add doc comments to `src/hooks/use_canvas_interaction.rs`:

```rust
/// Transformation result returned when user interaction ends (after 1.5s of inactivity)
///
/// Contains both discrete values and a pre-computed affine transformation matrix.
/// All coordinates are in screen pixel space.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformResult {
    /// Horizontal offset in pixels
    pub offset_x: f64,
    /// Vertical offset in pixels
    pub offset_y: f64,
    /// Cumulative zoom factor (1.0 = no zoom, 2.0 = 2x zoom, 0.5 = 0.5x zoom)
    pub zoom_factor: f64,
    /// 2D affine transformation matrix [3x3] encoding offset + zoom
    pub matrix: [[f64; 3]; 3],
}

/// Handle returned by the canvas interaction hook
///
/// Provides event handlers to attach to canvas element and reactive interaction state.
pub struct InteractionHandle {
    /// Reactive signal indicating whether user is currently interacting
    pub is_interacting: ReadSignal<bool>,
    /// Event handler for pointerdown events
    pub on_pointer_down: Box<dyn Fn(web_sys::PointerEvent)>,
    /// Event handler for pointermove events
    pub on_pointer_move: Box<dyn Fn(web_sys::PointerEvent)>,
    /// Event handler for pointerup events
    pub on_pointer_up: Box<dyn Fn(web_sys::PointerEvent)>,
    /// Event handler for wheel events (zoom)
    pub on_wheel: Box<dyn Fn(web_sys::WheelEvent)>,
    /// Reset all interaction state
    pub reset: Box<dyn Fn()>,
}

/// Generic canvas interaction hook providing real-time pan/zoom preview
///
/// Designed for canvases where full re-renders are expensive (seconds to hours).
/// Captures canvas ImageData on interaction start, provides real-time preview
/// using pixel transformations, and fires callback after 1.5s of inactivity.
///
/// # Example
///
/// ```rust
/// let canvas_ref = create_node_ref::<leptos::html::Canvas>();
///
/// let handle = use_canvas_interaction(
///     canvas_ref,
///     move |result: TransformResult| {
///         // Convert pixel transform to domain coordinates
///         // Trigger expensive full re-render
///     },
/// );
///
/// view! {
///     <canvas
///         node_ref=canvas_ref
///         on:pointerdown=move |ev| (handle.on_pointer_down)(ev)
///         on:pointermove=move |ev| (handle.on_pointer_move)(ev)
///         on:pointerup=move |ev| (handle.on_pointer_up)(ev)
///         on:wheel=move |ev| (handle.on_wheel)(ev)
///     />
/// }
/// ```
///
/// # Arguments
///
/// * `canvas_ref` - Leptos NodeRef to canvas element
/// * `on_interaction_end` - Callback fired when interaction ends (1.5s inactivity)
///
/// # Returns
///
/// `InteractionHandle` with event handlers and interaction state signal
pub fn use_canvas_interaction<F>(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    on_interaction_end: F,
) -> InteractionHandle
where
    F: Fn(TransformResult) + 'static,
{
    // ... existing implementation ...
}
```

**Step 2: Create usage documentation**

File: `docs/canvas-interaction-hook.md`

```markdown
# Canvas Interaction Hook

## Overview

`use_canvas_interaction` is a generic Leptos hook for adding real-time pan/zoom interaction to HTML canvas elements. It's designed for applications where full canvas re-renders are expensive (seconds to hours), such as:

- High-precision fractal rendering
- Large geographic maps
- Complex scientific visualizations
- Game worlds with procedural generation

## How It Works

1. **Interaction Start**: User drags (pointerdown) or zooms (wheel)
2. **ImageData Capture**: Hook snapshots current canvas ImageData
3. **Real-time Preview**: Animation loop applies pixel transformations (translate/scale) to the snapshot
4. **Debounced End**: After 1.5s of inactivity, fires callback with final transformation
5. **Consumer Re-render**: Callback converts pixel transform to domain coordinates and triggers full re-render

## API

### `use_canvas_interaction(canvas_ref, on_interaction_end)`

**Parameters:**
- `canvas_ref: NodeRef<leptos::html::Canvas>` - Reference to canvas element
- `on_interaction_end: Fn(TransformResult)` - Callback fired when interaction ends

**Returns:** `InteractionHandle` with:
- `is_interacting: ReadSignal<bool>` - Reactive interaction state
- Event handlers: `on_pointer_down`, `on_pointer_move`, `on_pointer_up`, `on_wheel`
- `reset: Fn()` - Clear all interaction state

### `TransformResult`

```rust
pub struct TransformResult {
    pub offset_x: f64,        // Pixels dragged horizontally
    pub offset_y: f64,        // Pixels dragged vertically
    pub zoom_factor: f64,     // Cumulative zoom (1.0 = no zoom)
    pub matrix: [[f64; 3]; 3], // 2D affine transformation matrix
}
```

## Usage Example

```rust
use fractalwonder::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};

#[component]
pub fn MyCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    let handle = use_canvas_interaction(
        canvas_ref,
        move |result: TransformResult| {
            // Convert pixel coordinates to your domain
            let domain_offset = pixel_to_domain(result.offset_x, result.offset_y);

            // Trigger expensive re-render
            render_full_quality(domain_offset, result.zoom_factor);
        },
    );

    view! {
        <canvas
            node_ref=canvas_ref
            width="800"
            height="600"
            on:pointerdown=move |ev| (handle.on_pointer_down)(ev)
            on:pointermove=move |ev| (handle.on_pointer_move)(ev)
            on:pointerup=move |ev| (handle.on_pointer_up)(ev)
            on:wheel=move |ev| (handle.on_wheel)(ev)
            style="cursor: grab; touch-action: none;"
        />
        <Show when=move || handle.is_interacting.get()>
            <div class="interaction-indicator">"Navigating..."</div>
        </Show>
    }
}
```

## Implementation Details

- **Coordinate System**: Works purely in pixel space (no domain knowledge)
- **ImageData Memory**: Stores snapshot only during active interaction
- **Animation Loop**: Uses requestAnimationFrame for 60fps preview
- **Zoom Center**: Zoom is centered on current pointer position
- **Timeout**: 1.5s debounce, restarts on any new pointer/wheel event
- **State Management**: Leptos signals for reactive state, StoredValue for internal bookkeeping

## Testing

Unit tests for matrix math:
```bash
cargo test build_transform_matrix
```

Browser integration tests:
```bash
cargo test --target wasm32-unknown-unknown
```

Manual testing:
```bash
trunk serve
# Navigate to http://localhost:8080
# Test drag, zoom, verify console logs
```

## Architecture Notes

This hook is **framework-agnostic in design** but **Leptos-specific in implementation**. All transformation logic operates in pixel space with no assumptions about the underlying data (fractals, maps, etc.), making it suitable for any canvas-based application where re-renders are expensive.
```

**Step 3: Verify documentation builds**

Run: `cargo doc --no-deps --open`
Expected: Documentation opens in browser with rustdoc comments

**Step 4: Commit**

```bash
git add docs/canvas-interaction-hook.md src/hooks/use_canvas_interaction.rs
git commit -m "docs: add comprehensive documentation for canvas interaction hook"
```

---

## Task 12: Final verification and cleanup

**Files:**
- All modified files

**Step 1: Run all tests**

Run: `cargo test --workspace --all-targets --all-features`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 4: Build release**

Run: `cargo build --release`
Expected: Successful build

**Step 5: Manual browser test**

Run: `trunk serve --open`
Test checklist:
- [ ] Canvas renders
- [ ] Drag moves image in real-time
- [ ] Mouse wheel zooms centered on pointer
- [ ] Dragging image off-screen shows gap, dragging back recovers image
- [ ] After 1.5s of inactivity, console shows TransformResult
- [ ] Starting new interaction within 1.5s continues (no callback)
- [ ] Interaction indicator appears/disappears correctly

**Step 6: Final commit**

```bash
git add -A
git commit -m "feat: complete canvas interaction hook implementation

- Generic pixel-space pan/zoom with real-time preview
- Works with any canvas where full re-renders are expensive
- 1.5s debounce timeout before triggering callback
- Returns both discrete values and transformation matrix
- Fully documented with examples and tests"
```

---

## Completion Checklist

- [ ] Task 1: Module structure created
- [ ] Task 2: Matrix transformation tests pass
- [ ] Task 3: Web-sys features added
- [ ] Task 4: Core hook implemented
- [ ] Task 5: Preview rendering loop working
- [ ] Task 6: Pointer drag handlers implemented
- [ ] Task 7: Wheel zoom handler implemented
- [ ] Task 8: Timeout and callback working
- [ ] Task 9: Event handlers exported
- [ ] Task 10: Example integration created
- [ ] Task 11: Documentation complete
- [ ] Task 12: All tests pass, manual testing complete

## Notes

- Follow TDD: Write test → See it fail → Implement → See it pass → Commit
- Keep commits small and focused (one per task step)
- If you encounter issues with closures/lifetimes, consider using `store_value` instead of signals
- The hook is intentionally generic - no fractal-specific logic
- Browser testing is critical - use chrome-devtools MCP for automated interaction testing
