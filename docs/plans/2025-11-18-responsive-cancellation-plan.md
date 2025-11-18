# Responsive Cancellation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace cooperative cancellation with forceful worker termination for predictable <500ms cancellation latency.

**Architecture:** Add Weak self-reference to MessageWorkerPool, extract worker creation into reusable function, update cancel_current_render to terminate and recreate workers.

**Tech Stack:** Rust, wasm-bindgen, web-sys, Web Workers API

---

## Task 1: Add Weak Self-Reference Field

**Files:**
- Modify: `fractalwonder-ui/src/workers/message_worker_pool.rs:21-33`

**Step 1: Add self_ref field to MessageWorkerPool struct**

In `message_worker_pool.rs`, add the `Weak` import and field:

```rust
use std::rc::Weak;  // Add to imports at top

pub struct MessageWorkerPool {
    workers: Vec<Worker>,
    pending_tiles: VecDeque<TileRequest>,
    failed_tiles: HashMap<(u32, u32), u32>,
    current_render_id: u32,
    current_viewport: Viewport<BigFloat>,
    canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    progress_signal: RwSignal<crate::rendering::RenderProgress>,
    render_start_time: Rc<RefCell<Option<f64>>>,
    self_ref: Weak<RefCell<Self>>,  // NEW FIELD
}
```

**Step 2: Initialize self_ref in constructor**

In `MessageWorkerPool::new()`, around line 56, initialize the field:

```rust
let pool = Rc::new(RefCell::new(Self {
    workers: Vec::new(),
    pending_tiles: VecDeque::new(),
    failed_tiles: HashMap::new(),
    current_render_id: 0,
    current_viewport: Viewport::new(
        fractalwonder_core::Point::new(BigFloat::from(0.0), BigFloat::from(0.0)),
        1.0,
    ),
    canvas_size: (0, 0),
    on_tile_complete,
    progress_signal,
    render_start_time: Rc::new(RefCell::new(None)),
    self_ref: Weak::new(),  // NEW: Initialize empty
}));

// NEW: Store weak reference to self
pool.borrow_mut().self_ref = Rc::downgrade(&pool);
```

**Step 3: Build to verify compilation**

Run: `cargo check --workspace`
Expected: SUCCESS (no compilation errors)

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/workers/message_worker_pool.rs
git commit -m "feat: add Weak self-reference to MessageWorkerPool

Stores Weak<RefCell<Self>> to enable worker recreation from
cancel_current_render. Required because worker closures need
Rc<RefCell<>> but method only has &mut self access.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: Extract create_workers Function

**Files:**
- Modify: `fractalwonder-ui/src/workers/message_worker_pool.rs:71-118`

**Step 1: Extract create_workers as standalone function**

Add this function before the `impl MessageWorkerPool` block (or as a private method inside impl):

```rust
fn create_workers(
    worker_count: usize,
    pool: Rc<RefCell<MessageWorkerPool>>,
) -> Result<Vec<Worker>, JsValue> {
    let mut workers = Vec::new();

    for i in 0..worker_count {
        let worker = Worker::new("./message-compute-worker.js")?;

        let worker_id = i;
        let pool_clone = Rc::clone(&pool);

        // Message handler
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Some(msg_str) = e.data().as_string() {
                if let Ok(msg) = serde_json::from_str::<WorkerToMain>(&msg_str) {
                    pool_clone
                        .borrow_mut()
                        .handle_worker_message(worker_id, msg);
                } else {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "Worker {} sent invalid message: {}",
                        worker_id, msg_str
                    )));
                }
            }
        }) as Box<dyn FnMut(_)>);

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // Error handler
        let error_handler = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
            web_sys::console::error_1(&JsValue::from_str(&format!(
                "Worker {} error: {}",
                worker_id,
                e.message()
            )));
        }) as Box<dyn FnMut(_)>);

        worker.set_onerror(Some(error_handler.as_ref().unchecked_ref()));
        error_handler.forget();

        workers.push(worker);

        web_sys::console::log_1(&JsValue::from_str(&format!("Worker {} created", i)));
    }

    Ok(workers)
}
```

**Step 2: Update MessageWorkerPool::new() to use create_workers**

Replace the worker creation loop (lines ~73-115) with:

```rust
// Get hardware concurrency
let worker_count = web_sys::window()
    .map(|w| w.navigator().hardware_concurrency() as usize)
    .unwrap_or(4);

web_sys::console::log_1(&JsValue::from_str(&format!(
    "Creating MessageWorkerPool with {} workers",
    worker_count
)));

let on_tile_complete = Rc::new(on_tile_complete);

// Create pool structure
let pool = Rc::new(RefCell::new(Self {
    workers: Vec::new(),
    pending_tiles: VecDeque::new(),
    failed_tiles: HashMap::new(),
    current_render_id: 0,
    current_viewport: Viewport::new(
        fractalwonder_core::Point::new(BigFloat::from(0.0), BigFloat::from(0.0)),
        1.0,
    ),
    canvas_size: (0, 0),
    on_tile_complete,
    progress_signal,
    render_start_time: Rc::new(RefCell::new(None)),
    self_ref: Weak::new(),
}));

// Store weak reference to self
pool.borrow_mut().self_ref = Rc::downgrade(&pool);

// Create workers using extracted function
let workers = create_workers(worker_count, Rc::clone(&pool))?;
pool.borrow_mut().workers = workers;

Ok(pool)
```

**Step 3: Build to verify compilation**

Run: `cargo check --workspace`
Expected: SUCCESS

**Step 4: Test in browser**

