# Iteration 2: UI Panel Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a UI panel overlay with autohide, fullscreen toggle, info button, and canvas dimensions display.

**Architecture:** Bottom bar overlay that fades based on mouse activity. Two hooks (`use_fullscreen`, `use_ui_visibility`) manage browser APIs. `InteractiveCanvas` exposes canvas dimensions via callback, `App` composes everything.

**Tech Stack:** Leptos 0.6, leptos-use (timeout/events), web-sys (fullscreen API), Tailwind CSS

---

## Task 1: Add use_fullscreen Hook

**Files:**
- Create: `fractalwonder-ui/src/hooks/fullscreen.rs`
- Modify: `fractalwonder-ui/src/hooks/mod.rs`

**Step 1: Create fullscreen.rs with hook**

```rust
// fractalwonder-ui/src/hooks/fullscreen.rs
use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::{window, Document};

/// Toggle fullscreen mode for the document
pub fn toggle_fullscreen() {
    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if is_fullscreen(&document) {
                document.exit_fullscreen();
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

/// Leptos hook to track fullscreen state reactively.
/// Returns (is_fullscreen signal, toggle function).
pub fn use_fullscreen() -> (ReadSignal<bool>, impl Fn()) {
    let (is_fullscreen_signal, set_is_fullscreen) = create_signal(false);

    create_effect(move |_| {
        if let Some(win) = window() {
            if let Some(document) = win.document() {
                // Set initial state
                set_is_fullscreen.set(document.fullscreen_element().is_some());

                let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Some(doc) = window().and_then(|w| w.document()) {
                        set_is_fullscreen.set(doc.fullscreen_element().is_some());
                    }
                }) as Box<dyn FnMut(_)>);

                let _ = document.add_event_listener_with_callback(
                    "fullscreenchange",
                    closure.as_ref().unchecked_ref(),
                );

                closure.forget();
            }
        }
    });

    (is_fullscreen_signal, toggle_fullscreen)
}
```

**Step 2: Export from hooks/mod.rs**

Add to `fractalwonder-ui/src/hooks/mod.rs`:

```rust
mod fullscreen;
mod use_canvas_interaction;

pub use fullscreen::{toggle_fullscreen, use_fullscreen};
pub use use_canvas_interaction::{use_canvas_interaction, InteractionHandle};

// Re-export PixelTransform for convenience (so users can import from hooks module)
pub use fractalwonder_core::PixelTransform;
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Success, no errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/hooks/fullscreen.rs fractalwonder-ui/src/hooks/mod.rs
git commit -m "feat(ui): add use_fullscreen hook"
```

---

## Task 2: Add use_ui_visibility Hook

**Files:**
- Create: `fractalwonder-ui/src/hooks/ui_visibility.rs`
- Modify: `fractalwonder-ui/src/hooks/mod.rs`

**Step 1: Create ui_visibility.rs with hook**

```rust
// fractalwonder-ui/src/hooks/ui_visibility.rs
use leptos::*;

const UI_HIDE_DELAY_MS: f64 = 2000.0;

/// UI visibility state returned by use_ui_visibility hook
#[derive(Debug, Clone, Copy)]
pub struct UiVisibility {
    pub is_visible: ReadSignal<bool>,
    pub set_is_visible: WriteSignal<bool>,
    pub is_hovering: ReadSignal<bool>,
    pub set_is_hovering: WriteSignal<bool>,
}

/// Hook that manages UI panel visibility with autohide.
/// - Starts visible
/// - Hides after 2s of mouse inactivity (unless hovering over UI)
/// - Shows on any mouse movement
pub fn use_ui_visibility() -> UiVisibility {
    let (is_visible, set_is_visible) = create_signal(true);
    let (is_hovering, set_is_hovering) = create_signal(false);

    // Create a timer that fires after delay
    let timeout_fn = leptos_use::use_timeout_fn(
        move |_: ()| {
            // Only hide if not hovering over UI
            if !is_hovering.get_untracked() {
                set_is_visible.set(false);
            }
        },
        UI_HIDE_DELAY_MS,
    );

    // Start the initial timer immediately
    (timeout_fn.start)(());

    // Listen for mouse movement on the window
    let _ = leptos_use::use_event_listener(
        leptos_use::use_window(),
        leptos::ev::mousemove,
        move |_| {
            set_is_visible.set(true);
            // Cancel previous timer before starting new one
            (timeout_fn.stop)();
            (timeout_fn.start)(());
        },
    );

    UiVisibility {
        is_visible,
        set_is_visible,
        is_hovering,
        set_is_hovering,
    }
}
```

**Step 2: Export from hooks/mod.rs**

Update `fractalwonder-ui/src/hooks/mod.rs`:

