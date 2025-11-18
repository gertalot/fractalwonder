# Dynamic Tile Sizing Based on Zoom Level

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make tiles smaller at extreme zoom levels (>1e10) for better progressive rendering UX

**Architecture:** Calculate tile size dynamically based on zoom level, smaller tiles at deep zoom for finer-grained progressive feedback

**Tech Stack:** Rust, existing MessageWorkerPool

---

## Overview

At extreme zoom levels (>1e10), the current 128px tiles are too large, making progressive rendering feel slow. We need smaller tiles (e.g., 64px) at deep zoom for more frequent visual feedback during long renders.

**Current state:**
- Tile size: Hardcoded 128px in `fractalwonder-ui/src/app.rs:61`
- Same tile size regardless of zoom level

**Target state:**
- Tile size: Dynamic based on zoom level
- 128px for normal zoom (< 1e10)
- 64px for deep zoom (>= 1e10)

---

## Task 1: Add calculate_tile_size Function

**Files:**
- Create: `fractalwonder-ui/src/rendering/tile_size.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Write failing test for tile size calculation**

Create `fractalwonder-ui/src/rendering/tile_size.rs`:

```rust
/// Calculate appropriate tile size based on zoom level
///
/// At extreme zoom levels, we use smaller tiles for more frequent
/// progressive rendering updates during long renders.
pub fn calculate_tile_size(zoom: f64) -> u32 {
    const DEEP_ZOOM_THRESHOLD: f64 = 1e10;
    const NORMAL_TILE_SIZE: u32 = 128;
    const DEEP_ZOOM_TILE_SIZE: u32 = 64;

    if zoom >= DEEP_ZOOM_THRESHOLD {
        DEEP_ZOOM_TILE_SIZE
    } else {
        NORMAL_TILE_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_zoom_uses_128px_tiles() {
        assert_eq!(calculate_tile_size(1.0), 128);
        assert_eq!(calculate_tile_size(100.0), 128);
        assert_eq!(calculate_tile_size(1e9), 128);
        assert_eq!(calculate_tile_size(9.9e9), 128);
    }

    #[test]
    fn test_deep_zoom_uses_64px_tiles() {
        assert_eq!(calculate_tile_size(1e10), 64);
        assert_eq!(calculate_tile_size(1e11), 64);
        assert_eq!(calculate_tile_size(1e50), 64);
        assert_eq!(calculate_tile_size(1e100), 64);
    }

    #[test]
    fn test_threshold_boundary() {
        // Just below threshold
        assert_eq!(calculate_tile_size(1e10 - 1.0), 128);
        // At threshold
        assert_eq!(calculate_tile_size(1e10), 64);
        // Just above threshold
        assert_eq!(calculate_tile_size(1e10 + 1.0), 64);
    }
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-ui tile_size`
Expected: PASS (3 tests)

**Step 3: Add module to mod.rs**

Modify `fractalwonder-ui/src/rendering/mod.rs`:

```rust
// Add this line
pub mod tile_size;

// Re-export
pub use tile_size::calculate_tile_size;
```

**Step 4: Run full UI crate tests**

Run: `cargo test --package fractalwonder-ui`
Expected: PASS (all tests)

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/rendering/tile_size.rs fractalwonder-ui/src/rendering/mod.rs
git commit -m "$(cat <<'EOF'
feat: add dynamic tile size calculation based on zoom level

Calculate tile size dynamically:
- 128px for normal zoom (< 1e10)
- 64px for deep zoom (>= 1e10)

Smaller tiles at extreme zoom provide more frequent progressive
rendering updates during long renders (30+ minutes).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Use Dynamic Tile Size in MessageParallelRenderer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/message_parallel_renderer.rs`

**Step 1: Remove tile_size field from MessageParallelRenderer**

Modify `fractalwonder-ui/src/rendering/message_parallel_renderer.rs`:

Add import:
```rust
use super::calculate_tile_size;
```

Remove `tile_size` field from struct (around line 35):

OLD:
```rust
pub struct MessageParallelRenderer {
    worker_pool: Rc<RefCell<MessageWorkerPool>>,
    colorizer: Rc<RefCell<Colorizer<AppData>>>,
    tile_size: u32,
    canvas: Rc<RefCell<Option<HtmlCanvasElement>>>,
    cached_state: Arc<Mutex<CachedState>>,
    progress: RwSignal<crate::rendering::RenderProgress>,
}
```

NEW:
```rust
pub struct MessageParallelRenderer {
    worker_pool: Rc<RefCell<MessageWorkerPool>>,
    colorizer: Rc<RefCell<Colorizer<AppData>>>,
    canvas: Rc<RefCell<Option<HtmlCanvasElement>>>,
    cached_state: Arc<Mutex<CachedState>>,
    progress: RwSignal<crate::rendering::RenderProgress>,
}
```

**Step 2: Update constructor to remove tile_size parameter**

Update constructor signature (around line 42):

OLD:
```rust
pub fn new(colorizer: Colorizer<AppData>, tile_size: u32) -> Result<Self, JsValue> {
```

NEW:
```rust
pub fn new(colorizer: Colorizer<AppData>) -> Result<Self, JsValue> {
```

Update constructor body (around line 84):

OLD:
```rust
web_sys::console::log_1(&JsValue::from_str(&format!(
    "MessageParallelRenderer created with {} workers, tile_size={}",
    worker_pool.borrow().worker_count(),
    tile_size
)));

Ok(Self {
    worker_pool,
    colorizer,
    tile_size,
    canvas,
    cached_state,
    progress,
})
```

NEW:
```rust
web_sys::console::log_1(&JsValue::from_str(&format!(
    "MessageParallelRenderer created with {} workers",
    worker_pool.borrow().worker_count(),
)));

Ok(Self {
    worker_pool,
    colorizer,
    canvas,
    cached_state,
    progress,
})
```

**Step 3: Update Clone implementation**

Update Clone (around line 145):

OLD:
```rust
Self {
    worker_pool: Rc::clone(&self.worker_pool),
    colorizer: Rc::clone(&self.colorizer),
    tile_size: self.tile_size,
    canvas: Rc::clone(&self.canvas),
    cached_state: Arc::clone(&self.cached_state),
    progress: self.progress,
}
```

NEW:
```rust
Self {
    worker_pool: Rc::clone(&self.worker_pool),
    colorizer: Rc::clone(&self.colorizer),
    canvas: Rc::clone(&self.canvas),
    cached_state: Arc::clone(&self.cached_state),
    progress: self.progress,
}
```

**Step 4: Calculate tile size in render method**

Update render method (around line 175-220):

Find this code:
```rust
fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement) {
    let width = canvas.width();
    let height = canvas.height();

    *self.canvas.borrow_mut() = Some(canvas.clone());

    let mut cache = self.cached_state.lock().unwrap();
    let render_id = cache.render_id.fetch_add(1, Ordering::SeqCst) + 1;

    // Convert f64 viewport to BigFloat
    let viewport_bf = Viewport::new(
        Point::new(
            BigFloat::from(*viewport.center.x()),
            BigFloat::from(*viewport.center.y()),
        ),
        viewport.zoom,
    );
```

Add tile size calculation after viewport conversion:
```rust
fn render(&self, viewport: &Viewport<Self::Scalar>, canvas: &HtmlCanvasElement) {
    let width = canvas.width();
    let height = canvas.height();

    *self.canvas.borrow_mut() = Some(canvas.clone());

    let mut cache = self.cached_state.lock().unwrap();
    let render_id = cache.render_id.fetch_add(1, Ordering::SeqCst) + 1;

    // Convert f64 viewport to BigFloat
    let viewport_bf = Viewport::new(
        Point::new(
            BigFloat::from(*viewport.center.x()),
            BigFloat::from(*viewport.center.y()),
        ),
        viewport.zoom,
    );

    // Calculate tile size based on zoom level
    let tile_size = calculate_tile_size(viewport.zoom);

    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
        "Using tile_size={} for zoom={}",
        tile_size, viewport.zoom
    )));
```

Then update the call to start_render:

OLD:
```rust
self.worker_pool
    .borrow_mut()
    .start_render(viewport_bf, width, height, self.tile_size);
```

NEW:
```rust
self.worker_pool
    .borrow_mut()
    .start_render(viewport_bf, width, height, tile_size);
```

**Step 5: Run cargo check**

Run: `cargo check --package fractalwonder-ui`
Expected: Errors in app.rs about create_message_parallel_renderer

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/message_parallel_renderer.rs
git commit -m "$(cat <<'EOF'
feat: use dynamic tile size in MessageParallelRenderer

Calculate tile size per-render based on viewport zoom level.
Remove hardcoded tile_size field.

Tiles are now 64px at deep zoom (>= 1e10) and 128px otherwise.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Update App to Remove tile_size Parameter

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Update create_message_parallel_renderer**

Modify `fractalwonder-ui/src/app.rs`:

Update function signature (around line 58):

OLD:
```rust
fn create_message_parallel_renderer(
    colorizer: Colorizer<AppData>,
) -> Result<MessageParallelRenderer, JsValue> {
    MessageParallelRenderer::new(colorizer, 128)
}
```

NEW:
```rust
fn create_message_parallel_renderer(
    colorizer: Colorizer<AppData>,
) -> Result<MessageParallelRenderer, JsValue> {
    MessageParallelRenderer::new(colorizer)
}
```

**Step 2: Run cargo check**

Run: `cargo check --workspace`
Expected: SUCCESS (no errors)

**Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: PASS (all tests)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "$(cat <<'EOF'
feat: remove hardcoded tile size from renderer creation

Tile size is now calculated dynamically based on zoom level.
No longer need to pass tile_size parameter.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Manual Testing and Validation

**Files:**
- N/A (testing only)

**Step 1: Build and run**

Run: `trunk serve`
Navigate to: `http://localhost:8080`

**Step 2: Test normal zoom levels**

Test with zoom < 1e10:
- [ ] Render at default zoom (zoom ~1.0)
- [ ] Check console: Should see "Using tile_size=128"
- [ ] Observe progressive rendering
- [ ] Verify tiles appear in reasonable chunks

**Step 3: Test deep zoom levels**

Test with zoom >= 1e10:
- [ ] Zoom in to 1e10 or beyond
- [ ] Check console: Should see "Using tile_size=64"
- [ ] Observe progressive rendering
- [ ] Verify smaller tiles provide more frequent updates

**Step 4: Test boundary**

Test at zoom = 1e10 exactly:
- [ ] Set zoom to exactly 1e10
- [ ] Verify tile_size switches to 64
- [ ] Zoom slightly out (< 1e10)
- [ ] Verify tile_size switches back to 128

**Step 5: Visual verification**

Compare UX:
- [ ] Normal zoom (128px tiles): Larger chunks, feels faster
- [ ] Deep zoom (64px tiles): More frequent updates, better feedback during long renders

**Step 6: Performance check**

Monitor:
- [ ] No performance regression
- [ ] Worker utilization still good
- [ ] No console errors
- [ ] Memory usage stable

**Step 7: Run full test suite**

Run: `cargo test --workspace --all-targets --all-features`
Expected: PASS (all tests)

**Step 8: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 9: Run formatter check**

Run: `cargo fmt --all -- --check`
Expected: All files formatted

**Step 10: Document any issues**

If issues found, document them for follow-up.

---

## Task 5: Optional - Fine-tune Threshold and Sizes

**Files:**
- Modify: `fractalwonder-ui/src/rendering/tile_size.rs` (if needed)

**Step 1: Evaluate threshold**

Based on manual testing:
- Is 1e10 the right threshold?
- Should we have multiple tiers (e.g., 128px → 96px → 64px)?
- Should extreme zoom (>1e50) use even smaller tiles (32px)?

**Step 2: Adjust if needed**

If current threshold doesn't feel right, update the constants:

```rust
pub fn calculate_tile_size(zoom: f64) -> u32 {
    // Example: multiple tiers
    const EXTREME_ZOOM_THRESHOLD: f64 = 1e50;
    const DEEP_ZOOM_THRESHOLD: f64 = 1e10;

    if zoom >= EXTREME_ZOOM_THRESHOLD {
        32  // Very small tiles for extreme zoom
    } else if zoom >= DEEP_ZOOM_THRESHOLD {
        64  // Small tiles for deep zoom
    } else {
        128  // Normal tiles
    }
}
```

**Step 3: Update tests**

Update tests to match new logic.

**Step 4: Re-test**

Test the new thresholds manually.

**Step 5: Commit if changed**

```bash
git add fractalwonder-ui/src/rendering/tile_size.rs
git commit -m "$(cat <<'EOF'
tune: adjust tile size thresholds

[Describe what you changed and why]

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Success Criteria

**Dynamic tile sizing is complete when:**

1. ✅ `calculate_tile_size` function implemented with tests
2. ✅ MessageParallelRenderer uses dynamic tile size
3. ✅ App no longer passes hardcoded tile size
4. ✅ Manual testing shows:
   - 128px tiles at normal zoom
   - 64px tiles at deep zoom (>= 1e10)
   - Smooth transition at boundary
5. ✅ All tests pass
6. ✅ No Clippy warnings
7. ✅ Code formatted correctly
8. ✅ Console logs show tile size changing with zoom

**UX improvement:**
- More frequent progressive updates during long renders at extreme zoom
- Better visual feedback that rendering is progressing
- No performance degradation

---

## Notes

**Why this works:**
- Deep zoom renders take longer (more iterations)
- Smaller tiles complete faster
- More frequent completions = better progressive UX
- Trade-off: More tiles = slightly more overhead, but worth it for UX

**Why not make tiles even smaller?**
- 64px is a good balance
- Too small (e.g., 16px) = too much overhead
- Too large (e.g., 256px) = infrequent updates

**Future optimization:**
- Could make tile size adaptive based on render complexity
- Could consider canvas size when calculating tile size
- Could profile actual render times to tune thresholds
- But start simple!
