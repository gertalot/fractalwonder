# Live UI Coordinate Updates Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable real-time UI coordinate updates during drag/zoom interactions without triggering expensive renders.

**Architecture:** Decouple viewport updates (cheap) from rendering (expensive) by adding a second callback to `use_canvas_interaction` that fires during interaction. Both callbacks update the viewport signal, but rendering is gated by the `is_interacting` flag.

**Tech Stack:** Rust, Leptos (reactive signals), WebAssembly

---

## Task 1: Add second callback parameter to use_canvas_interaction hook

**Files:**
- Modify: `fractalwonder-ui/src/hooks/use_canvas_interaction.rs:166-172`

**Context:** The hook currently accepts one callback `on_interaction_end` that fires after 1.5s timeout. We need to add a second callback `on_interaction` that fires during interaction.

**Step 1: Update function signature**

Replace lines 166-172 with:

```rust
pub fn use_canvas_interaction<F, G>(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    on_interaction: F,
    on_interaction_end: G,
) -> InteractionHandle
where
    F: Fn(TransformResult) + 'static + Clone,
    G: Fn(TransformResult) + 'static + Clone,
{
```

**Step 2: Update documentation example**

Replace lines 128-156 (the doc comment example) with:

```rust
/// Generic canvas interaction hook providing real-time pan/zoom preview
///
/// Designed for canvases where full re-renders are expensive (seconds to hours).
/// Captures canvas ImageData on interaction start, provides real-time preview
/// using pixel transformations, and fires callbacks during and after interaction.
///
/// # Example
///
/// ```rust,no_run
/// use leptos::*;
/// use fractalwonder_ui::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};
///
/// #[component]
/// pub fn MyCanvas() -> impl IntoView {
///     let canvas_ref = create_node_ref::<leptos::html::Canvas>();
///
///     let handle = use_canvas_interaction(
///         canvas_ref,
///         move |result: TransformResult| {
///             // Fires during interaction - update UI coordinates
///         },
///         move |result: TransformResult| {
///             // Fires after interaction ends - trigger expensive render
///         },
///     );
///
///     view! {
///         <canvas node_ref=canvas_ref class="w-full h-full" />
///     }
/// }
/// ```
///
/// # Arguments
///
/// * `canvas_ref` - Leptos NodeRef to canvas element
/// * `on_interaction` - Callback fired during interaction (on every transform change)
/// * `on_interaction_end` - Callback fired when interaction ends (1.5s inactivity)
///
/// # Returns
///
/// `InteractionHandle` with interaction state signal. All event listeners are attached internally.
```

**Step 3: Run format and clippy**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Expected: Format succeeds, clippy may show unused parameter warnings (we'll fix in next task)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/hooks/use_canvas_interaction.rs
git commit -m "feat: add on_interaction callback parameter to use_canvas_interaction

Adds second callback that will fire during interaction for live UI updates.
Hook signature now accepts both on_interaction and on_interaction_end.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Store on_interaction callback and create helper to build TransformResult

**Files:**
- Modify: `fractalwonder-ui/src/hooks/use_canvas_interaction.rs:252-253`
- Modify: `fractalwonder-ui/src/hooks/use_canvas_interaction.rs:262-308`

**Context:** We need to extract the TransformResult building logic so it can be called both during interaction and at the end.

**Step 1: Store on_interaction callback**

After line 252, add:

```rust
    let on_interaction_end = store_value(on_interaction_end);
    let on_interaction = store_value(on_interaction);
```

So lines 252-253 become:

```rust
    // Stop interaction handler - builds TransformResult and fires callback
    let on_interaction_end = store_value(on_interaction_end);
    let on_interaction = store_value(on_interaction);
