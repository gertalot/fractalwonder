# Responsive Cancellation Implementation

**Date:** 2025-11-18
**Status:** Design Complete
**Context:** Iteration 4 - Replace cooperative cancellation with forceful worker termination

---

## Problem Statement

At extreme zoom levels, individual tiles can take minutes to compute. Current cooperative cancellation (via `render_id` checking) means workers cannot respond to cancellation until they finish their current tile. This causes unpredictable lag ranging from seconds to minutes when users interact during long renders.

**Goal:** Achieve predictable <500ms cancellation latency regardless of tile complexity.

---

## Solution: Terminate and Recreate Workers

When user interacts (pan/zoom):
1. Immediately terminate all workers with `worker.terminate()`
2. Recreate worker pool
3. Start new render

**Key insight:** Predictable bounded latency (<500ms) is better than unpredictable unbounded latency (seconds to minutes).

---

## Implementation Details

### File: `fractalwonder-ui/src/workers/message_worker_pool.rs`

### Change 1: Add Weak Self-Reference Field

Add one field to `MessageWorkerPool` struct (line ~21):

```rust
pub struct MessageWorkerPool {
    workers: Vec<Worker>,
    // ... existing fields
    self_ref: Weak<RefCell<Self>>,
}
```

**Why:** Worker creation requires `Rc<RefCell<MessageWorkerPool>>` for callback closures. From inside `cancel_current_render(&mut self)`, we only have access to the inner struct, not the outer `Rc` wrapper. Storing a `Weak` reference allows us to access the wrapper when recreating workers.

### Change 2: Initialize Self-Reference in Constructor

In `MessageWorkerPool::new()` (after line 56):

```rust
let pool = Rc::new(RefCell::new(Self {
    workers: Vec::new(),
    // ... other fields
    self_ref: Weak::new(),
}));

pool.borrow_mut().self_ref = Rc::downgrade(&pool);
```

### Change 3: Extract Worker Creation Function

Extract lines 72-113 into a standalone function:

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

        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Some(msg_str) = e.data().as_string() {
                if let Ok(msg) = serde_json::from_str::<WorkerToMain>(&msg_str) {
                    pool_clone.borrow_mut().handle_worker_message(worker_id, msg);
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

### Change 4: Update Constructor to Use create_workers

Replace lines 72-115 in `new()`:

```rust
let worker_count = web_sys::window()
    .map(|w| w.navigator().hardware_concurrency() as usize)
    .unwrap_or(4);

let workers = create_workers(worker_count, Rc::clone(&pool))?;
pool.borrow_mut().workers = workers;
```

### Change 5: Rewrite cancel_current_render

Replace existing `cancel_current_render()` (lines 288-296):

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

---

## Testing Strategy

### Manual Validation

1. Start a render at extreme zoom (tiles take 10+ seconds each)
2. Pan or zoom during render
3. Observe in browser console:
   - "Terminated all workers for cancellation"
   - "Recreated N workers"
4. Verify new render starts quickly (feels <500ms)
5. Monitor CPU usage - should drop immediately on cancel

### Expected Behavior

- Old render stops instantly
- New render starts within ~500ms
- UI stays responsive throughout

### Failure Modes to Watch For

- Workers don't terminate (CPU stays high)
- Worker recreation fails (error in console, no workers)
- Multiple rapid cancellations cause issues

---

## Summary of Changes

**Files modified:** 1 file (`fractalwonder-ui/src/workers/message_worker_pool.rs`)

**Changes:**
1. Add `self_ref: Weak<RefCell<Self>>` field
2. Initialize self-reference in constructor
3. Extract `create_workers()` function
4. Update `new()` to use `create_workers()`
5. Rewrite `cancel_current_render()` to terminate and recreate workers

**Complexity:** Minimal - one new field, one extracted function, one updated method.

---

## Success Criteria

- User interaction during long render → new render starts within 500ms
- CPU usage drops immediately when workers terminated
- No worker creation failures
- UI remains responsive throughout