Run: `trunk serve` (if not already running)
Navigate to: `http://localhost:8080`
Expected: Application loads, workers are created (check console logs)

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/workers/message_worker_pool.rs
git commit -m "refactor: extract create_workers function

Extracts worker creation logic into reusable function to enable
worker recreation during cancellation. No behavior change.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: Implement Worker Termination in cancel_current_render

**Files:**
- Modify: `fractalwonder-ui/src/workers/message_worker_pool.rs:288-296`

**Step 1: Rewrite cancel_current_render to terminate and recreate workers**

Replace the existing `cancel_current_render` method (lines 288-296):

```rust
pub fn cancel_current_render(&mut self) {
    // 1. Terminate all workers immediately
    for worker in &self.workers {
        worker.terminate();
    }

    web_sys::console::log_1(&JsValue::from_str(
        "Terminated all workers for cancellation"
    ));

    // 2. Recreate workers using stored self-reference
    if let Some(pool_rc) = self.self_ref.upgrade() {
        match create_workers(self.workers.len(), pool_rc) {
            Ok(new_workers) => {
                self.workers = new_workers;
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Recreated {} workers",
                    self.workers.len()
                )));
            }
            Err(e) => {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Failed to recreate workers: {:?}",
                    e
                )));
                // Keep empty workers vec - pool is broken
                self.workers.clear();
            }
        }
    }

    // 3. Increment render ID and clear pending work
    self.current_render_id += 1;
    self.pending_tiles.clear();

    web_sys::console::log_1(&JsValue::from_str(&format!(
        "Cancelled render, new render_id: {}",
        self.current_render_id
    )));
}
```

**Step 2: Build to verify compilation**

Run: `cargo check --workspace`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/workers/message_worker_pool.rs
git commit -m "feat: implement forceful worker termination on cancel

Replace cooperative cancellation with worker.terminate() and
recreation. Provides predictable <500ms cancellation latency
regardless of tile computation time.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: Manual Testing and Validation

**Files:**
- None (manual browser testing)

**Step 1: Build and run application**

Run: `trunk serve`
Navigate to: `http://localhost:8080`
Open browser console (F12)

**Step 2: Test basic cancellation**

1. Start a render at normal zoom
2. Pan or zoom immediately
3. Check console for:
   - "Terminated all workers for cancellation"
   - "Recreated N workers"
   - New render starts

Expected: Logs appear, new render starts smoothly

**Step 3: Test extreme zoom cancellation**

1. Zoom in to extreme level (zoom > 1e10) where tiles take 10+ seconds
2. Wait for tiles to start rendering
3. Pan or zoom to cancel
4. Observe:
   - Console shows termination/recreation logs
   - CPU usage drops immediately (check Activity Monitor/Task Manager)
   - New render starts within ~500ms

Expected: Instant cancellation, fast restart

**Step 4: Test rapid cancellation**

1. Pan/zoom rapidly multiple times in succession
2. Check console for multiple termination/recreation cycles
3. Verify no errors or worker creation failures

Expected: Handles rapid cancellations gracefully, no errors

**Step 5: Document results**

Create: `docs/testing/2025-11-18-responsive-cancellation-validation.md`

```markdown
# Responsive Cancellation Validation

**Date:** 2025-11-18
**Tester:** [Your name]

## Test Results

### Basic Cancellation
- ✅ Workers terminate on cancel
- ✅ Workers recreate successfully
- ✅ New render starts immediately

### Extreme Zoom Cancellation
- ✅ Cancellation feels instant (<500ms)
- ✅ CPU usage drops immediately
- ✅ No lag waiting for tile completion

### Rapid Cancellation
- ✅ Multiple rapid cancels handled
- ✅ No worker creation failures
- ✅ No console errors

## Performance Notes
- Worker recreation time: ~[measured time]ms
- Cancellation latency: ~[measured time]ms
- Number of workers: [N]

## Issues Found
[List any issues or unexpected behavior]
```

**Step 6: Commit test results**

```bash
git add docs/testing/2025-11-18-responsive-cancellation-validation.md
git commit -m "docs: add responsive cancellation validation results

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: Final Quality Checks

**Files:**
- All workspace files

**Step 1: Run formatter**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 2: Run linter**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 3: Run workspace checks**

Run: `cargo check --workspace --all-targets --all-features`
Expected: SUCCESS

**Step 4: Run tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 5: Build release**

Run: `trunk build --release`
Expected: Successful build in `dist/`

**Step 6: Final commit (if any fixes needed)**

```bash
git add .
git commit -m "chore: apply formatting and fix clippy warnings

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Success Criteria

- ✅ All code compiles without warnings
- ✅ All tests pass
- ✅ Application runs in browser
- ✅ Cancellation terminates workers immediately
- ✅ Workers recreate successfully (<500ms)
- ✅ UI remains responsive during cancellation
- ✅ No console errors or worker creation failures
- ✅ CPU usage drops immediately on cancel

---

## Related Documentation

- Design: `docs/plans/2025-11-18-responsive-cancellation-implementation.md`
- Architecture: `docs/multicore-plans/2025-11-17-progressive-parallel-rendering-design.md` (Iteration 4)

---

## Notes for Implementation

- **Careful with borrow checking**: Drop borrows before calling `create_workers()` to avoid double-borrow panics
- **Error handling**: If worker recreation fails, pool becomes non-functional with empty workers vec
- **Testing**: Manual browser testing is primary validation method (no automated tests for worker termination)
- **Performance**: Actual recreation time may differ from estimate - validate during testing