```

**Step 2: Extract TransformResult building into helper function**

Replace lines 262-308 (inside `stop_interaction`) with a call to a new helper. First, add the helper before `stop_interaction` (around line 251):

```rust
    // Helper: Build TransformResult from current transform sequence
    let build_transform_result = move || -> Option<TransformResult> {
        let sequence = transform_sequence.get_value();

        if sequence.is_empty() {
            return None;
        }

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "build_transform_result: sequence has {} transforms: {:?}",
            sequence.len(),
            sequence
        )));

        let composed_matrix: Mat3 = compose_affine_transformations(sequence);

        // Extract center-relative offset and zoom from the composed matrix
        // The matrix is in the form: [[zoom, 0, offset_x], [0, zoom, offset_y], [0, 0, 1]]
        let zoom_factor = composed_matrix.data[0][0];
        let absolute_offset_x = composed_matrix.data[0][2];
        let absolute_offset_y = composed_matrix.data[1][2];

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "Composed: zoom={}, offset=({}, {})",
            zoom_factor, absolute_offset_x, absolute_offset_y
        )));

        // Convert absolute pixel offset to center-relative offset
        // This makes the values more intuitive: (0, 0) means we zoomed at canvas center
        let canvas_ref = canvas_ref_stored.get_value();
        let (center_relative_x, center_relative_y) =
            if let Some(canvas) = canvas_ref.get_untracked() {
                let canvas_center_x = canvas.width() as f64 / 2.0;
                let canvas_center_y = canvas.height() as f64 / 2.0;

                // Offset is relative to top-left (0, 0), convert to relative to center
                (
                    absolute_offset_x - canvas_center_x * (1.0 - zoom_factor),
                    absolute_offset_y - canvas_center_y * (1.0 - zoom_factor),
                )
            } else {
                (absolute_offset_x, absolute_offset_y)
            };

        Some(TransformResult {
            offset_x: center_relative_x,
            offset_y: center_relative_y,
            zoom_factor,
            matrix: composed_matrix.data,
        })
    };

    // Stop interaction handler - builds TransformResult and fires callback
    let on_interaction_end = store_value(on_interaction_end);
    let on_interaction = store_value(on_interaction);
    let build_transform_result_stored = store_value(build_transform_result);
```

**Step 3: Update stop_interaction to use helper**

Replace lines 262-308 in `stop_interaction` with:

```rust
        // Build and fire transform result
        let build_fn = build_transform_result_stored.get_value();
        if let Some(result) = build_fn() {
            // Clear state
            initial_image_data.set_value(None);
            base_offset.set_value((0.0, 0.0));
            current_drag_offset.set_value((0.0, 0.0));
            accumulated_zoom.set_value(1.0);
            zoom_center.set_value(None);
            transform_sequence.set_value(Vec::new());

            // Fire callback
            on_interaction_end.with_value(|cb| cb(result));
        }
```

**Step 4: Run format and clippy**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Expected: Format succeeds, clippy should pass (on_interaction still unused but stored)

**Step 5: Run check to verify compiles**

```bash
cargo check --workspace --all-targets --all-features
```

Expected: SUCCESS (no compile errors)

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/hooks/use_canvas_interaction.rs
git commit -m "refactor: extract TransformResult building into reusable helper

Extract transform result building logic from stop_interaction into
build_transform_result helper that can be called during interaction.
Store on_interaction callback for use in next commit.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Call on_interaction during transform updates

**Files:**
- Modify: `fractalwonder-ui/src/hooks/use_canvas_interaction.rs` (around lines 190-248, in the RAF loop and event handlers)

**Context:** We need to call `on_interaction` with the current TransformResult whenever the transform changes during interaction. This happens in the `requestAnimationFrame` loop that renders the preview.

**Step 1: Find the RAF loop that renders preview**

Search for the code that calls `render_transformed_canvas`. This should be around line 197-233. The loop should look like:

```rust
    let _ = use_raf_fn(move |_| {
        if !is_interacting.get() {
            return;
        }

        // ... rendering logic ...
        let _ = render_transformed_canvas(...);
    });
```

**Step 2: Add on_interaction call after render in RAF loop**

After the `render_transformed_canvas` call (and its error handling), add:

```rust
        // Fire on_interaction callback with current transform
        let build_fn = build_transform_result_stored.get_value();
        if let Some(result) = build_fn() {
            on_interaction.with_value(|cb| cb(result));
        }
```

**Step 3: Run format and clippy**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Expected: Format succeeds, clippy passes

**Step 4: Run check**

```bash
cargo check --workspace --all-targets --all-features
```

Expected: SUCCESS

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/hooks/use_canvas_interaction.rs
git commit -m "feat: fire on_interaction callback during transform updates

Call on_interaction with current TransformResult during the RAF preview loop.
This enables live UI updates during drag/zoom interactions.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Update InteractiveCanvas to extract viewport update logic

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs:22-39`

**Context:** Currently InteractiveCanvas has a single callback passed to the hook. We need to extract the viewport update logic into a reusable closure and pass it as both callbacks.

**Step 1: Extract viewport update logic into reusable closure**

Replace lines 22-39 with:

```rust
    // Extract viewport update logic - will be used for both callbacks
    let update_viewport_from_transform = move |transform_result| {
        if let Some(canvas_el) = canvas_ref.get_untracked() {
            let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();
            let width = canvas.width();
            let height = canvas.height();

            set_viewport.update(|vp| {
                *vp = crate::rendering::apply_pixel_transform_to_viewport(
                    vp,
                    &natural_bounds.get_untracked(),
                    &transform_result,
                    width,
                    height,
                );
            });
        }
    };

    // Canvas interaction hook - both callbacks update viewport
    let interaction = use_canvas_interaction(
        canvas_ref,
        update_viewport_from_transform.clone(), // During interaction
        update_viewport_from_transform,          // At end
    );
```

