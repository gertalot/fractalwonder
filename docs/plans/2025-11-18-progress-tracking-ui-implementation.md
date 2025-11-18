# Progress Tracking UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire up live progress tracking to display "render: X/Y" during rendering and "render: N.NNs" when complete.

**Architecture:** Add RenderProgress signal to MessageParallelRenderer, pass to MessageWorkerPool, update signal on tile completions, connect to UI InfoDisplay component.

**Tech Stack:** Rust, Leptos (reactive signals), WebAssembly, web_sys (performance.now())

---

## Task 1: Add RenderProgress struct to rendering module

**Files:**
- Modify: `fractalwonder-ui/src/rendering/mod.rs:1-26`

**Step 1: Add RenderProgress struct**

After line 16 (after `pub use tiling_canvas_renderer::TilingCanvasRenderer;`), add:

```rust
/// Progress information for ongoing renders
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderProgress {
    pub completed_tiles: u32,
    pub total_tiles: u32,
    pub render_id: u32,
    pub elapsed_ms: f64,
    pub is_complete: bool,
}

impl RenderProgress {
    pub fn new(total_tiles: u32, render_id: u32) -> Self {
        Self {
            completed_tiles: 0,
            total_tiles,
            render_id,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }

    pub fn percentage(&self) -> f32 {
        if self.total_tiles == 0 {
            0.0
        } else {
            (self.completed_tiles as f32 / self.total_tiles as f32) * 100.0
        }
    }
}

impl Default for RenderProgress {
    fn default() -> Self {
        Self {
            completed_tiles: 0,
            total_tiles: 0,
            render_id: 0,
            elapsed_ms: 0.0,
            is_complete: false,
        }
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check --workspace`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/mod.rs
git commit -m "feat: add RenderProgress struct to rendering module"
```

---

## Task 2: Add progress signal to MessageParallelRenderer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/message_parallel_renderer.rs:31-92`

**Step 1: Import RwSignal and add progress field**

At top of file, ensure leptos is imported:
```rust
use leptos::*;
```

In `MessageParallelRenderer` struct (line 31), add field:
```rust
pub struct MessageParallelRenderer {
    worker_pool: Rc<RefCell<MessageWorkerPool>>,
    colorizer: Rc<RefCell<Colorizer<AppData>>>,
    tile_size: u32,
    canvas: Rc<RefCell<Option<HtmlCanvasElement>>>,
    cached_state: Arc<Mutex<CachedState>>,
    progress: RwSignal<crate::rendering::RenderProgress>,  // NEW
}
```

**Step 2: Initialize progress signal in constructor**

In `MessageParallelRenderer::new()` function (around line 40), before creating worker_pool:

```rust
pub fn new(colorizer: Colorizer<AppData>, tile_size: u32) -> Result<Self, JsValue> {
    let canvas: Rc<RefCell<Option<HtmlCanvasElement>>> = Rc::new(RefCell::new(None));
    let canvas_clone = Rc::clone(&canvas);
    let colorizer = Rc::new(RefCell::new(colorizer));
    let colorizer_clone = Rc::clone(&colorizer);
    let cached_state = Arc::new(Mutex::new(CachedState::default()));
    let cached_state_clone = Arc::clone(&cached_state);

    // NEW: Create progress signal
    let progress = create_rw_signal(crate::rendering::RenderProgress::default());

    let on_tile_complete = move |tile_result: TileResult| {
        // ... existing code ...
    };

    let worker_pool = MessageWorkerPool::new(on_tile_complete)?;

    // ... rest of constructor ...

    Ok(Self {
        worker_pool,
        colorizer,
        tile_size,
        canvas,
        cached_state,
        progress,  // NEW
    })
}
```

**Step 3: Add progress() accessor method**

After the `new()` method, add:

```rust
pub fn progress(&self) -> RwSignal<crate::rendering::RenderProgress> {
    self.progress
}
```

**Step 4: Update Clone impl**

In `Clone` implementation (around line 135), add progress field:

```rust
impl Clone for MessageParallelRenderer {
    fn clone(&self) -> Self {
        Self {
            worker_pool: Rc::clone(&self.worker_pool),
            colorizer: Rc::clone(&self.colorizer),
            tile_size: self.tile_size,
            canvas: Rc::clone(&self.canvas),
            cached_state: Arc::clone(&self.cached_state),
            progress: self.progress,  // NEW: RwSignal is Copy
        }
    }
}
```

