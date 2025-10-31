# UI Buttons and Renderer Info Display Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Implement a complete UI overlay system with Info, Home, and Fullscreen buttons, plus a generic renderer info display system that works across different renderer types (test images, fractals, maps).

**Architecture:** Create a `RendererInfo` trait that renderers implement to expose displayable metadata. `InteractiveCanvas` tracks render performance and exposes viewport reset. UI component displays unified info and provides three functional buttons. All communication via explicit signals and callbacks (no context magic).

**Tech Stack:** Leptos 0.6+, Rust/WASM, web_sys for fullscreen API, inline SVG icons

---

## Task 1: Create RendererInfo Trait and Data Structures

**Files:**
- Create: `src/rendering/renderer_info.rs`
- Modify: `src/rendering/mod.rs` (add module export)

**Step 1: Create renderer_info.rs with trait definition**

Create `src/rendering/renderer_info.rs`:

```rust
use crate::rendering::viewport::Viewport;

/// Optional trait for renderers to expose displayable information to the UI.
/// Combines viewport state with renderer-specific parameters.
pub trait RendererInfo {
    type Coord;

    /// Returns current display information including viewport and custom parameters.
    /// Performance metrics (render_time_ms) are filled by InteractiveCanvas.
    fn info(&self, viewport: &Viewport<Self::Coord>) -> RendererInfoData;
}

/// Display information for UI overlay
#[derive(Clone, Debug)]
pub struct RendererInfoData {
    /// Renderer name (e.g., "Test Image", "Mandelbrot Fractal", "Map View")
    pub name: String,

    /// Viewport center point, formatted for display by renderer
    pub center_display: String,

    /// Zoom level, formatted for display by renderer
    pub zoom_display: String,

    /// Custom renderer parameters (e.g., "Iterations: 1000", "Color: rainbow")
    /// Each tuple is (parameter_name, display_value)
    pub custom_params: Vec<(String, String)>,

    /// Performance metrics (filled by InteractiveCanvas after render)
    pub render_time_ms: Option<f64>,
}
```

**Step 2: Export from rendering module**

Modify `src/rendering/mod.rs`, add at the end of the module exports:

```rust
pub mod renderer_info;
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src/rendering/renderer_info.rs src/rendering/mod.rs
git commit -m "feat: add RendererInfo trait for UI display"
```

---

## Task 2: Implement RendererInfo for TestImageRenderer

**Files:**
- Modify: `src/components/test_image.rs`

**Step 1: Import RendererInfo trait**

At the top of `src/components/test_image.rs`, add to imports:

```rust
use crate::rendering::renderer_info::{RendererInfo, RendererInfoData};
use crate::rendering::viewport::Viewport;
```

**Step 2: Implement RendererInfo for TestImageRenderer**

Add implementation after the existing `ImagePointComputer` impl (around line 50):

```rust
impl RendererInfo for TestImageRenderer {
    type Coord = f64;

    fn info(&self, viewport: &Viewport<f64>) -> RendererInfoData {
        RendererInfoData {
            name: "Test Image".to_string(),
            center_display: format!("x: {:.2}, y: {:.2}", viewport.center.x, viewport.center.y),
            zoom_display: format!("{:.2}x", viewport.zoom),
            custom_params: vec![
                ("Checkerboard size".to_string(), format!("{:.1}", self.checkerboard_size)),
                ("Circle radius step".to_string(), format!("{:.1}", self.circle_radius_step)),
                ("Circle line thickness".to_string(), format!("{:.2}", self.circle_line_thickness)),
            ],
            render_time_ms: None, // Filled by InteractiveCanvas
        }
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: implement RendererInfo for TestImageRenderer"
```

---

## Task 3: Add CanvasWithInfo Return Type to InteractiveCanvas

**Files:**
- Modify: `src/components/interactive_canvas.rs`

**Step 1: Import dependencies**

At the top of the file, add these imports:

```rust
use crate::rendering::renderer_info::{RendererInfo, RendererInfoData};
use web_sys::window;
```

**Step 2: Add CanvasWithInfo struct before component definition**

Add before the `#[component]` attribute (around line 25):

```rust
/// Return value from InteractiveCanvas containing the view and control signals
pub struct CanvasWithInfo {
    pub view: View,
    pub info: ReadSignal<RendererInfoData>,
    pub reset_viewport: Box<dyn Fn()>,
}
```

**Step 3: Update component signature**

Change the component signature from:

```rust
#[component]
pub fn InteractiveCanvas<T, R>(renderer: R) -> impl IntoView
where
    T: 'static,
    R: Renderer<Coord = T> + Clone + 'static,
```