**Step 2: Run format and clippy**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Expected: Format succeeds, clippy passes

**Step 3: Run check**

```bash
cargo check --workspace --all-targets --all-features
```

Expected: SUCCESS

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat: pass viewport update logic to both hook callbacks

Extract viewport update into reusable closure and pass to both
on_interaction and on_interaction_end callbacks. Viewport now updates
during interaction, not just at the end.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Gate rendering by is_interacting flag

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs:61-75`

**Context:** The render effect currently fires whenever viewport changes. We need to add a check to skip rendering when `is_interacting` is true.

**Step 1: Add is_interacting check to render effect**

Replace lines 61-75 with:

```rust
    // Effect: Render when canvas_renderer OR viewport changes, but NOT during interaction
    create_effect(move |_| {
        let vp = viewport.get();
        canvas_renderer.track();
        let interacting = interaction.is_interacting.get();

        // Only render when not interacting
        if !interacting {
            if let Some(canvas_el) = canvas_ref.get() {
                let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();

                let start = web_sys::window().unwrap().performance().unwrap().now();

                canvas_renderer.with(|cr| cr.render(&vp, canvas));

                let elapsed = web_sys::window().unwrap().performance().unwrap().now() - start;
                set_render_time_ms.set(Some(elapsed));
            }
        }
    });
```

**Step 2: Run format and clippy**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

Expected: Format succeeds, clippy passes

**Step 3: Run check**

```bash
cargo check --workspace --all-targets --all-features
```

Expected: SUCCESS

**Step 4: Run full test suite**

```bash
cargo test --workspace --all-targets --all-features -- --nocapture
```

Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat: gate rendering during interaction

Skip expensive render when is_interacting is true. Viewport updates
fire the effect, but rendering only happens when interaction ends.
This completes the decoupling of viewport updates from rendering.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: Manual testing in browser

**Context:** Verify the implementation works correctly in the browser.

**Step 1: Ensure trunk serve is running**

Check if trunk is already running on localhost:8080. If not, start it:

```bash
trunk serve
```

Expected: Server starts on http://localhost:8080

**Step 2: Open browser and test drag interaction**

Manual test checklist:
1. Open http://localhost:8080
2. Start dragging the canvas
3. **Verify:** Coordinates in UI panel update in real-time during drag
4. **Verify:** Render time does NOT update during drag
5. Continue dragging for a few seconds
6. Release mouse
7. **Verify:** After 1.5 seconds, render happens (render time updates)
8. **Verify:** Final coordinates match the dragged position

**Step 3: Test zoom interaction**

Manual test checklist:
1. Scroll to zoom with mouse wheel
2. **Verify:** Coordinates update in real-time during zoom
3. **Verify:** Render time does NOT update during zoom
4. Stop scrolling
5. **Verify:** After 1.5 seconds, render happens (render time updates)
6. **Verify:** Final coordinates and zoom match

**Step 4: Test edge cases**

Manual test checklist:
1. Drag, release, drag again within 1.5 seconds - verify coordinates stay live
2. Resize browser window during interaction - verify no crashes
3. Switch renderer during interaction - verify coordinates update correctly

**Step 5: Document test results**

If all tests pass, proceed to next task. If any tests fail, note the failure and debug before proceeding.

---

## Task 7: Final verification and completion

**Step 1: Run full build**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features -- --nocapture
```

Expected: All checks pass

**Step 2: Build release version**

```bash
trunk build --release
```

Expected: Build succeeds, output in dist/

**Step 3: Review all commits**

```bash
git log --oneline -7
```

Expected: Should see 5 commits for this feature:
1. Add on_interaction callback parameter
2. Extract TransformResult building
3. Fire on_interaction during transforms
4. Pass viewport update to both callbacks
5. Gate rendering during interaction

**Step 4: Final commit with summary**

```bash
git commit --allow-empty -m "feat: complete live UI coordinate updates during interaction

Summary of changes:
- Added on_interaction callback to use_canvas_interaction hook
- Extracted TransformResult building into reusable helper
- Fire on_interaction during RAF preview loop
- InteractiveCanvas passes viewport update to both callbacks
- Gated rendering by is_interacting flag

Result: UI coordinates update in real-time during drag/zoom, while
expensive renders only happen when interaction ends.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Implementation Complete

All tasks completed. The feature is ready for:
- Merge to main (or create PR)
- Deployment testing
- User feedback

**Key achievements:**
- Real-time UI coordinate updates during interaction
- No performance degradation (renders only at end)
- Clean abstraction boundaries maintained
- Hook remains generic and reusable