**Step 5: Verify it compiles**

Run: `cargo check --workspace`
Expected: No errors

**Step 6: Commit**

```bash
git add fractalwonder-ui/src/rendering/message_parallel_renderer.rs
git commit -m "feat: add progress signal to MessageParallelRenderer"
```

---

## Task 3: Pass progress signal to MessageWorkerPool

**Files:**
- Modify: `fractalwonder-ui/src/workers/message_worker_pool.rs:20-108`
- Modify: `fractalwonder-ui/src/rendering/message_parallel_renderer.rs:77`

**Step 1: Add progress_signal field to MessageWorkerPool**

In `MessageWorkerPool` struct (line 20), add fields:

```rust
pub struct MessageWorkerPool {
    workers: Vec<Worker>,
    pending_tiles: Rc<RefCell<VecDeque<TileRequest>>>,
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    render_id: Arc<AtomicU32>,
    progress_signal: RwSignal<crate::rendering::RenderProgress>,  // NEW
    render_start_time: Rc<RefCell<Option<f64>>>,  // NEW
}
```

**Step 2: Update MessageWorkerPool::new() signature**

Change constructor to accept progress signal (around line 31):

```rust
pub fn new(
    on_tile_complete: impl Fn(TileResult) + 'static,
    progress_signal: RwSignal<crate::rendering::RenderProgress>,  // NEW
) -> Result<Self, JsValue> {
    let worker_count = web_sys::window()
        .and_then(|w| w.navigator().hardware_concurrency())
        .map(|c| c as usize)
        .unwrap_or(4);

    // ... existing worker creation code ...

    Ok(Self {
        workers,
        pending_tiles: Rc::new(RefCell::new(VecDeque::new())),
        on_tile_complete: Rc::new(on_tile_complete),
        render_id: Arc::new(AtomicU32::new(0)),
        progress_signal,  // NEW
        render_start_time: Rc::new(RefCell::new(None)),  // NEW
    })
}
```

**Step 3: Update MessageParallelRenderer to pass progress signal**

In `message_parallel_renderer.rs`, update the `MessageWorkerPool::new()` call (around line 77):

```rust
let worker_pool = MessageWorkerPool::new(on_tile_complete, progress)?;
```

**Step 4: Verify it compiles**

Run: `cargo check --workspace`
Expected: No errors

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/workers/message_worker_pool.rs fractalwonder-ui/src/rendering/message_parallel_renderer.rs
git commit -m "feat: pass progress signal to MessageWorkerPool"
```

---

## Task 4: Update progress on render start

**Files:**
- Modify: `fractalwonder-ui/src/workers/message_worker_pool.rs:192-251`

**Step 1: Record start time and initialize progress in start_render()**

In `start_render()` method (around line 222), after incrementing render_id and before distributing tiles:

```rust
pub fn start_render(
    &mut self,
    viewport: Viewport<BigFloat>,
    canvas_width: u32,
    canvas_height: u32,
    tile_size: u32,
) {
    let current_render_id = self.render_id.fetch_add(1, Ordering::SeqCst) + 1;

    web_sys::console::log_1(&JsValue::from_str(&format!(
        "MessageWorkerPool::start_render (render_id: {})",
        current_render_id
    )));

    // Generate tiles
    let tiles = Self::generate_tiles(canvas_width, canvas_height, tile_size);
    let total_tiles = tiles.len() as u32;

    // NEW: Record start time
    let start_time = web_sys::window()
        .unwrap()
        .performance()
        .unwrap()
        .now();
    *self.render_start_time.borrow_mut() = Some(start_time);

    // NEW: Initialize progress signal
    self.progress_signal.set(crate::rendering::RenderProgress::new(
        total_tiles,
        current_render_id,
    ));

    // ... rest of existing code (populate pending_tiles, distribute work) ...
}
```

**Step 2: Verify it compiles**

Run: `cargo check --workspace`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/workers/message_worker_pool.rs
git commit -m "feat: initialize progress on render start"
```

---

## Task 5: Update progress on tile completion

**Files:**
- Modify: `fractalwonder-ui/src/workers/message_worker_pool.rs:114-190`

**Step 1: Update progress in handle_worker_message()**

In `handle_worker_message()` method, in the `TileComplete` branch (around line 127):