To:

```rust
#[component]
pub fn InteractiveCanvas<T, R>(renderer: R) -> CanvasWithInfo
where
    T: 'static,
    R: Renderer<Coord = T> + RendererInfo<Coord = T> + Clone + 'static,
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Compilation errors about return type mismatch (we'll fix in next task)

**Step 5: Commit**

```bash
git add src/components/interactive_canvas.rs
git commit -m "feat: add CanvasWithInfo return type (WIP)"
```

---

## Task 4: Add Info Signal and Viewport Reset to InteractiveCanvas

**Files:**
- Modify: `src/components/interactive_canvas.rs`

**Step 1: Create info signal after viewport signal**

Find the line where viewport is created (around line 49):

```rust
let viewport = create_rw_signal(Viewport::new(center, 1.0, natural_bounds));
```

Add immediately after:

```rust
// Create info signal for UI display
let info = create_rw_signal(renderer.info(&viewport.get()));
```

**Step 2: Create viewport reset callback**

Add after the info signal creation:

```rust
// Reset viewport callback for Home button
let renderer_for_reset = renderer.clone();
let reset_viewport = move || {
    let bounds = renderer_for_reset.natural_bounds();
    viewport.set(Viewport::new(
        Point::new(bounds.center().x().clone(), bounds.center().y().clone()),
        1.0,
        bounds,
    ));
};
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Still has errors about return type (we'll fix next)

**Step 4: Commit**

```bash
git add src/components/interactive_canvas.rs
git commit -m "feat: add info signal and viewport reset callback"
```

---

## Task 5: Update Rendering Effect to Track Performance and Update Info

**Files:**
- Modify: `src/components/interactive_canvas.rs`

**Step 1: Find the rendering effect**

Locate the `create_effect` block that handles rendering (around line 82-88). It currently looks like:

```rust
create_effect(move |_| {
    // ... existing code to render
});
```

**Step 2: Wrap render call with performance timing**

Replace the rendering effect with this version that times the render and updates info:

```rust
let renderer_for_render = renderer.clone();
create_effect(move |_| {
    let vp = viewport.get();
    let canvas = canvas_ref.get()?;
    let context = get_2d_context(&canvas)?;
    let width = canvas.width();
    let height = canvas.height();

    // Calculate pixel rect (full canvas for now)
    let pixel_rect = crate::rendering::pixel_rect::PixelRect::new(0, 0, width, height);

    // Time the render
    let start = window()?.performance()?.now();
    let pixels = renderer_for_render.render(&vp, pixel_rect, (width, height));
    let end = window()?.performance()?.now();

    // Render to canvas
    let image_data = web_sys::ImageData::new_with_u8_clamped_array(
        wasm_bindgen::Clamped(&pixels),
        width,
    )
    .ok()?;
    context.put_image_data(&image_data, 0.0, 0.0).ok()?;

    // Update info with performance metrics
    let mut info_data = renderer_for_render.info(&vp);
    info_data.render_time_ms = Some(end - start);
    info.set(info_data);

    Some(())
});
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Errors about PixelRect import and possibly get_2d_context (we need to check what helpers exist)

**Step 4: Check what rendering code already exists**

Read the current implementation to see what helper functions are available:

Look for how rendering is currently done in the effect.

**Step 5: Adjust based on existing code**

Once you see the existing rendering pattern, adapt the timing code to wrap the existing render logic rather than replacing it entirely.

**Step 6: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 7: Commit**

```bash
git add src/components/interactive_canvas.rs
git commit -m "feat: add performance timing and info updates to render effect"
```

---

## Task 6: Update InteractiveCanvas Return Statement

**Files:**
- Modify: `src/components/interactive_canvas.rs`

**Step 1: Find the return statement**

At the end of the `InteractiveCanvas` component, find the current return (around line 120):

```rust
view! {
    <canvas _ref=canvas_ref class="w-full h-full" />
}
```

**Step 2: Replace with CanvasWithInfo return**

Replace the return with:

```rust
CanvasWithInfo {
    view: view! {
        <canvas
            _ref=canvas_ref
            class="w-full h-full"
            on:pointerdown=handle.on_pointer_down
            on:pointermove=handle.on_pointer_move
            on:pointerup=handle.on_pointer_up
            on:wheel=handle.on_wheel
        />
    }
    .into_view(),
    info: info.read_only(),
    reset_viewport: Box::new(reset_viewport),
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/interactive_canvas.rs
git commit -m "feat: return CanvasWithInfo from InteractiveCanvas"
```

---

## Task 7: Create Fullscreen Utility Module

**Files:**
- Create: `src/utils/mod.rs`
- Create: `src/utils/fullscreen.rs`
- Modify: `src/lib.rs`

**Step 1: Create utils module structure**

Create `src/utils/mod.rs`:

```rust
pub mod fullscreen;
```

**Step 2: Create fullscreen utility**

Create `src/utils/fullscreen.rs`:

```rust
use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::{Document, window};

/// Toggle fullscreen mode for the document
pub fn toggle_fullscreen() {
    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if is_fullscreen(&document) {
                let _ = document.exit_fullscreen();
            } else if let Some(element) = document.document_element() {
                let _ = element.request_fullscreen();
            }
        }
    }
}