```rust
mod fullscreen;
mod ui_visibility;
mod use_canvas_interaction;

pub use fullscreen::{toggle_fullscreen, use_fullscreen};
pub use ui_visibility::{use_ui_visibility, UiVisibility};
pub use use_canvas_interaction::{use_canvas_interaction, InteractionHandle};

// Re-export PixelTransform for convenience (so users can import from hooks module)
pub use fractalwonder_core::PixelTransform;
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Success, no errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/hooks/ui_visibility.rs fractalwonder-ui/src/hooks/mod.rs
git commit -m "feat(ui): add use_ui_visibility hook with autohide"
```

---

## Task 3: Add FullscreenButton Component

**Files:**
- Create: `fractalwonder-ui/src/components/fullscreen_button.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create fullscreen_button.rs**

```rust
// fractalwonder-ui/src/components/fullscreen_button.rs
use crate::hooks::use_fullscreen;
use leptos::*;

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
pub fn FullscreenButton() -> impl IntoView {
    let (is_fullscreen, toggle) = use_fullscreen();

    view! {
        <button
            class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
            on:click=move |_| toggle()
        >
            {move || if is_fullscreen.get() {
                view! { <MinimizeIcon /> }.into_view()
            } else {
                view! { <MaximizeIcon /> }.into_view()
            }}
        </button>
    }
}
```

**Step 2: Export from components/mod.rs**

Update `fractalwonder-ui/src/components/mod.rs`:

```rust
mod fullscreen_button;
mod interactive_canvas;

pub use fullscreen_button::FullscreenButton;
pub use interactive_canvas::InteractiveCanvas;
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Success, no errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/fullscreen_button.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(ui): add FullscreenButton component"
```

---

## Task 4: Add InfoButton Component

**Files:**
- Create: `fractalwonder-ui/src/components/info_button.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create info_button.rs**

```rust
// fractalwonder-ui/src/components/info_button.rs
use leptos::*;

#[component]
fn InfoIcon() -> impl IntoView {
    view! {
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="16" x2="12" y2="12"/>
            <circle cx="12" cy="8" r="0.5" fill="currentColor"/>
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

#[component]
pub fn InfoButton() -> impl IntoView {
    let (is_open, set_is_open) = create_signal(false);

    view! {
        <div class="relative">
            <button
                class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                <InfoIcon />
            </button>

            {move || is_open.get().then(|| view! {
                <div class="absolute bottom-full mb-3 left-0 w-80 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg p-4 text-white">
                    <h3 class="font-medium mb-2">"Fractal Wonder"</h3>
                    <p class="text-sm text-gray-300 mb-4">
                        "Use mouse/touch to pan and zoom."
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
```

**Step 2: Export from components/mod.rs**

Update `fractalwonder-ui/src/components/mod.rs`:

```rust
mod fullscreen_button;
mod info_button;
mod interactive_canvas;

pub use fullscreen_button::FullscreenButton;
pub use info_button::InfoButton;
pub use interactive_canvas::InteractiveCanvas;
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Success, no errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/info_button.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(ui): add InfoButton component with popover"
```

---

## Task 5: Modify InteractiveCanvas to Report Dimensions

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Add on_resize callback prop**

Replace the entire file with:

```rust
// fractalwonder-ui/src/components/interactive_canvas.rs
use leptos::*;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Calculate gradient color for a pixel position.
/// R increases left-to-right, G increases top-to-bottom, B constant at 128.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn gradient_color(x: u32, y: u32, width: u32, height: u32) -> [u8; 4] {
    let r = ((x as f64 / width as f64) * 255.0) as u8;
    let g = ((y as f64 / height as f64) * 255.0) as u8;
    let b = 128u8;
    let a = 255u8;
    [r, g, b, a]
}

#[component]
pub fn InteractiveCanvas(
    /// Callback fired when canvas dimensions change, receives (width, height)
    #[prop(optional)]
    on_resize: Option<Callback<(u32, u32)>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    create_effect(move |_| {
        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        // Set canvas dimensions to fill viewport
        let window = web_sys::window().expect("should have window");
        let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
        let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
        canvas.set_width(width);
        canvas.set_height(height);

        // Notify parent of dimensions
        if let Some(callback) = on_resize {
            callback.call((width, height));
        }

        // Get 2D rendering context
        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .unchecked_into::<CanvasRenderingContext2d>();

        // Create pixel buffer and fill with gradient
        let mut data = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let [r, g, b, a] = gradient_color(x, y, width, height);
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = a;
            }
        }

        // Create ImageData and draw to canvas
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&data), width, height)
            .expect("should create ImageData");
        ctx.put_image_data(&image_data, 0.0, 0.0)
            .expect("should put image data");
    });

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_top_left_is_green_blue() {
        let [r, g, b, a] = gradient_color(0, 0, 100, 100);
        assert_eq!(r, 0, "top-left red should be 0");
        assert_eq!(g, 0, "top-left green should be 0");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }

    #[test]
    fn gradient_bottom_right_is_red_green_blue() {
        let [r, g, b, a] = gradient_color(99, 99, 100, 100);
        // 99/100 * 255 = 252.45 -> 252
        assert_eq!(r, 252, "bottom-right red should be ~252");
        assert_eq!(g, 252, "bottom-right green should be ~252");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }

    #[test]
    fn gradient_center_is_half_intensity() {
        let [r, g, b, a] = gradient_color(50, 50, 100, 100);
        // 50/100 * 255 = 127.5 -> 127
        assert_eq!(r, 127, "center red should be ~127");
        assert_eq!(g, 127, "center green should be ~127");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }
}
```

