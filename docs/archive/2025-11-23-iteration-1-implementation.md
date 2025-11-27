# Iteration 1: Canvas with Static Pattern - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prove we can render pixels to a canvas using ImageData pixel manipulation.

**Architecture:** Create `InteractiveCanvas` component that fills browser viewport with a position-dependent gradient. Extract gradient calculation to testable function, use ImageData for pixel manipulation.

**Tech Stack:** Leptos 0.6, web-sys (HtmlCanvasElement, CanvasRenderingContext2d, ImageData)

---

## Task 1: Create Components Module Structure

**Files:**
- Create: `fractalwonder-ui/src/components/mod.rs`
- Modify: `fractalwonder-ui/src/lib.rs`

**Step 1: Create components module file**

Create `fractalwonder-ui/src/components/mod.rs`:
```rust
mod interactive_canvas;

pub use interactive_canvas::InteractiveCanvas;
```

**Step 2: Create placeholder interactive_canvas module**

Create `fractalwonder-ui/src/components/interactive_canvas.rs`:
```rust
use leptos::*;

#[component]
pub fn InteractiveCanvas() -> impl IntoView {
    view! {
        <canvas class="block" />
    }
}
```

**Step 3: Add components module to lib.rs**

Modify `fractalwonder-ui/src/lib.rs` to add `mod components;` after `mod app;`:
```rust
mod app;
mod components;
pub mod hooks;
```

**Step 4: Run cargo check to verify structure**

Run: `cargo check --workspace`
Expected: Compiles successfully (warning about unused components is OK)

**Step 5: Commit**

```bash
git add -A && git commit -m "feat(ui): add components module structure"
```

---

## Task 2: Write and Test Gradient Function

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Write failing test for gradient function**

Add to `fractalwonder-ui/src/components/interactive_canvas.rs`:
```rust
use leptos::*;

/// Calculate gradient color for a pixel position.
/// R increases left-to-right, G increases top-to-bottom, B constant at 128.
fn gradient_color(x: u32, y: u32, width: u32, height: u32) -> [u8; 4] {
    todo!()
}

#[component]
pub fn InteractiveCanvas() -> impl IntoView {
    view! {
        <canvas class="block" />
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

**Step 2: Run tests to verify they fail**

Run: `cargo test --package fractalwonder-ui gradient`
Expected: FAIL with "not yet implemented"

**Step 3: Implement gradient_color function**

Replace the `todo!()` with:
```rust
fn gradient_color(x: u32, y: u32, width: u32, height: u32) -> [u8; 4] {
    let r = ((x as f64 / width as f64) * 255.0) as u8;
    let g = ((y as f64 / height as f64) * 255.0) as u8;
    let b = 128u8;
    let a = 255u8;
    [r, g, b, a]
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package fractalwonder-ui gradient`
Expected: All 3 tests PASS

**Step 5: Commit**

```bash
git add -A && git commit -m "feat(ui): add gradient_color function with tests"
```

---

## Task 3: Implement Canvas Rendering

**Files:**
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Add required imports**

Update imports at top of `interactive_canvas.rs`:
```rust
use leptos::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::Clamped;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};
```

**Step 2: Implement the component with rendering effect**

Replace the `InteractiveCanvas` component with:
```rust
#[component]
pub fn InteractiveCanvas() -> impl IntoView {
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
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&data),
            width,
            height,
        )
        .expect("should create ImageData");
        ctx.put_image_data(&image_data, 0.0, 0.0)
            .expect("should put image data");
    });

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}
```

**Step 3: Run cargo check**

Run: `cargo check --workspace`
Expected: Compiles successfully

**Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 5: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No errors or warnings

**Step 6: Commit**

```bash
git add -A && git commit -m "feat(ui): implement canvas rendering with ImageData"
```

---

## Task 4: Wire Up App Component

**Files:**
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Update app.rs to use InteractiveCanvas**

Replace contents of `fractalwonder-ui/src/app.rs` with:
```rust
use leptos::*;

use crate::components::InteractiveCanvas;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <InteractiveCanvas />
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check --workspace`
Expected: Compiles successfully

**Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 4: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No errors or warnings

**Step 5: Commit**

```bash
git add -A && git commit -m "feat(ui): wire InteractiveCanvas into App"
```

---

## Task 5: Browser Verification

**Step 1: Ensure trunk serve is running**

Verify `trunk serve` is running on host.

**Step 2: Open browser and verify**

Open: `http://127.0.0.1:8000/fractalwonder`

Expected:
- Gradient fills entire browser viewport
- Red intensity increases left to right
- Green intensity increases top to bottom
- Blue constant (creates purple tint in bottom-right)
- No console errors

**Step 3: Final commit if any fixes needed**

If changes were needed:
```bash
git add -A && git commit -m "fix(ui): address browser testing feedback"
```

---

## Summary

| Task | Description | Tests |
|------|-------------|-------|
| 1 | Module structure | cargo check |
| 2 | Gradient function | 3 unit tests |
| 3 | Canvas rendering | cargo check + clippy |
| 4 | App wiring | cargo check + clippy |
| 5 | Browser verification | Visual |

**Total commits:** 4-5
**Estimated time:** 15-20 minutes