/// Check if currently in fullscreen mode
fn is_fullscreen(document: &Document) -> bool {
    document.fullscreen_element().is_some()
}

/// Leptos hook to track fullscreen state reactively
pub fn use_fullscreen() -> (ReadSignal<bool>, impl Fn()) {
    let (is_fullscreen, set_is_fullscreen) = create_signal(false);

    // Set up fullscreen change listener
    create_effect(move |_| {
        if let Some(window) = window() {
            if let Some(document) = window.document() {
                let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Some(doc) = window().and_then(|w| w.document()) {
                        set_is_fullscreen.set(doc.fullscreen_element().is_some());
                    }
                }) as Box<dyn FnMut(_)>);

                let _ = document.add_event_listener_with_callback(
                    "fullscreenchange",
                    closure.as_ref().unchecked_ref(),
                );

                closure.forget(); // Keep listener alive
            }
        }
    });

    (is_fullscreen, toggle_fullscreen)
}
```

**Step 3: Add utils module to lib.rs**

Modify `src/lib.rs`, add after the existing modules:

```rust
mod utils;
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 5: Commit**

```bash
git add src/utils/mod.rs src/utils/fullscreen.rs src/lib.rs
git commit -m "feat: add fullscreen utility with reactive hook"
```

---

## Task 8: Create UI Button Components

**Files:**
- Modify: `src/components/ui.rs`

**Step 1: Add imports**

Replace the entire file contents. Start with imports:

```rust
use leptos::*;
use crate::rendering::renderer_info::RendererInfoData;
use crate::utils::fullscreen::use_fullscreen;
```

**Step 2: Create icon components**

Add icon components (inline SVGs):

```rust
#[component]
fn InfoIcon() -> impl IntoView {
    view! {
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="16" x2="12" y2="12"/>
        <line x1="12" y1="8" x2="12.01" y2="8"/>
      </svg>
    }
}

#[component]
fn HomeIcon() -> impl IntoView {
    view! {
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
        <polyline points="9 22 9 12 15 12 15 22"/>
      </svg>
    }
}

#[component]
fn MaximizeIcon() -> impl IntoView {
    view! {
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"/>
      </svg>
    }
}

#[component]
fn MinimizeIcon() -> impl IntoView {
    view! {
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M8 3v3a2 2 0 0 1-2 2H3m18 0h-3a2 2 0 0 1-2-2V3m0 18v-3a2 2 0 0 1 2-2h3M3 16h3a2 2 0 0 1 2 2v3"/>
      </svg>
    }
}

#[component]
fn GithubIcon() -> impl IntoView {
    view! {
      <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
      </svg>
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/ui.rs
git commit -m "feat: add UI icon components"
```

---

## Task 9: Create Button Components

**Files:**
- Modify: `src/components/ui.rs`

**Step 1: Add InfoButton with popover**

Add after the icon components:

```rust
#[component]
fn InfoButton(is_open: ReadSignal<bool>, set_is_open: WriteSignal<bool>) -> impl IntoView {
    view! {
      <div class="relative">
        <button
          class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
          on:click=move |_| set_is_open.set(!is_open.get())
        >
          <InfoIcon />
        </button>

        {move || is_open.get().then(|| view! {
          <div class="absolute bottom-full mb-3 left-0 w-80 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg p-4 text-white">
            <h3 class="font-medium mb-2">"Fractal Wonder"</h3>
            <p class="text-sm text-gray-300 mb-4">
              "Use mouse/touch to pan and zoom. Keyboard shortcuts: [ and ] to cycle color schemes."
            </p>
            <div class="flex items-center gap-2 text-sm text-gray-400">
              <a
                href="https://github.com/gertalot/fractalwonder"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white hover:text-gray-200 transition-colors"
              >
                <GithubIcon />
              </a>
              <span>"Made by Gert"</span>
            </div>
          </div>
        })}
      </div>
    }
}

#[component]
fn HomeButton(on_click: impl Fn() + 'static) -> impl IntoView {
    view! {
      <button
        class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
        on:click=move |_| on_click()
      >
        <HomeIcon />
      </button>
    }
}

#[component]
fn FullscreenButton(on_click: impl Fn() + 'static) -> impl IntoView {
    let (is_fullscreen, _) = use_fullscreen();

    view! {
      <button
        class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
        on:click=move |_| on_click()
      >
        {move || if is_fullscreen.get() {
          view! { <MinimizeIcon /> }
        } else {
          view! { <MaximizeIcon /> }
        }}
      </button>
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/components/ui.rs
git commit -m "feat: add InfoButton, HomeButton, and FullscreenButton components"
```