```rust
WorkerToMain::TileComplete {
    render_id,
    tile,
    data,
    compute_time_ms,
} => {
    if render_id != self.render_id.load(Ordering::SeqCst) {
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Ignoring stale tile completion (render_id: {} vs current: {})",
            render_id,
            self.render_id.load(Ordering::SeqCst)
        )));
        return;
    }

    // NEW: Calculate elapsed time and update progress
    let elapsed_ms = if let Some(start) = *self.render_start_time.borrow() {
        web_sys::window().unwrap().performance().unwrap().now() - start
    } else {
        0.0
    };

    self.progress_signal.update(|p| {
        if p.render_id == render_id {
            p.completed_tiles += 1;
            p.elapsed_ms = elapsed_ms;
            p.is_complete = p.completed_tiles >= p.total_tiles;
        }
    });

    // ... rest of existing code (call on_tile_complete callback, send_work_to_worker) ...
}
```

**Step 2: Verify it compiles**

Run: `cargo check --workspace`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/workers/message_worker_pool.rs
git commit -m "feat: update progress on tile completion"
```

---

## Task 6: Update InfoDisplay component to show progress

**Files:**
- Modify: `fractalwonder-ui/src/components/ui.rs:118-144`

**Step 1: Add optional progress prop to InfoDisplay**

Update `InfoDisplay` component signature and display logic (around line 118):

```rust
#[component]
fn InfoDisplay(
    info: ReadSignal<RendererInfoData>,
    #[prop(optional)] progress: Option<Signal<crate::rendering::RenderProgress>>,
) -> impl IntoView {
    view! {
      <div class="text-white text-sm">
        <p>
          {move || {
            let i = info.get();
            format!("Center: {}, zoom: {}", i.center_display, i.zoom_display)
          }}
          {move || {
            if let Some(prog_signal) = progress {
                let prog = prog_signal.get();
                if prog.is_complete && prog.total_tiles > 0 {
                    // Render complete: show total time
                    format!(", render: {:.2}s", prog.elapsed_ms / 1000.0)
                } else if prog.total_tiles > 0 {
                    // Render in progress: show tiles completed/total
                    format!(", render: {}/{}", prog.completed_tiles, prog.total_tiles)
                } else {
                    String::new()
                }
            } else {
                // Fallback to old behavior if no progress signal
                info.get().render_time_ms.map(|ms|
                    format!(", render: {:.2}s", ms / 1000.0)
                ).unwrap_or_default()
            }
          }}
        </p>
        <p>
          {move || info.get().name}
          {move || {
            info.get().custom_params.iter()
              .map(|(k, v)| format!(" | {}: {}", k, v))
              .collect::<Vec<_>>()
              .join("")
          }}
        </p>
      </div>
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check --workspace`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/ui.rs
git commit -m "feat: update InfoDisplay to show progress"
```

---

## Task 7: Pass progress to UI component

**Files:**
- Modify: `fractalwonder-ui/src/components/ui.rs:146-217`

**Step 1: Add optional progress prop to UI component**

Update `UI` component signature (around line 146):

```rust
#[component]
pub fn UI(
    info: ReadSignal<RendererInfoData>,
    is_visible: ReadSignal<bool>,
    set_is_hovering: WriteSignal<bool>,
    on_home_click: impl Fn() + 'static,
    on_fullscreen_click: impl Fn() + 'static,
    render_function_options: Signal<Vec<(String, String)>>,
    selected_renderer_id: Signal<String>,
    on_renderer_select: impl Fn(String) + 'static + Copy,
    color_scheme_options: Signal<Vec<(String, String)>>,
    selected_color_scheme_id: Signal<String>,
    on_color_scheme_select: impl Fn(String) + 'static + Copy,
    #[prop(optional)] progress: Option<Signal<crate::rendering::RenderProgress>>,  // NEW
) -> impl IntoView {
```

**Step 2: Pass progress to InfoDisplay**

Update the InfoDisplay call (around line 205):

```rust
// Center section: info display
<div class="flex-1 text-center">
  {
    move || match progress {
      Some(p) => view! { <InfoDisplay info=info progress=p /> }.into_view(),
      None => view! { <InfoDisplay info=info /> }.into_view(),
    }
  }
</div>
```

**Step 3: Verify it compiles**

Run: `cargo check --workspace`
Expected: No errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/ui.rs
git commit -m "feat: pass progress to UI component"
```

---

## Task 8: Wire progress from App to UI

**Files:**
- Modify: `fractalwonder-ui/src/app.rs:20-40`
- Modify: `fractalwonder-ui/src/app.rs:269-281`

**Step 1: Add progress() method to CanvasRendererHolder**

In `CanvasRendererHolder` impl (around line 20), add method:

```rust
impl CanvasRendererHolder {
    fn render(&self, viewport: &Viewport<f64>, canvas: &HtmlCanvasElement) {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.render(viewport, canvas)
    }

    fn natural_bounds(&self) -> crate::rendering::Rect<f64> {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.natural_bounds()
    }

    fn set_colorizer(&mut self, colorizer: Colorizer<AppData>) {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.set_colorizer(colorizer)
    }

    fn cancel_render(&self) {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.cancel_render()
    }

    // NEW
    fn progress(&self) -> RwSignal<crate::rendering::RenderProgress> {
        let CanvasRendererHolder::MessageParallel(r) = self;
        r.progress()
    }
}
```

**Step 2: Create progress memo in App component**

After `natural_bounds` memo (around line 94), add:

```rust
// ========== Natural bounds - reactive to renderer changes ==========
let natural_bounds = create_memo(move |_| canvas_renderer.with(|cr| cr.natural_bounds()));

// NEW: Progress tracking
let progress = create_memo(move |_| {
    canvas_renderer.with(|cr| cr.progress().get())
});
```

**Step 3: Pass progress to UI component**

Update UI call (around line 269):

```rust
<UI
    info=renderer_info
    is_visible=ui_visibility.is_visible
    set_is_hovering=ui_visibility.set_is_hovering
    on_home_click=on_home_click
    on_fullscreen_click=on_fullscreen_click
    render_function_options=render_function_options.into()
    selected_renderer_id=Signal::derive(move || selected_renderer_id.get())
    on_renderer_select=move |id: String| set_selected_renderer_id.set(id)
    color_scheme_options=color_scheme_options.into()
    selected_color_scheme_id=Signal::derive(move || selected_color_scheme_id.get())
    on_color_scheme_select=on_color_scheme_select
    progress=Signal::derive(move || progress.get())  // NEW
/>
```

**Step 4: Verify it compiles**

Run: `cargo check --workspace`
Expected: No errors

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat: wire progress from App to UI"
```

---

## Task 9: Run all quality checks

**Step 1: Format code**

Run: `cargo fmt --all`
Expected: No output (code already formatted)

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 3: Run tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 4: Final commit if needed**

```bash
git add -A
git commit -m "chore: format and lint"
```

---

## Task 10: Manual browser testing

**Step 1: Start dev server**

Ensure trunk is running: `trunk serve` (should already be running per requirements)

**Step 2: Open browser**

Navigate to: http://localhost:8080

**Step 3: Verify progress display**

**Expected behavior:**
1. On initial load: See "render: 0/0" or blank (no tiles yet)
2. While rendering: See "render: X/Y" counting up (e.g., "render: 45/230")
3. When complete: See "render: N.NNs" (e.g., "render: 2.34s")

**Test actions:**
- Pan around: Should see progress reset and count up
- Change color scheme: Should show quick render time (cache hit)
- Zoom in/out: Should see progress counting tiles

**Step 4: Document any issues**

If behavior doesn't match expected, note the issue for debugging.

---

## Verification Checklist

- [ ] RenderProgress struct compiles
- [ ] MessageParallelRenderer has progress field
- [ ] MessageWorkerPool accepts and updates progress signal
- [ ] Progress initializes on start_render()
- [ ] Progress updates on each tile completion
- [ ] InfoDisplay shows "render: X/Y" when rendering
- [ ] InfoDisplay shows "render: N.NNs" when complete
- [ ] UI component passes progress through
- [ ] App wires progress from renderer to UI
- [ ] All tests pass
- [ ] Clippy clean
- [ ] Browser shows live progress updates

---

## Success Criteria

**The implementation is successful when:**

1. Starting a render shows "render: 0/N" immediately
2. During render, see live updates "render: 1/N", "render: 2/N", etc.
3. On completion, shows "render: X.XXs" with the total elapsed time
4. Changing color scheme (cache hit) shows very fast time like "render: 0.02s"
5. All automated tests pass
6. No Clippy warnings