**Step 2: Verify tests pass**

Run: `cargo test -p fractalwonder-ui`
Expected: All tests pass

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/interactive_canvas.rs
git commit -m "feat(ui): add on_resize callback to InteractiveCanvas"
```

---

## Task 6: Add UIPanel Component

**Files:**
- Create: `fractalwonder-ui/src/components/ui_panel.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`

**Step 1: Create ui_panel.rs**

```rust
// fractalwonder-ui/src/components/ui_panel.rs
use crate::components::{FullscreenButton, InfoButton};
use crate::hooks::{use_ui_visibility, UiVisibility};
use leptos::*;

#[component]
pub fn UIPanel(
    /// Canvas dimensions to display, as (width, height)
    canvas_size: Signal<(u32, u32)>,
) -> impl IntoView {
    let UiVisibility {
        is_visible,
        is_hovering: _,
        set_is_visible: _,
        set_is_hovering,
    } = use_ui_visibility();

    let opacity_class = move || {
        if is_visible.get() {
            "opacity-100"
        } else {
            "opacity-0 pointer-events-none"
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
                // Left section: info button
                <div class="flex items-center space-x-2">
                    <InfoButton />
                </div>

                // Center section: canvas dimensions
                <div class="flex-1 text-center text-white text-sm">
                    {move || {
                        let (w, h) = canvas_size.get();
                        format!("Canvas: {} × {}", w, h)
                    }}
                </div>

                // Right section: fullscreen
                <div>
                    <FullscreenButton />
                </div>
            </div>
        </div>
    }
}
```

**Step 2: Export from components/mod.rs**

Update `fractalwonder-ui/src/components/mod.rs`:

```rust
mod fullscreen_button;
mod info_button;
mod interactive_canvas;
mod ui_panel;

pub use fullscreen_button::FullscreenButton;
pub use info_button::InfoButton;
pub use interactive_canvas::InteractiveCanvas;
pub use ui_panel::UIPanel;
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Success, no errors

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/components/ui_panel.rs fractalwonder-ui/src/components/mod.rs
git commit -m "feat(ui): add UIPanel component with autohide"
```

---

## Task 7: Wire Everything in App

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Update App to compose components**

```rust
// fractalwonder-ui/src/app.rs
use leptos::*;

use crate::components::{InteractiveCanvas, UIPanel};

#[component]
pub fn App() -> impl IntoView {
    let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

    let on_resize = Callback::new(move |size: (u32, u32)| {
        set_canvas_size.set(size);
    });

    view! {
        <InteractiveCanvas on_resize=on_resize />
        <UIPanel canvas_size=canvas_size.into() />
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Success, no errors

**Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/app.rs
git commit -m "feat(ui): wire UIPanel into App"
```

---

## Task 8: Browser Verification

**Step 1: Ensure trunk is serving**

The dev server should already be running at http://localhost:8080

**Step 2: Manual browser tests**

Open http://localhost:8080 and verify:

1. [ ] Gradient canvas fills viewport
2. [ ] UI panel visible at bottom
3. [ ] Panel shows "Canvas: [width] × [height]"
4. [ ] Click "i" button → info popover appears
5. [ ] Click fullscreen button → app goes fullscreen
6. [ ] Leave mouse idle 2s → panel fades out
7. [ ] Move mouse → panel fades back in
8. [ ] Hover over panel → panel stays visible

**Step 3: Final commit if any fixes needed**

If browser tests reveal issues, fix and commit.

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | use_fullscreen hook | hooks/fullscreen.rs |
| 2 | use_ui_visibility hook | hooks/ui_visibility.rs |
| 3 | FullscreenButton component | components/fullscreen_button.rs |
| 4 | InfoButton component | components/info_button.rs |
| 5 | InteractiveCanvas on_resize | components/interactive_canvas.rs |
| 6 | UIPanel component | components/ui_panel.rs |
| 7 | Wire in App | app.rs |
| 8 | Browser verification | Manual testing |