---

## Task 10: Create InfoDisplay Component

**Files:**
- Modify: `src/components/ui.rs`

**Step 1: Add InfoDisplay component**

Add after the button components:

```rust
#[component]
fn InfoDisplay(info: ReadSignal<RendererInfoData>) -> impl IntoView {
    view! {
      <div class="text-white text-sm">
        <p>
          {move || {
            let i = info.get();
            format!("Center: {}, zoom: {}", i.center_display, i.zoom_display)
          }}
          {move || {
            info.get().render_time_ms.map(|ms|
              format!(", render: {:.2}s", ms / 1000.0)
            ).unwrap_or_default()
          }}
        </p>
        <p>
          "Algorithm: "
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

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/components/ui.rs
git commit -m "feat: add InfoDisplay component"
```

---

## Task 11: Create Main UI Component

**Files:**
- Modify: `src/components/ui.rs`

**Step 1: Add main UI component**

Add at the end of the file:

```rust
#[component]
pub fn UI(
    info: ReadSignal<RendererInfoData>,
    is_visible: ReadSignal<bool>,
    set_is_hovering: WriteSignal<bool>,
    on_home_click: impl Fn() + 'static,
    on_fullscreen_click: impl Fn() + 'static,
) -> impl IntoView {
    let (is_popover_open, set_is_popover_open) = create_signal(false);

    // Keep UI visible when popover is open
    create_effect(move |_| {
        if is_popover_open.get() {
            set_is_hovering.set(true);
        }
    });

    let opacity_class = move || {
        if is_visible.get() {
            "opacity-100"
        } else {
            "opacity-0"
        }
    };

    view! {
      <div
        class=move || format!(
          "fixed inset-x-0 bottom-0 transition-opacity duration-300 {}",
          opacity_class()
        )
        on:mouseenter=move |_| set_is_hovering.set(true)
        on:mouseleave=move |_| set_is_hovering.set(false)
      >
        <div class="flex items-center justify-between px-4 py-3 bg-black/50 backdrop-blur-sm">
          // Left section: buttons
          <div class="flex items-center space-x-4">
            <InfoButton is_open=is_popover_open set_is_open=set_is_popover_open />
            <HomeButton on_click=on_home_click />
          </div>

          // Center section: info display
          <div class="flex-1 text-center">
            <InfoDisplay info=info />
          </div>

          // Right section: fullscreen
          <div>
            <FullscreenButton on_click=on_fullscreen_click />
          </div>
        </div>
      </div>
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/components/ui.rs
git commit -m "feat: add main UI component with layout"
```

---

## Task 12: Update TestImageView to Wire Everything Together

**Files:**
- Modify: `src/components/test_image.rs`

**Step 1: Add imports**

At the top of the file, add:

```rust
use crate::components::ui::UI;
use crate::components::ui_visibility::use_ui_visibility;
use crate::utils::fullscreen::toggle_fullscreen;
```

**Step 2: Update TestImageView component**

Find the `TestImageView` component (around line 65) and replace it entirely:

```rust
#[component]
pub fn TestImageView() -> impl IntoView {
    let renderer = PixelRenderer::new(TestImageRenderer::new());
    let canvas_with_info = InteractiveCanvas(renderer);

    // UI visibility
    let ui_visibility = use_ui_visibility();

    // Clone reset callback for use in closure
    let reset_fn = canvas_with_info.reset_viewport;
    let on_home_click = move || {
        reset_fn();
    };

    // Fullscreen callback
    let on_fullscreen_click = move || {
        toggle_fullscreen();
    };

    view! {
        <div class="w-full h-full">
            {canvas_with_info.view}
        </div>
        <UI
            info=canvas_with_info.info
            is_visible=ui_visibility.is_visible
            set_is_hovering=ui_visibility.set_is_hovering
            on_home_click=on_home_click
            on_fullscreen_click=on_fullscreen_click
        />
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Possible type errors about Box<dyn Fn()> not implementing Copy

**Step 4: Fix callback handling if needed**

If there are errors about the reset callback, wrap it properly:

```rust
let reset_fn = canvas_with_info.reset_viewport;
let on_home_click = move || {
    (reset_fn)();
};
```

**Step 5: Verify compilation again**

Run: `cargo check`
Expected: No errors

**Step 6: Commit**

```bash
git add src/components/test_image.rs
git commit -m "feat: wire UI buttons to TestImageView"
```

---

## Task 13: Update App Component

**Files:**
- Modify: `src/app.rs`

**Step 1: Simplify App component**

The App component should now just render the selected view. Replace the entire App component:

```rust
use leptos::*;
use crate::components::test_image::TestImageView;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererType {
    TestImage,
}

#[component]
pub fn App() -> impl IntoView {
    // Currently fixed to TestImage
    let current_renderer = RendererType::TestImage;

    view! {
      <div class="relative w-screen h-screen overflow-hidden bg-black">
        {match current_renderer {
          RendererType::TestImage => {
            view! { <TestImageView /> }.into_view()
          }
        }}
      </div>
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/app.rs
git commit -m "refactor: simplify App component after UI refactor"
```

---

## Task 14: Build and Test in Browser

**Files:**
- None (testing only)

**Step 1: Build the project**

Run: `trunk build`
Expected: Build succeeds

**Step 2: Start dev server**

Run: `trunk serve` (or it should already be running)
Expected: Server starts on http://localhost:8080

**Step 3: Test UI in browser**

Manual testing checklist:
- [ ] Test image renders correctly
- [ ] UI bar appears at bottom
- [ ] UI fades out after inactivity
- [ ] UI reappears on hover
- [ ] Info button opens popover
- [ ] Popover keeps UI visible
- [ ] Popover closes on second click
- [ ] Home button resets viewport to center/zoom 1.0
- [ ] Fullscreen button toggles fullscreen
- [ ] Fullscreen icon changes state
- [ ] Center coordinates display correctly
- [ ] Zoom level displays correctly
- [ ] Custom parameters show (checkerboard size, etc.)
- [ ] Render time displays after first render
- [ ] GitHub link opens correctly

**Step 4: Document any issues found**

If issues found, create follow-up tasks.

**Step 5: Take screenshot**

If UI works, take a screenshot for documentation.

---

## Task 15: Run Full Test Suite

**Files:**
- None (testing only)

**Step 1: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 3: Run tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests pass

**Step 4: Run WASM tests**

Run: `wasm-pack test --headless --chrome`
Expected: All tests pass

**Step 5: Commit any formatting changes**

```bash
git add -u
git commit -m "chore: format code"
```

---

## Task 16: Final Integration Commit

**Files:**
- None (final commit only)

**Step 1: Review all changes**

Run: `git log --oneline origin/refactor-canvas-renderer..HEAD`
Expected: See all commits from this implementation

**Step 2: Verify clean working directory**

Run: `git status`
Expected: Nothing to commit, working tree clean

**Step 3: Create summary**

Document what was implemented:
- RendererInfo trait for generic renderer metadata
- CanvasWithInfo return type from InteractiveCanvas
- Performance timing in render loop
- Fullscreen utility with reactive hook
- Complete UI component system (Info, Home, Fullscreen buttons)
- Wired everything together in TestImageView

**Step 4: Optional: Create annotated tag**

```bash
git tag -a ui-v1 -m "UI system with renderer info display"
```

---

## Post-Implementation Notes

**Future Enhancements:**
- Add progress bar for long renders
- Add keyboard shortcuts (F for fullscreen, H for home, I for info)
- Add render quality/performance toggles
- Make custom parameters clickable to adjust values
- Add tooltip to display full coordinate precision on hover
- Add copy-to-clipboard for coordinates

**Testing Considerations:**
- Most UI logic is in components, could add unit tests
- Fullscreen API requires user interaction, hard to test automatically
- Consider adding integration tests with browser automation

**Performance Considerations:**
- Info signal updates after every render - acceptable for now
- Popover uses conditional rendering - efficient
- Icons are inline SVG - no external requests but increases bundle size slightly
- Consider extracting icons to separate module if adding many more

**Architecture Notes:**
- Each renderer view (TestImageView, future FractalView) owns its UI wiring
- Allows different renderers to have different UI configurations
- RendererInfo trait is optional - renderers can choose not to implement
- InteractiveCanvas stays generic and reusable
