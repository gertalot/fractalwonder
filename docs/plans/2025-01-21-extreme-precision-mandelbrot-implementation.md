# Extreme-Precision Mandelbrot Explorer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a ground-up Mandelbrot explorer supporting extreme zoom levels (10^2000+) with strict precision enforcement and no legacy compromises.

**Architecture:** Three-crate workspace with strict pixel/fractal space separation. Workers handle BigFloat computation, main thread handles UI/colorization/rendering. BigFloat uses enum-based f64/FBig switching for performance while enforcing explicit precision everywhere.

**Design Reference:** See `2025-01-21-extreme-precision-mandelbrot-design.md` for architecture rationale.

**Tech Stack:** Rust 1.80+, Leptos 0.6+, dashu/dashu-float 0.4, Web Workers, WASM, Trunk

---

## Prerequisites

### Task 0: Archive Existing Code

**What this delivers:** Clean slate - existing code preserved in `_archive/`, ready for ground-up rebuild.

**Files:**
- Move: All current `fractalwonder-*` directories → `_archive/`
- Keep: `Cargo.toml` (will be rewritten), `Trunk.toml`, `docs/`, `.claude/`

**Step 1: Create archive directory**

```bash
mkdir -p _archive
```

**Step 2: Move existing crates**

```bash
mv fractalwonder-ui _archive/
mv fractalwonder-compute _archive/
mv fractalwonder-core _archive/
mv tests _archive/ 2>/dev/null || true
```

**Step 3: Verify structure**

Run: `ls -la`
Expected: Only `_archive/`, `Cargo.toml`, `Trunk.toml`, `docs/`, `.claude/`, `target/` remain

**Step 4: Test**

`./scripts/run-all-checks.sh` has no errors. `trunk serve` has no errors.

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: archive existing implementation for ground-up rebuild"
```

**Deliverable:** Existing code safely archived. Ready to build from scratch.

---

## Stage 0: Basic Web App Skeleton

**Goal:** Minimal working Leptos app with static canvas. No interaction, no computation yet. Validates basic plumbing.

### Task 0.1: Create Minimal Workspace with Hello World

**What this delivers:** Working Leptos app displaying "Fractal Wonder" text in browser.

**Files:**
- Create: Root `Cargo.toml`, `fractalwonder-ui/Cargo.toml`, `fractalwonder-ui/src/lib.rs`, `index.html`

**Step 1: Create root workspace manifest**

Create `Cargo.toml`:

```toml
[workspace]
members = ["fractalwonder-ui"]
resolver = "2"

[workspace.dependencies]
# Leptos framework
leptos = { version = "0.6", features = ["csr", "nightly"] }
leptos_meta = { version = "0.6", features = ["csr"] }

# WASM/JS bindings
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3" }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
console_error_panic_hook = "0.1"
console_log = "1.0"

# Testing
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
```

**Step 2: Create UI crate manifest**

Create `fractalwonder-ui/Cargo.toml`:

```toml
[package]
name = "fractalwonder-ui"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
leptos = { workspace = true }
leptos_meta = { workspace = true }
wasm-bindgen = { workspace = true }
console_error_panic_hook = { workspace = true }
console_log = { workspace = true }
```

**Step 3: Create minimal Leptos app**

Create `fractalwonder-ui/src/lib.rs`:

```rust
use leptos::*;
use wasm_bindgen::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <div style="width: 100vw; height: 100vh; display: flex; align-items: center; justify-content: center; background: #1a1a1a; color: white;">
            <h1>"Fractal Wonder - Stage 0"</h1>
        </div>
    }
}

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    leptos::mount_to_body(App);
}
```

**Step 4: Create index.html**

Create `index.html`:

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Fractal Wonder</title>
    <link data-trunk rel="rust" data-wasm-opt="z" data-bin="fractalwonder-ui" />
    <style>
      html, body {
        margin: 0;
        padding: 0;
        width: 100%;
        height: 100%;
        overflow: hidden;
      }
    </style>
  </head>
  <body></body>
</html>
```

**Step 5: Verify it builds and runs**

Run: `cargo check --workspace`
Expected: Success

Run: `trunk serve` (in separate terminal)
Expected: Server starts on http://localhost:8080

Open browser: http://localhost:8080
Expected: See "Fractal Wonder - Stage 0" centered on dark background

**Step 6: Test**

`./scripts/run-all-checks.sh` has no errors. `trunk serve` has no errors.


**Step 7: Commit**

```bash
git add -A
git commit -m "feat(stage0): minimal Leptos hello world app"
```

**Deliverable:** Working web app. Foundation for all future work.

---

### Task 0.2: Add Canvas with Static Test Pattern

**What this delivers:** Full-screen canvas rendering a static test pattern (gradient). No interaction yet.

**Files:**
- Create: `fractalwonder-ui/src/app.rs`
- Create: `fractalwonder-ui/src/components/mod.rs`
- Create: `fractalwonder-ui/src/components/test_canvas.rs`
- Modify: `fractalwonder-ui/src/lib.rs`
- Modify: `fractalwonder-ui/Cargo.toml`

**Step 1: Add web-sys features to Cargo.toml**

Modify `fractalwonder-ui/Cargo.toml`, update web-sys dependency:

```toml
web-sys = { workspace = true, features = [
    "CanvasRenderingContext2d",
    "Document",
    "Element",
    "HtmlCanvasElement",
    "ImageData",
    "Window",
] }
```

**Step 2: Create app module**

Create `fractalwonder-ui/src/app.rs`:

```rust
use leptos::*;

mod components;
use components::TestCanvas;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <TestCanvas />
    }
}
```

**Step 3: Create components module**

Create `fractalwonder-ui/src/components/mod.rs`:

```rust
mod test_canvas;
pub use test_canvas::TestCanvas;
```

**Step 4: Implement TestCanvas with static gradient**

Create `fractalwonder-ui/src/components/test_canvas.rs`:

```rust
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

#[component]
pub fn TestCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Render test pattern on mount and when canvas resizes
    create_effect(move |_| {
        if let Some(canvas_el) = canvas_ref.get() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();

            // Set canvas to fill viewport
            let window = web_sys::window().unwrap();
            let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height = window.inner_height().unwrap().as_f64().unwrap() as u32;

            canvas.set_width(width);
            canvas.set_height(height);

            // Render test pattern: blue-orange gradient
            render_test_pattern(&canvas, width, height);
        }
    });

    view! {
        <canvas
            node_ref=canvas_ref
            style="display: block; width: 100vw; height: 100vh;"
        />
    }
}

fn render_test_pattern(canvas: &HtmlCanvasElement, width: u32, height: u32) {
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .unchecked_into();

    // Create pixel buffer
    let pixel_count = (width * height * 4) as usize;
    let mut pixels = vec![0u8; pixel_count];

    // Generate gradient: blue (top-left) to orange (bottom-right)
    for y in 0..height {
        for x in 0..width {
            let t_x = x as f64 / width as f64;
            let t_y = y as f64 / height as f64;

            let r = (t_x * 255.0) as u8;
            let g = 128;
            let b = (t_y * 255.0) as u8;

            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = r;
            pixels[idx + 1] = g;
            pixels[idx + 2] = b;
            pixels[idx + 3] = 255; // Alpha
        }
    }

    // Put pixels on canvas
    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&pixels),
        width,
        height,
    )
    .unwrap();

    ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
}
```

**Step 5: Update lib.rs to use App module**

Modify `fractalwonder-ui/src/lib.rs`:

```rust
use leptos::*;
use wasm_bindgen::prelude::*;

mod app;
use app::App;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    leptos::mount_to_body(App);
}
```

**Step 6: Verify it builds and renders**

Run: `./scripts/run-all-checks.sh`
Expected: Success

Refresh browser: http://localhost:8080
Expected: Full-screen canvas with blue-orange gradient

**Step 7: Commit**

```bash
git add -A
git commit -m "feat(stage0): add canvas with static test pattern"
```

**Deliverable:** Full-screen canvas rendering pixels. Foundation for fractal rendering.

---

## Stage 1: BigFloat Foundation

**Goal:** Build BigFloat with comprehensive tests, core types, and transform functions. Everything uses BigFloat with explicit precision.

### Task 1.1: BigFloat Implementation with Comprehensive Tests

**What this delivers:** Fully tested BigFloat with f64/FBig enum, explicit precision enforcement, and all arithmetic operations.

**Files:**
- Modify: Root `Cargo.toml`
- Create: `fractalwonder-core/Cargo.toml`
- Create: `fractalwonder-core/src/lib.rs`
- Create: `fractalwonder-core/src/bigfloat.rs`

**Step 1: Add core crate to workspace**

Modify root `Cargo.toml`, add to workspace section:

```toml
[workspace]
members = ["fractalwonder-core", "fractalwonder-ui"]
resolver = "2"

[workspace.dependencies]
# Add these new entries:
fractalwonder-core = { path = "./fractalwonder-core" }
dashu = "0.4"
dashu-float = "0.4"

# ... keep existing dependencies
```

**Step 2: Create core crate manifest**

Create `fractalwonder-core/Cargo.toml`:

```toml
[package]
name = "fractalwonder-core"
version = "0.1.0"
edition = "2021"

[dependencies]
dashu = { workspace = true }
dashu-float = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
wasm-bindgen-test = { workspace = true }
```

**Step 3: Create core lib.rs**

Create `fractalwonder-core/src/lib.rs`:

```rust
pub mod bigfloat;

pub use bigfloat::BigFloat;
```

**Step 4: Write COMPREHENSIVE BigFloat tests**

Create `fractalwonder-core/src/bigfloat.rs` with tests FIRST:

```rust
use dashu_float::FBig;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    // === Creation Tests ===

    #[test]
    fn test_zero_with_precision() {
        let bf = BigFloat::zero(64);
        assert_eq!(bf.precision_bits(), 64);
        assert_eq!(bf.to_f64(), 0.0);

        let bf2 = BigFloat::zero(256);
        assert_eq!(bf2.precision_bits(), 256);
        assert_eq!(bf2.to_f64(), 0.0);
    }

    #[test]
    fn test_one_with_precision() {
        let bf = BigFloat::one(64);
        assert_eq!(bf.precision_bits(), 64);
        assert_eq!(bf.to_f64(), 1.0);

        let bf2 = BigFloat::one(128);
        assert_eq!(bf2.precision_bits(), 128);
        assert_eq!(bf2.to_f64(), 1.0);
    }

    #[test]
    fn test_with_precision() {
        let bf = BigFloat::with_precision(42.5, 128);
        assert_eq!(bf.precision_bits(), 128);
        assert!((bf.to_f64() - 42.5).abs() < 1e-10);

        let bf2 = BigFloat::with_precision(-3.14159, 256);
        assert_eq!(bf2.precision_bits(), 256);
        assert!((bf2.to_f64() - (-3.14159)).abs() < 1e-5);
    }

    #[test]
    fn test_f64_path_used_for_low_precision() {
        let bf = BigFloat::with_precision(2.0, 64);
        // Should use f64 internally when precision <= 64
        if let BigFloatValue::F64(_) = bf.value {
            // Correct
        } else {
            panic!("Should use f64 fast path for precision=64");
        }
    }

    #[test]
    fn test_arbitrary_path_used_for_high_precision() {
        let bf = BigFloat::with_precision(2.0, 128);
        // Should use FBig internally when precision > 64
        if let BigFloatValue::Arbitrary(_) = bf.value {
            // Correct
        } else {
            panic!("Should use FBig for precision > 64");
        }
    }

    // === Addition Tests ===

    #[test]
    fn test_add_same_precision() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(1.5, 128);
        let result = a.add(&b);
        assert_eq!(result.precision_bits(), 128);
        assert!((result.to_f64() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_preserves_max_precision() {
        let a = BigFloat::with_precision(2.5, 64);
        let b = BigFloat::with_precision(1.5, 256);
        let result = a.add(&b);
        assert_eq!(result.precision_bits(), 256); // Max precision
        assert!((result.to_f64() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_negative_numbers() {
        let a = BigFloat::with_precision(-5.0, 128);
        let b = BigFloat::with_precision(3.0, 128);
        let result = a.add(&b);
        assert!((result.to_f64() - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_add_with_zero() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::zero(128);
        let result = a.add(&b);
        assert!((result.to_f64() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_large_numbers() {
        let a = BigFloat::with_precision(1e100, 256);
        let b = BigFloat::with_precision(1e100, 256);
        let result = a.add(&b);
        assert!((result.to_f64() - 2e100).abs() / 2e100 < 1e-10);
    }

    #[test]
    fn test_add_very_small_numbers() {
        let a = BigFloat::with_precision(1e-100, 256);
        let b = BigFloat::with_precision(1e-100, 256);
        let result = a.add(&b);
        assert!((result.to_f64() - 2e-100).abs() / 2e-100 < 1e-10);
    }

    // === Subtraction Tests ===

    #[test]
    fn test_sub_same_precision() {
        let a = BigFloat::with_precision(5.0, 128);
        let b = BigFloat::with_precision(3.0, 128);
        let result = a.sub(&b);
        assert!((result.to_f64() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_sub_preserves_max_precision() {
        let a = BigFloat::with_precision(5.0, 64);
        let b = BigFloat::with_precision(3.0, 256);
        let result = a.sub(&b);
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn test_sub_negative_result() {
        let a = BigFloat::with_precision(3.0, 128);
        let b = BigFloat::with_precision(5.0, 128);
        let result = a.sub(&b);
        assert!((result.to_f64() - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_sub_with_zero() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::zero(128);
        let result = a.sub(&b);
        assert!((result.to_f64() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_sub_from_zero() {
        let a = BigFloat::zero(128);
        let b = BigFloat::with_precision(42.0, 128);
        let result = a.sub(&b);
        assert!((result.to_f64() - (-42.0)).abs() < 1e-10);
    }

    // === Multiplication Tests ===

    #[test]
    fn test_mul_same_precision() {
        let a = BigFloat::with_precision(3.0, 128);
        let b = BigFloat::with_precision(4.0, 128);
        let result = a.mul(&b);
        assert!((result.to_f64() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_mul_preserves_max_precision() {
        let a = BigFloat::with_precision(3.0, 64);
        let b = BigFloat::with_precision(4.0, 256);
        let result = a.mul(&b);
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn test_mul_with_zero() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::zero(128);
        let result = a.mul(&b);
        assert_eq!(result.to_f64(), 0.0);
    }

    #[test]
    fn test_mul_with_one() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::one(128);
        let result = a.mul(&b);
        assert!((result.to_f64() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_mul_negative_numbers() {
        let a = BigFloat::with_precision(-3.0, 128);
        let b = BigFloat::with_precision(4.0, 128);
        let result = a.mul(&b);
        assert!((result.to_f64() - (-12.0)).abs() < 1e-10);

        let c = BigFloat::with_precision(-3.0, 128);
        let d = BigFloat::with_precision(-4.0, 128);
        let result2 = c.mul(&d);
        assert!((result2.to_f64() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_mul_large_numbers() {
        let a = BigFloat::with_precision(1e50, 256);
        let b = BigFloat::with_precision(1e50, 256);
        let result = a.mul(&b);
        assert!((result.to_f64() - 1e100).abs() / 1e100 < 1e-10);
    }

    // === Division Tests ===

    #[test]
    fn test_div_same_precision() {
        let a = BigFloat::with_precision(10.0, 128);
        let b = BigFloat::with_precision(2.0, 128);
        let result = a.div(&b);
        assert!((result.to_f64() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_div_preserves_max_precision() {
        let a = BigFloat::with_precision(10.0, 64);
        let b = BigFloat::with_precision(2.0, 256);
        let result = a.div(&b);
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn test_div_by_one() {
        let a = BigFloat::with_precision(42.0, 128);
        let b = BigFloat::one(128);
        let result = a.div(&b);
        assert!((result.to_f64() - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_div_fractional_result() {
        let a = BigFloat::with_precision(1.0, 128);
        let b = BigFloat::with_precision(3.0, 128);
        let result = a.div(&b);
        assert!((result.to_f64() - 0.333333).abs() < 1e-5);
    }

    #[test]
    fn test_div_negative_numbers() {
        let a = BigFloat::with_precision(-10.0, 128);
        let b = BigFloat::with_precision(2.0, 128);
        let result = a.div(&b);
        assert!((result.to_f64() - (-5.0)).abs() < 1e-10);
    }

    #[test]
    fn test_div_by_large_number() {
        let a = BigFloat::with_precision(1.0, 256);
        let b = BigFloat::with_precision(1e100, 256);
        let result = a.div(&b);
        assert!((result.to_f64() - 1e-100).abs() / 1e-100 < 1e-10);
    }

    // === Square Root Tests ===

    #[test]
    fn test_sqrt_perfect_square() {
        let a = BigFloat::with_precision(16.0, 128);
        let result = a.sqrt();
        assert!((result.to_f64() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt_preserves_precision() {
        let a = BigFloat::with_precision(2.0, 256);
        let result = a.sqrt();
        assert_eq!(result.precision_bits(), 256);
    }

    #[test]
    fn test_sqrt_non_perfect_square() {
        let a = BigFloat::with_precision(2.0, 128);
        let result = a.sqrt();
        assert!((result.to_f64() - 1.414213).abs() < 1e-5);
    }

    #[test]
    fn test_sqrt_of_one() {
        let a = BigFloat::one(128);
        let result = a.sqrt();
        assert!((result.to_f64() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt_of_zero() {
        let a = BigFloat::zero(128);
        let result = a.sqrt();
        assert_eq!(result.to_f64(), 0.0);
    }

    #[test]
    fn test_sqrt_large_number() {
        let a = BigFloat::with_precision(1e100, 256);
        let result = a.sqrt();
        assert!((result.to_f64() - 1e50).abs() / 1e50 < 1e-10);
    }

    // === Comparison Tests ===

    #[test]
    fn test_partial_eq_same_value() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(2.5, 128);
        assert_eq!(a, b);
    }

    #[test]
    fn test_partial_eq_different_precision_same_value() {
        let a = BigFloat::with_precision(2.5, 64);
        let b = BigFloat::with_precision(2.5, 256);
        assert_eq!(a, b); // Values equal, precision doesn't affect equality
    }

    #[test]
    fn test_partial_eq_different_value() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(3.5, 128);
        assert_ne!(a, b);
    }

    #[test]
    fn test_partial_ord_less_than() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(3.5, 128);
        assert!(a < b);
    }

    #[test]
    fn test_partial_ord_greater_than() {
        let a = BigFloat::with_precision(3.5, 128);
        let b = BigFloat::with_precision(2.5, 128);
        assert!(a > b);
    }

    #[test]
    fn test_partial_ord_equal() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(2.5, 128);
        assert!(!(a < b));
        assert!(!(a > b));
    }

    #[test]
    fn test_partial_ord_negative_numbers() {
        let a = BigFloat::with_precision(-2.5, 128);
        let b = BigFloat::with_precision(-1.5, 128);
        assert!(a < b); // -2.5 < -1.5
    }

    #[test]
    fn test_partial_ord_zero() {
        let a = BigFloat::zero(128);
        let b = BigFloat::with_precision(1.0, 128);
        assert!(a < b);

        let c = BigFloat::with_precision(-1.0, 128);
        let d = BigFloat::zero(128);
        assert!(c < d);
    }

    // === Serialization Tests ===

    #[test]
    fn test_serialization_roundtrip_f64_precision() {
        let original = BigFloat::with_precision(3.14159, 64);
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: BigFloat = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(restored.precision_bits(), 64);
        assert!((restored.to_f64() - 3.14159).abs() < 1e-5);
    }

    #[test]
    fn test_serialization_roundtrip_high_precision() {
        let original = BigFloat::with_precision(3.14159, 256);
        let json = serde_json::to_string(&original).expect("serialize failed");
        let restored: BigFloat = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(restored.precision_bits(), 256);
        assert!((restored.to_f64() - 3.14159).abs() < 1e-5);
    }

    #[test]
    fn test_serialization_preserves_zero() {
        let original = BigFloat::zero(128);
        let json = serde_json::to_string(&original).unwrap();
        let restored: BigFloat = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.to_f64(), 0.0);
        assert_eq!(restored.precision_bits(), 128);
    }

    #[test]
    fn test_serialization_preserves_negative() {
        let original = BigFloat::with_precision(-42.5, 256);
        let json = serde_json::to_string(&original).unwrap();
        let restored: BigFloat = serde_json::from_str(&json).unwrap();

        assert!((restored.to_f64() - (-42.5)).abs() < 1e-10);
        assert_eq!(restored.precision_bits(), 256);
    }

    // === Complex Expression Tests ===

    #[test]
    fn test_complex_expression_with_all_operations() {
        // (2 + 3) * 4 / 2 - 1 = 9
        let two = BigFloat::with_precision(2.0, 128);
        let three = BigFloat::with_precision(3.0, 128);
        let four = BigFloat::with_precision(4.0, 128);
        let one = BigFloat::one(128);

        let result = two.add(&three).mul(&four).div(&two).sub(&one);
        assert!((result.to_f64() - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_mandelbrot_iteration_formula() {
        // Test z = z^2 + c where z=(2,1), c=(0.5, 0.3)
        // z^2 = (2^2 - 1^2, 2*2*1) = (3, 4)
        // z^2 + c = (3.5, 4.3)
        let z_real = BigFloat::with_precision(2.0, 128);
        let z_imag = BigFloat::with_precision(1.0, 128);
        let c_real = BigFloat::with_precision(0.5, 128);
        let c_imag = BigFloat::with_precision(0.3, 128);

        let z_real_sq = z_real.mul(&z_real);
        let z_imag_sq = z_imag.mul(&z_imag);
        let new_real = z_real_sq.sub(&z_imag_sq).add(&c_real);

        let two = BigFloat::with_precision(2.0, 128);
        let new_imag = two.mul(&z_real).mul(&z_imag).add(&c_imag);

        assert!((new_real.to_f64() - 3.5).abs() < 1e-10);
        assert!((new_imag.to_f64() - 4.3).abs() < 1e-10);
    }

    // === Edge Case Tests ===

    #[test]
    fn test_very_large_precision() {
        let bf = BigFloat::with_precision(1.0, 1024);
        assert_eq!(bf.precision_bits(), 1024);
        assert_eq!(bf.to_f64(), 1.0);
    }

    #[test]
    fn test_operations_maintain_finite_values() {
        let a = BigFloat::with_precision(1e100, 256);
        let b = BigFloat::with_precision(1e-100, 256);

        let product = a.mul(&b);
        assert!(product.to_f64().is_finite());

        let quotient = a.div(&b);
        assert!(quotient.to_f64().is_finite());
    }

    #[test]
    fn test_chain_of_operations_maintains_precision() {
        let start = BigFloat::with_precision(1.0, 256);
        let two = BigFloat::with_precision(2.0, 256);

        let result = start.add(&two).mul(&two).div(&two).sub(&two);

        // Should get back to 1.0
        assert!((result.to_f64() - 1.0).abs() < 1e-10);
        assert_eq!(result.precision_bits(), 256);
    }
}
```

**Step 5: Verify tests FAIL**

Run: `cargo test -p fractalwonder-core --lib bigfloat`
Expected: FAIL - BigFloat types not defined

**Step 6: Implement BigFloat** (complete implementation to pass all tests)

Add before the test module in `fractalwonder-core/src/bigfloat.rs`:

```rust
/// Arbitrary precision floating point with explicit precision enforcement
///
/// Uses f64 internally when precision_bits <= 64, FBig otherwise.
/// This optimization is completely transparent to external code.
#[derive(Clone, Debug)]
pub struct BigFloat {
    value: BigFloatValue,
    precision_bits: usize,
}

#[derive(Clone, Debug)]
pub enum BigFloatValue {
    F64(f64),
    Arbitrary(FBig),
}

impl BigFloat {
    /// Create BigFloat from f64 with explicit precision
    ///
    /// NO DEFAULT - precision must always be specified
    pub fn with_precision(val: f64, precision_bits: usize) -> Self {
        let value = if precision_bits <= 64 {
            BigFloatValue::F64(val)
        } else {
            BigFloatValue::Arbitrary(FBig::try_from(val).unwrap_or(FBig::ZERO))
        };

        Self {
            value,
            precision_bits,
        }
    }

    /// Create zero with explicit precision
    pub fn zero(precision_bits: usize) -> Self {
        Self::with_precision(0.0, precision_bits)
    }

    /// Create one with explicit precision
    pub fn one(precision_bits: usize) -> Self {
        Self::with_precision(1.0, precision_bits)
    }

    /// Get precision in bits
    pub fn precision_bits(&self) -> usize {
        self.precision_bits
    }

    /// Convert to f64 (for display/colorization only)
    /// May lose precision for values requiring > 64 bits
    pub fn to_f64(&self) -> f64 {
        match &self.value {
            BigFloatValue::F64(v) => *v,
            BigFloatValue::Arbitrary(v) => v.to_f64().value(),
        }
    }

    /// Add two BigFloats, preserving max precision
    pub fn add(&self, other: &Self) -> Self {
        let result_precision = self.precision_bits.max(other.precision_bits);

        let result_value = match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
                BigFloatValue::F64(a + b)
            }
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                BigFloatValue::Arbitrary(&a_big + &b_big)
            }
        };

        Self {
            value: result_value,
            precision_bits: result_precision,
        }
    }

    /// Subtract two BigFloats, preserving max precision
    pub fn sub(&self, other: &Self) -> Self {
        let result_precision = self.precision_bits.max(other.precision_bits);

        let result_value = match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
                BigFloatValue::F64(a - b)
            }
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                BigFloatValue::Arbitrary(&a_big - &b_big)
            }
        };

        Self {
            value: result_value,
            precision_bits: result_precision,
        }
    }

    /// Multiply two BigFloats, preserving max precision
    pub fn mul(&self, other: &Self) -> Self {
        let result_precision = self.precision_bits.max(other.precision_bits);

        let result_value = match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
                BigFloatValue::F64(a * b)
            }
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                BigFloatValue::Arbitrary(&a_big * &b_big)
            }
        };

        Self {
            value: result_value,
            precision_bits: result_precision,
        }
    }

    /// Divide two BigFloats, preserving max precision
    pub fn div(&self, other: &Self) -> Self {
        let result_precision = self.precision_bits.max(other.precision_bits);

        let result_value = match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
                BigFloatValue::F64(a / b)
            }
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                BigFloatValue::Arbitrary(&a_big / &b_big)
            }
        };

        Self {
            value: result_value,
            precision_bits: result_precision,
        }
    }

    /// Square root, preserving precision
    pub fn sqrt(&self) -> Self {
        let result_value = match &self.value {
            BigFloatValue::F64(v) if self.precision_bits <= 64 => {
                BigFloatValue::F64(v.sqrt())
            }
            _ => {
                let v_big = self.to_fbig();
                BigFloatValue::Arbitrary(v_big.sqrt())
            }
        };

        Self {
            value: result_value,
            precision_bits: self.precision_bits,
        }
    }

    /// Convert to FBig for arbitrary precision operations
    fn to_fbig(&self) -> FBig {
        match &self.value {
            BigFloatValue::F64(v) => FBig::try_from(*v).unwrap_or(FBig::ZERO),
            BigFloatValue::Arbitrary(v) => v.clone(),
        }
    }
}

impl PartialEq for BigFloat {
    fn eq(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) => a == b,
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                a_big == b_big
            }
        }
    }
}

impl PartialOrd for BigFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (&self.value, &other.value) {
            (BigFloatValue::F64(a), BigFloatValue::F64(b)) => a.partial_cmp(b),
            _ => {
                let a_big = self.to_fbig();
                let b_big = other.to_fbig();
                a_big.partial_cmp(&b_big)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct BigFloatSerde {
    value: String,
    precision_bits: usize,
}

impl Serialize for BigFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value_str = match &self.value {
            BigFloatValue::F64(v) => v.to_string(),
            BigFloatValue::Arbitrary(v) => v.to_string(),
        };

        let serde = BigFloatSerde {
            value: value_str,
            precision_bits: self.precision_bits,
        };

        serde.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BigFloat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serde = BigFloatSerde::deserialize(deserializer)?;

        let value = if serde.precision_bits <= 64 {
            let f = serde
                .value
                .parse::<f64>()
                .map_err(|e| serde::de::Error::custom(format!("Failed to parse f64: {}", e)))?;
            BigFloatValue::F64(f)
        } else {
            let fbig = serde
                .value
                .parse::<FBig>()
                .map_err(|e| serde::de::Error::custom(format!("Failed to parse FBig: {}", e)))?;
            BigFloatValue::Arbitrary(fbig)
        };

        Ok(BigFloat {
            value,
            precision_bits: serde.precision_bits,
        })
    }
}
```

**Step 7: Verify ALL tests pass**

Run: `cargo test -p fractalwonder-core --lib bigfloat -- --nocapture`
Expected: ALL TESTS PASS (50+ tests)

**Step 8: Run clippy**

Run: `cargo clippy -p fractalwonder-core -- -D warnings`
Expected: No warnings

**Step 9: Commit**

```bash
git add -A
git commit -m "feat(core): add BigFloat with comprehensive test suite"
```

**Deliverable:** Fully tested BigFloat implementation. Foundation for all fractal-space arithmetic. Web app still shows test pattern (unchanged).

---

### Task 1.2: Viewport and PixelRect Types

**What this delivers:** Core coordinate types with BigFloat support. Viewport uses BigFloat for center and zoom. PixelRect uses u32 for pixel coordinates.

**Files:**
- Create: `fractalwonder-core/src/viewport.rs`
- Create: `fractalwonder-core/src/pixel_rect.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write Viewport tests FIRST**

Create `fractalwonder-core/src/viewport.rs`:

```rust
use crate::BigFloat;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_creation() {
        let center = (
            BigFloat::with_precision(0.0, 128),
            BigFloat::with_precision(0.0, 128),
        );
        let zoom = BigFloat::with_precision(1.0, 128);

        let viewport = Viewport { center, zoom };

        assert_eq!(viewport.center.0.to_f64(), 0.0);
        assert_eq!(viewport.center.1.to_f64(), 0.0);
        assert_eq!(viewport.zoom.to_f64(), 1.0);
    }

    #[test]
    fn test_viewport_extreme_zoom() {
        let center = (
            BigFloat::with_precision(-0.5, 256),
            BigFloat::with_precision(0.3, 256),
        );
        // Zoom level beyond f64 range (10^500)
        let zoom = BigFloat::with_precision(1e100, 256)
            .mul(&BigFloat::with_precision(1e100, 256))
            .mul(&BigFloat::with_precision(1e100, 256))
            .mul(&BigFloat::with_precision(1e100, 256))
            .mul(&BigFloat::with_precision(1e100, 256));

        let viewport = Viewport { center, zoom };

        assert_eq!(viewport.center.0.to_f64(), -0.5);
        assert_eq!(viewport.zoom.precision_bits(), 256);
    }

    #[test]
    fn test_viewport_serialization_roundtrip() {
        let center = (
            BigFloat::with_precision(-0.5, 256),
            BigFloat::with_precision(0.3, 256),
        );
        let zoom = BigFloat::with_precision(1e50, 256);

        let original = Viewport { center, zoom };
        let json = serde_json::to_string(&original).unwrap();
        let restored: Viewport = serde_json::from_str(&json).unwrap();

        assert!((restored.center.0.to_f64() - (-0.5)).abs() < 1e-10);
        assert!((restored.center.1.to_f64() - 0.3).abs() < 1e-10);
        assert_eq!(restored.zoom.precision_bits(), 256);
    }

    #[test]
    fn test_viewport_with_different_precisions() {
        let center = (
            BigFloat::with_precision(0.0, 64),
            BigFloat::with_precision(0.0, 64),
        );
        let zoom = BigFloat::with_precision(1.0, 256);

        let viewport = Viewport { center, zoom };

        assert_eq!(viewport.center.0.precision_bits(), 64);
        assert_eq!(viewport.zoom.precision_bits(), 256);
    }
}
```

**Step 2: Write PixelRect tests FIRST**

Create `fractalwonder-core/src/pixel_rect.rs`:

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_rect_creation() {
        let rect = PixelRect {
            x: 10,
            y: 20,
            width: 100,
            height: 200,
        };

        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 200);
    }

    #[test]
    fn test_pixel_rect_area() {
        let rect = PixelRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        assert_eq!(rect.area(), 1920 * 1080);
    }

    #[test]
    fn test_pixel_rect_contains_point() {
        let rect = PixelRect {
            x: 10,
            y: 20,
            width: 100,
            height: 50,
        };

        assert!(rect.contains(50, 40));
        assert!(rect.contains(10, 20)); // Top-left corner
        assert!(rect.contains(109, 69)); // Bottom-right corner
        assert!(!rect.contains(110, 70)); // Just outside
        assert!(!rect.contains(9, 20)); // Just left
        assert!(!rect.contains(50, 19)); // Just above
    }

    #[test]
    fn test_pixel_rect_serialization_roundtrip() {
        let original = PixelRect {
            x: 100,
            y: 200,
            width: 640,
            height: 480,
        };

        let json = serde_json::to_string(&original).unwrap();
        let restored: PixelRect = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, original);
    }
}
```

**Step 3: Verify tests FAIL**

Run: `cargo test -p fractalwonder-core`
Expected: FAIL - Types not defined

**Step 4: Implement Viewport**

Add to `fractalwonder-core/src/viewport.rs` (before test module):

```rust
/// Viewport in fractal space with BigFloat precision
///
/// - `center`: Fractal-space center coordinates (BigFloat for extreme precision)
/// - `zoom`: Zoom level (BigFloat because zoom can exceed 10^308)
///
/// Note: At zoom level 10^2000, we're viewing a region of width ~10^-2000
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Viewport {
    pub center: (BigFloat, BigFloat),
    pub zoom: BigFloat,
}

impl Viewport {
    /// Create new viewport with explicit precision
    pub fn new(center_x: f64, center_y: f64, zoom: f64, precision_bits: usize) -> Self {
        Self {
            center: (
                BigFloat::with_precision(center_x, precision_bits),
                BigFloat::with_precision(center_y, precision_bits),
            ),
            zoom: BigFloat::with_precision(zoom, precision_bits),
        }
    }

    /// Default Mandelbrot view: center at origin, zoom = 1.0
    pub fn default_mandelbrot(precision_bits: usize) -> Self {
        Self::new(0.0, 0.0, 1.0, precision_bits)
    }
}
```

**Step 5: Implement PixelRect**

Add to `fractalwonder-core/src/pixel_rect.rs` (before test module):

```rust
/// Rectangle in pixel space (always u32 coordinates)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    /// Create new pixel rectangle
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Calculate area in pixels
    pub fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Check if point is inside rectangle
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Create full-canvas rectangle
    pub fn full_canvas(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}
```

**Step 6: Update lib.rs**

Modify `fractalwonder-core/src/lib.rs`:

```rust
pub mod bigfloat;
pub mod pixel_rect;
pub mod viewport;

pub use bigfloat::BigFloat;
pub use pixel_rect::PixelRect;
pub use viewport::Viewport;
```

**Step 7: Verify tests pass**

Run: `cargo test -p fractalwonder-core -- --nocapture`
Expected: ALL TESTS PASS

**Step 8: Run clippy**

Run: `cargo clippy -p fractalwonder-core -- -D warnings`
Expected: No warnings

**Step 9: Commit**

```bash
git add -A
git commit -m "feat(core): add Viewport and PixelRect types with BigFloat support"
```

**Deliverable:** Core coordinate types ready for transforms. Web app still shows test pattern (unchanged).

---

### Task 1.3: Coordinate Transform Functions with BigFloat

**What this delivers:** Functions to convert pixel coordinates → fractal coordinates and apply viewport transformations using BigFloat arithmetic throughout.

**Files:**
- Create: `fractalwonder-core/src/transforms.rs`
- Modify: `fractalwonder-core/src/lib.rs`

**Step 1: Write transform tests FIRST**

Create `fractalwonder-core/src/transforms.rs`:

```rust
use crate::{BigFloat, PixelRect, Viewport};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_to_fractal_center() {
        // Canvas 800x600, viewport centered at (0,0) zoom=1
        let viewport = Viewport::new(0.0, 0.0, 1.0, 128);
        let canvas_size = (800, 600);

        // Center pixel (400, 300) should map to fractal center (0, 0)
        let (fx, fy) = pixel_to_fractal(400.0, 300.0, &viewport, canvas_size, 128);

        assert!((fx.to_f64() - 0.0).abs() < 1e-10);
        assert!((fy.to_f64() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pixel_to_fractal_corners() {
        // Viewport: center=(0,0), zoom=1, canvas=800x600
        // Visible region width = 4.0 (Mandelbrot default)
        // height = 4.0 * 600/800 = 3.0
        let viewport = Viewport::new(0.0, 0.0, 1.0, 128);
        let canvas_size = (800, 600);

        // Top-left pixel (0, 0)
        let (fx, fy) = pixel_to_fractal(0.0, 0.0, &viewport, canvas_size, 128);
        assert!((fx.to_f64() - (-2.0)).abs() < 1e-5); // Left edge at -2
        assert!((fy.to_f64() - (-1.5)).abs() < 1e-5); // Top edge at -1.5

        // Bottom-right pixel (799, 599)
        let (fx, fy) = pixel_to_fractal(799.0, 599.0, &viewport, canvas_size, 128);
        assert!((fx.to_f64() - 2.0).abs() < 1e-2);
        assert!((fy.to_f64() - 1.5).abs() < 1e-2);
    }

    #[test]
    fn test_pixel_to_fractal_with_zoom() {
        // Zoom=2 means viewing half the region
        let viewport = Viewport::new(0.0, 0.0, 2.0, 128);
        let canvas_size = (800, 600);

        // Top-left pixel should map to smaller fractal region
        let (fx, fy) = pixel_to_fractal(0.0, 0.0, &viewport, canvas_size, 128);
        assert!((fx.to_f64() - (-1.0)).abs() < 1e-5); // Half the default range
        assert!((fy.to_f64() - (-0.75)).abs() < 1e-5);
    }

    #[test]
    fn test_pixel_to_fractal_with_offset_center() {
        // Viewport centered at (-0.5, 0.3)
        let viewport = Viewport::new(-0.5, 0.3, 1.0, 128);
        let canvas_size = (800, 600);

        // Center pixel should map to viewport center
        let (fx, fy) = pixel_to_fractal(400.0, 300.0, &viewport, canvas_size, 128);
        assert!((fx.to_f64() - (-0.5)).abs() < 1e-10);
        assert!((fy.to_f64() - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_pixel_to_fractal_preserves_precision() {
        let viewport = Viewport::new(0.0, 0.0, 1.0, 256);
        let canvas_size = (800, 600);

        let (fx, fy) = pixel_to_fractal(100.0, 200.0, &viewport, canvas_size, 256);

        assert_eq!(fx.precision_bits(), 256);
        assert_eq!(fy.precision_bits(), 256);
    }

    #[test]
    fn test_fractal_to_pixel_center() {
        let viewport = Viewport::new(0.0, 0.0, 1.0, 128);
        let canvas_size = (800, 600);

        let fx = BigFloat::with_precision(0.0, 128);
        let fy = BigFloat::with_precision(0.0, 128);

        let (px, py) = fractal_to_pixel(&fx, &fy, &viewport, canvas_size);

        assert!((px - 400.0).abs() < 1e-5);
        assert!((py - 300.0).abs() < 1e-5);
    }

    #[test]
    fn test_fractal_to_pixel_with_zoom() {
        let viewport = Viewport::new(0.0, 0.0, 2.0, 128);
        let canvas_size = (800, 600);

        let fx = BigFloat::with_precision(-1.0, 128);
        let fy = BigFloat::with_precision(-0.75, 128);

        let (px, py) = fractal_to_pixel(&fx, &fy, &viewport, canvas_size);

        assert!((px - 0.0).abs() < 1e-2);
        assert!((py - 0.0).abs() < 1e-2);
    }

    #[test]
    fn test_apply_transform_zoom_in() {
        let viewport = Viewport::new(0.0, 0.0, 1.0, 128);
        let transform = TransformResult {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom_factor: 2.0, // Zoom in 2x
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size, 128);

        // Zoom should double
        assert!((new_vp.zoom.to_f64() - 2.0).abs() < 1e-10);
        // Center unchanged
        assert!((new_vp.center.0.to_f64() - 0.0).abs() < 1e-10);
        assert!((new_vp.center.1.to_f64() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply_transform_pan() {
        let viewport = Viewport::new(0.0, 0.0, 1.0, 128);
        let transform = TransformResult {
            offset_x: 100.0, // Pan right 100 pixels
            offset_y: -50.0, // Pan up 50 pixels
            zoom_factor: 1.0,
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size, 128);

        // Zoom unchanged
        assert!((new_vp.zoom.to_f64() - 1.0).abs() < 1e-10);

        // Center should shift (pan right = positive x in fractal space)
        // pixel_offset / canvas_width * visible_width
        // 100 / 800 * 4.0 = 0.5
        assert!((new_vp.center.0.to_f64() - 0.5).abs() < 1e-5);
        // -50 / 600 * 3.0 = 0.25
        assert!((new_vp.center.1.to_f64() - 0.25).abs() < 1e-5);
    }

    #[test]
    fn test_apply_transform_zoom_and_pan() {
        let viewport = Viewport::new(0.0, 0.0, 1.0, 128);
        let transform = TransformResult {
            offset_x: 100.0,
            offset_y: 50.0,
            zoom_factor: 2.0,
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size, 128);

        assert!((new_vp.zoom.to_f64() - 2.0).abs() < 1e-10);
        assert!(new_vp.center.0.to_f64() > 0.0); // Panned right
        assert!(new_vp.center.1.to_f64() > 0.0); // Panned down
    }

    #[test]
    fn test_apply_transform_preserves_precision() {
        let viewport = Viewport::new(0.0, 0.0, 1.0, 256);
        let transform = TransformResult {
            offset_x: 10.0,
            offset_y: 10.0,
            zoom_factor: 1.5,
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size, 256);

        assert_eq!(new_vp.center.0.precision_bits(), 256);
        assert_eq!(new_vp.center.1.precision_bits(), 256);
        assert_eq!(new_vp.zoom.precision_bits(), 256);
    }

    #[test]
    fn test_apply_transform_extreme_zoom() {
        // Start at zoom 10^100
        let initial_zoom = BigFloat::with_precision(1e100, 256);
        let viewport = Viewport {
            center: (
                BigFloat::with_precision(0.0, 256),
                BigFloat::with_precision(0.0, 256),
            ),
            zoom: initial_zoom.clone(),
        };

        let transform = TransformResult {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom_factor: 2.0,
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size, 256);

        // Zoom should be 2 * 10^100
        let expected_zoom = initial_zoom.mul(&BigFloat::with_precision(2.0, 256));
        assert!((new_vp.zoom.to_f64() - expected_zoom.to_f64()).abs() / expected_zoom.to_f64() < 1e-10);
    }

    #[test]
    fn test_roundtrip_pixel_fractal_pixel() {
        let viewport = Viewport::new(-0.5, 0.3, 2.5, 128);
        let canvas_size = (1920, 1080);

        let original_px = 1234.0;
        let original_py = 567.0;

        let (fx, fy) = pixel_to_fractal(original_px, original_py, &viewport, canvas_size, 128);
        let (px, py) = fractal_to_pixel(&fx, &fy, &viewport, canvas_size);

        assert!((px - original_px).abs() < 1e-5);
        assert!((py - original_py).abs() < 1e-5);
    }
}
```

**Step 2: Verify tests FAIL**

Run: `cargo test -p fractalwonder-core transforms`
Expected: FAIL - Functions not defined

**Step 3: Implement transform functions**

Add to `fractalwonder-core/src/transforms.rs` (before test module):

```rust
/// Result of user interaction (from use_canvas_interaction hook)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransformResult {
    pub offset_x: f64,    // Pixel offset (horizontal pan)
    pub offset_y: f64,    // Pixel offset (vertical pan)
    pub zoom_factor: f64, // RELATIVE zoom (2.0 = zoom in 2x from current)
}

/// Convert pixel coordinates to fractal coordinates
///
/// Uses BigFloat arithmetic throughout to preserve precision
pub fn pixel_to_fractal(
    pixel_x: f64,
    pixel_y: f64,
    viewport: &Viewport,
    canvas_size: (u32, u32),
    precision_bits: usize,
) -> (BigFloat, BigFloat) {
    let (canvas_width, canvas_height) = canvas_size;

    // Mandelbrot default visible width at zoom=1
    let base_width = BigFloat::with_precision(4.0, precision_bits);

    // visible_width = base_width / zoom
    let visible_width = base_width.div(&viewport.zoom);

    // visible_height = visible_width * (canvas_height / canvas_width)
    let aspect = BigFloat::with_precision(canvas_height as f64, precision_bits)
        .div(&BigFloat::with_precision(canvas_width as f64, precision_bits));
    let visible_height = visible_width.mul(&aspect);

    // Normalized pixel coordinates [-0.5, 0.5]
    let norm_x = BigFloat::with_precision(pixel_x, precision_bits)
        .div(&BigFloat::with_precision(canvas_width as f64, precision_bits))
        .sub(&BigFloat::with_precision(0.5, precision_bits));

    let norm_y = BigFloat::with_precision(pixel_y, precision_bits)
        .div(&BigFloat::with_precision(canvas_height as f64, precision_bits))
        .sub(&BigFloat::with_precision(0.5, precision_bits));

    // fractal_x = center_x + norm_x * visible_width
    let fractal_x = viewport.center.0.add(&norm_x.mul(&visible_width));

    // fractal_y = center_y + norm_y * visible_height
    let fractal_y = viewport.center.1.add(&norm_y.mul(&visible_height));

    (fractal_x, fractal_y)
}

/// Convert fractal coordinates to pixel coordinates
///
/// Note: This may lose precision when converting to f64 for pixel display
pub fn fractal_to_pixel(
    fractal_x: &BigFloat,
    fractal_y: &BigFloat,
    viewport: &Viewport,
    canvas_size: (u32, u32),
) -> (f64, f64) {
    let (canvas_width, canvas_height) = canvas_size;
    let precision = fractal_x.precision_bits();

    let base_width = BigFloat::with_precision(4.0, precision);
    let visible_width = base_width.div(&viewport.zoom);

    let aspect = BigFloat::with_precision(canvas_height as f64, precision)
        .div(&BigFloat::with_precision(canvas_width as f64, precision));
    let visible_height = visible_width.mul(&aspect);

    // norm_x = (fractal_x - center_x) / visible_width
    let norm_x = fractal_x.sub(&viewport.center.0).div(&visible_width);

    // norm_y = (fractal_y - center_y) / visible_height
    let norm_y = fractal_y.sub(&viewport.center.1).div(&visible_height);

    // pixel_x = (norm_x + 0.5) * canvas_width
    let pixel_x = (norm_x.to_f64() + 0.5) * canvas_width as f64;

    // pixel_y = (norm_y + 0.5) * canvas_height
    let pixel_y = (norm_y.to_f64() + 0.5) * canvas_height as f64;

    (pixel_x, pixel_y)
}

/// Apply user interaction transform to viewport
///
/// TransformResult contains RELATIVE zoom_factor (2.0 = zoom in 2x from current)
/// Uses BigFloat arithmetic to preserve precision at extreme zoom levels
pub fn apply_pixel_transform_to_viewport(
    viewport: &Viewport,
    transform: &TransformResult,
    canvas_size: (u32, u32),
    precision_bits: usize,
) -> Viewport {
    // Calculate new zoom: new_zoom = old_zoom * zoom_factor
    let zoom_factor_bf = BigFloat::with_precision(transform.zoom_factor, precision_bits);
    let new_zoom = viewport.zoom.mul(&zoom_factor_bf);

    // Calculate new center from pixel offsets
    // offset represents how much we panned in pixel space
    let base_width = BigFloat::with_precision(4.0, precision_bits);
    let visible_width = base_width.div(&viewport.zoom);

    let aspect = BigFloat::with_precision(canvas_size.1 as f64, precision_bits)
        .div(&BigFloat::with_precision(canvas_size.0 as f64, precision_bits));
    let visible_height = visible_width.mul(&aspect);

    // Convert pixel offset to fractal offset
    let offset_x_norm = BigFloat::with_precision(transform.offset_x, precision_bits)
        .div(&BigFloat::with_precision(canvas_size.0 as f64, precision_bits));
    let offset_y_norm = BigFloat::with_precision(transform.offset_y, precision_bits)
        .div(&BigFloat::with_precision(canvas_size.1 as f64, precision_bits));

    let dx = offset_x_norm.mul(&visible_width);
    let dy = offset_y_norm.mul(&visible_height);

    let new_center = (viewport.center.0.add(&dx), viewport.center.1.add(&dy));

    Viewport {
        center: new_center,
        zoom: new_zoom,
    }
}
```

**Step 4: Update lib.rs**

Modify `fractalwonder-core/src/lib.rs`:

```rust
pub mod bigfloat;
pub mod pixel_rect;
pub mod transforms;
pub mod viewport;

pub use bigfloat::BigFloat;
pub use pixel_rect::PixelRect;
pub use transforms::{apply_pixel_transform_to_viewport, fractal_to_pixel, pixel_to_fractal, TransformResult};
pub use viewport::Viewport;
```

**Step 5: Verify tests pass**

Run: `cargo test -p fractalwonder-core -- --nocapture`
Expected: ALL TESTS PASS (including new transform tests)

**Step 6: Run clippy**

Run: `cargo clippy -p fractalwonder-core -- -D warnings`
Expected: No warnings

**Step 7: Format code**

Run: `cargo fmt --all`
Expected: Success

**Step 8: Commit**

```bash
git add -A
git commit -m "feat(core): add coordinate transform functions with BigFloat"
```

**Deliverable:** Complete BigFloat foundation with transforms. Ready for interaction. Web app still shows test pattern (unchanged).

---

## Stage 2: Add Interaction

**Goal:** Make canvas interactive using use_canvas_interaction hook. Pan/zoom updates viewport state and logs transform details. Still rendering test pattern (no Mandelbrot yet).

### Task 2.1: Copy use_canvas_interaction Hook

**What this delivers:** Working interaction hook in new codebase. Can pan/zoom canvas, see transform logs in console.

**Files:**
- Create: `fractalwonder-ui/src/hooks/mod.rs`
- Create: `fractalwonder-ui/src/hooks/use_canvas_interaction.rs`
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Create hooks directory structure**

Create `fractalwonder-ui/src/hooks/mod.rs`:

```rust
mod use_canvas_interaction;

pub use use_canvas_interaction::use_canvas_interaction;
```

**Step 2: Copy hook from archive**

```bash
cp _archive/fractalwonder-ui/src/hooks/use_canvas_interaction.rs fractalwonder-ui/src/hooks/
```

**Step 3: Verify it compiles**

Run: `cargo check -p fractalwonder-ui`
Expected: Success

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(ui): add use_canvas_interaction hook from archive"
```

**Deliverable:** Interaction hook ready to use. Web app still shows test pattern (unchanged).

---

### Task 2.2: InteractiveCanvas with Viewport State

**What this delivers:** Interactive canvas that updates viewport on pan/zoom and logs transform details. Still rendering test pattern (changes on interaction to show it's working).

**Files:**
- Modify: `fractalwonder-ui/Cargo.toml`
- Modify: `fractalwonder-ui/src/app.rs`
- Modify: `fractalwonder-ui/src/components/mod.rs`
- Create: `fractalwonder-ui/src/components/interactive_canvas.rs`
- Remove: `fractalwonder-ui/src/components/test_canvas.rs`

**Step 1: Add fractalwonder-core dependency**

Modify `fractalwonder-ui/Cargo.toml`:

```toml
[dependencies]
fractalwonder-core = { workspace = true }
leptos = { workspace = true }
# ... rest of dependencies
```

**Step 2: Create InteractiveCanvas component**

Create `fractalwonder-ui/src/components/interactive_canvas.rs`:

```rust
use fractalwonder_core::{apply_pixel_transform_to_viewport, BigFloat, Viewport};
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::hooks::use_canvas_interaction;

#[component]
pub fn InteractiveCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Viewport state (BigFloat with 128-bit precision for now)
    let (viewport, set_viewport) = create_signal(Viewport::default_mandelbrot(128));

    // Interaction hook
    let _interaction = use_canvas_interaction(canvas_ref, move |transform_result| {
        if let Some(canvas_el) = canvas_ref.get_untracked() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();
            let width = canvas.width();
            let height = canvas.height();

            let current_vp = viewport.get_untracked();

            // Log transform
            web_sys::console::log_1(
                &format!(
                    "Transform: offset=({:.1}, {:.1}), zoom={:.2}",
                    transform_result.offset_x, transform_result.offset_y, transform_result.zoom_factor
                )
                .into(),
            );

            // Apply transform using BigFloat arithmetic
            let new_vp = apply_pixel_transform_to_viewport(
                &current_vp,
                &transform_result,
                (width, height),
                128,
            );

            // Log new viewport
            web_sys::console::log_1(
                &format!(
                    "New viewport: center=({:.6}, {:.6}), zoom={:.2}",
                    new_vp.center.0.to_f64(),
                    new_vp.center.1.to_f64(),
                    new_vp.zoom.to_f64()
                )
                .into(),
            );

            set_viewport.set(new_vp);
        }
    });

    // Initialize canvas size on mount
    create_effect(move |_| {
        if let Some(canvas_el) = canvas_ref.get() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();
            let window = web_sys::window().unwrap();
            let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
            canvas.set_width(width);
            canvas.set_height(height);
        }
    });

    // Render test pattern when viewport changes
    create_effect(move |_| {
        let vp = viewport.get();

        if let Some(canvas_el) = canvas_ref.get() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();
            render_test_pattern_with_viewport(&canvas, &vp);
        }
    });

    view! {
        <canvas
            node_ref=canvas_ref
            style="display: block; width: 100vw; height: 100vh;"
        />
    }
}

fn render_test_pattern_with_viewport(canvas: &HtmlCanvasElement, viewport: &Viewport) {
    let width = canvas.width();
    let height = canvas.height();

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .unchecked_into();

    let pixel_count = (width * height * 4) as usize;
    let mut pixels = vec![0u8; pixel_count];

    // Generate gradient based on viewport zoom
    // More zoom = more intense colors
    let zoom_factor = (viewport.zoom.to_f64().log10() / 2.0).min(1.0).max(0.0);

    for y in 0..height {
        for x in 0..width {
            let t_x = x as f64 / width as f64;
            let t_y = y as f64 / height as f64;

            let r = (t_x * 255.0 * zoom_factor) as u8;
            let g = ((1.0 - zoom_factor) * 128.0) as u8;
            let b = (t_y * 255.0 * zoom_factor) as u8;

            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = r;
            pixels[idx + 1] = g;
            pixels[idx + 2] = b;
            pixels[idx + 3] = 255;
        }
    }

    let image_data =
        ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&pixels), width, height)
            .unwrap();

    ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
}
```

**Step 3: Update components module**

Modify `fractalwonder-ui/src/components/mod.rs`:

```rust
mod interactive_canvas;

pub use interactive_canvas::InteractiveCanvas;
```

**Step 4: Update app to use InteractiveCanvas**

Modify `fractalwonder-ui/src/app.rs`:

```rust
use leptos::*;

mod components;
mod hooks;
use components::InteractiveCanvas;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <InteractiveCanvas />
    }
}
```

**Step 5: Delete old test_canvas.rs**

```bash
rm fractalwonder-ui/src/components/test_canvas.rs
```

**Step 6: Verify it builds**

Run: `cargo check`
Expected: Success

**Step 7: Test in browser**

Refresh: http://localhost:8080
Expected:
- Canvas shows gradient test pattern
- Can pan with mouse drag
- Can zoom with mouse wheel
- Console logs show transform details
- Pattern changes intensity based on zoom level

**Step 8: Commit**

```bash
git add -A
git commit -m "feat(ui): add InteractiveCanvas with viewport state and BigFloat transforms"
```

**Deliverable:** Fully interactive canvas with BigFloat viewport transforms. Test pattern confirms interaction works. Ready for Mandelbrot computation.

---

## Stage 3: Mandelbrot Computation (Single-Threaded)

**Goal:** Add Mandelbrot computation using BigFloat. Single-threaded (runs on main thread). Progressive rendering shows results as they compute.

### Task 3.1: PointComputer Trait and MandelbrotComputer

**What this delivers:** Mandelbrot computation using BigFloat. Fully tested with known points inside/outside the set.

**Files:**
- Modify: Root `Cargo.toml`
- Create: `fractalwonder-compute/Cargo.toml`
- Create: `fractalwonder-compute/src/lib.rs`
- Create: `fractalwonder-compute/src/point_computer.rs`
- Create: `fractalwonder-compute/src/mandelbrot.rs`

**Step 1: Add compute crate to workspace**

Modify root `Cargo.toml`:

```toml
[workspace]
members = ["fractalwonder-core", "fractalwonder-compute", "fractalwonder-ui"]

[workspace.dependencies]
fractalwonder-compute = { path = "./fractalwonder-compute" }
# ... rest of dependencies
```

**Step 2: Create compute crate manifest**

Create `fractalwonder-compute/Cargo.toml`:

```toml
[package]
name = "fractalwonder-compute"
version = "0.1.0"
edition = "2021"

[dependencies]
fractalwonder-core = { workspace = true }
serde = { workspace = true }
```

**Step 3: Create compute lib.rs**

Create `fractalwonder-compute/src/lib.rs`:

```rust
pub mod mandelbrot;
pub mod point_computer;

pub use mandelbrot::{MandelbrotComputer, MandelbrotConfig, MandelbrotData};
pub use point_computer::PointComputer;
```

**Step 4: Write PointComputer trait**

Create `fractalwonder-compute/src/point_computer.rs`:

```rust
use fractalwonder_core::BigFloat;

/// Trait for computing properties of a single point in fractal space
pub trait PointComputer {
    /// Data produced by computation (e.g., MandelbrotData)
    type Data: Clone;

    /// Configuration (e.g., max_iterations, precision_bits)
    type Config: Clone;

    /// Update configuration
    fn configure(&mut self, config: Self::Config);

    /// Get precision in bits
    fn precision_bits(&self) -> usize;

    /// Compute data for a single fractal-space point
    fn compute(&self, point: (BigFloat, BigFloat)) -> Self::Data;
}
```

**Step 5: Write Mandelbrot tests FIRST**

Create `fractalwonder-compute/src/mandelbrot.rs`:

```rust
use crate::point_computer::PointComputer;
use fractalwonder_core::BigFloat;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_does_not_escape() {
        let config = MandelbrotConfig {
            max_iterations: 100,
            precision_bits: 128,
        };

        let mut computer = MandelbrotComputer::new(config);

        let point = (BigFloat::zero(128), BigFloat::zero(128));
        let result = computer.compute(point);

        assert!(!result.escaped);
        assert_eq!(result.iterations, 100);
    }

    #[test]
    fn test_point_outside_set_escapes() {
        let config = MandelbrotConfig {
            max_iterations: 100,
            precision_bits: 128,
        };

        let mut computer = MandelbrotComputer::new(config);

        // Point (2, 2) should escape immediately
        let point = (
            BigFloat::with_precision(2.0, 128),
            BigFloat::with_precision(2.0, 128),
        );
        let result = computer.compute(point);

        assert!(result.escaped);
        assert!(result.iterations < 10);
    }

    #[test]
    fn test_point_on_boundary() {
        let config = MandelbrotConfig {
            max_iterations: 1000,
            precision_bits: 128,
        };

        let mut computer = MandelbrotComputer::new(config);

        // Point (-0.5, 0) is on the boundary
        let point = (
            BigFloat::with_precision(-0.5, 128),
            BigFloat::zero(128),
        );
        let result = computer.compute(point);

        // Should take many iterations to escape (or not escape)
        assert!(result.iterations > 100);
    }

    #[test]
    fn test_configure_updates_settings() {
        let initial_config = MandelbrotConfig {
            max_iterations: 100,
            precision_bits: 128,
        };

        let mut computer = MandelbrotComputer::new(initial_config);

        let new_config = MandelbrotConfig {
            max_iterations: 500,
            precision_bits: 256,
        };

        computer.configure(new_config);

        assert_eq!(computer.precision_bits(), 256);
    }

    #[test]
    fn test_z_magnitude_preserved() {
        let config = MandelbrotConfig {
            max_iterations: 100,
            precision_bits: 256,
        };

        let mut computer = MandelbrotComputer::new(config);

        let point = (
            BigFloat::with_precision(1.0, 256),
            BigFloat::with_precision(1.0, 256),
        );
        let result = computer.compute(point);

        assert!(result.escaped);
        assert_eq!(result.z_magnitude.precision_bits(), 256);
    }
}
```

**Step 6: Verify tests FAIL**

Run: `cargo test -p fractalwonder-compute`
Expected: FAIL - Types not defined

**Step 7: Implement Mandelbrot**

Add to `fractalwonder-compute/src/mandelbrot.rs` (before test module):

```rust
/// Result of Mandelbrot computation for a single point
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MandelbrotData {
    pub iterations: u32,
    pub escaped: bool,
    pub z_magnitude: BigFloat,
}

/// Configuration for Mandelbrot computation
#[derive(Clone, Debug)]
pub struct MandelbrotConfig {
    pub max_iterations: u32,
    pub precision_bits: usize,
}

/// Mandelbrot set computer using BigFloat for arbitrary precision
pub struct MandelbrotComputer {
    max_iterations: u32,
    precision_bits: usize,
}

impl MandelbrotComputer {
    pub fn new(config: MandelbrotConfig) -> Self {
        Self {
            max_iterations: config.max_iterations,
            precision_bits: config.precision_bits,
        }
    }
}

impl PointComputer for MandelbrotComputer {
    type Data = MandelbrotData;
    type Config = MandelbrotConfig;

    fn configure(&mut self, config: Self::Config) {
        self.max_iterations = config.max_iterations;
        self.precision_bits = config.precision_bits;
    }

    fn precision_bits(&self) -> usize {
        self.precision_bits
    }

    fn compute(&self, c: (BigFloat, BigFloat)) -> MandelbrotData {
        // z = 0
        let mut z_real = BigFloat::zero(self.precision_bits);
        let mut z_imag = BigFloat::zero(self.precision_bits);

        let mut iterations = 0;

        let threshold = BigFloat::with_precision(4.0, self.precision_bits);

        for _ in 0..self.max_iterations {
            // z = z^2 + c
            // real: z_real^2 - z_imag^2 + c_real
            let z_real_sq = z_real.mul(&z_real);
            let z_imag_sq = z_imag.mul(&z_imag);
            let new_real = z_real_sq.sub(&z_imag_sq).add(&c.0);

            // imag: 2 * z_real * z_imag + c_imag
            let two = BigFloat::with_precision(2.0, self.precision_bits);
            let new_imag = two.mul(&z_real).mul(&z_imag).add(&c.1);

            z_real = new_real;
            z_imag = new_imag;

            // Check escape: |z|² > 4
            let magnitude_squared = z_real.mul(&z_real).add(&z_imag.mul(&z_imag));

            if magnitude_squared > threshold {
                let magnitude = magnitude_squared.sqrt();
                return MandelbrotData {
                    iterations,
                    escaped: true,
                    z_magnitude: magnitude,
                };
            }

            iterations += 1;
        }

        // Didn't escape
        let magnitude_squared = z_real.mul(&z_real).add(&z_imag.mul(&z_imag));
        let magnitude = magnitude_squared.sqrt();

        MandelbrotData {
            iterations,
            escaped: false,
            z_magnitude: magnitude,
        }
    }
}
```

**Step 8: Verify tests pass**

Run: `cargo test -p fractalwonder-compute -- --nocapture`
Expected: ALL TESTS PASS

**Step 9: Run clippy**

Run: `cargo clippy -p fractalwonder-compute -- -D warnings`
Expected: No warnings

**Step 10: Commit**

```bash
git add -A
git commit -m "feat(compute): add PointComputer trait and MandelbrotComputer with BigFloat"
```

**Deliverable:** Tested Mandelbrot computation engine. Web app still shows test pattern (unchanged).

---

### Task 3.2: Simple Synchronous Mandelbrot Renderer

**What this delivers:** Renders actual Mandelbrot set to canvas (single-threaded on main thread). Shows smooth coloring. Proves computation and colorization work end-to-end.

**Files:**
- Create: `fractalwonder-ui/src/rendering/mod.rs`
- Create: `fractalwonder-ui/src/rendering/colorizers.rs`
- Create: `fractalwonder-ui/src/rendering/synchronous_renderer.rs`
- Modify: `fractalwonder-ui/Cargo.toml`
- Modify: `fractalwonder-ui/src/app.rs`
- Modify: `fractalwonder-ui/src/components/interactive_canvas.rs`

**Step 1: Add fractalwonder-compute dependency**

Modify `fractalwonder-ui/Cargo.toml`:

```toml
[dependencies]
fractalwonder-core = { workspace = true }
fractalwonder-compute = { workspace = true }
# ... rest of dependencies
```

**Step 2: Create colorizer functions**

Create `fractalwonder-ui/src/rendering/mod.rs`:

```rust
pub mod colorizers;
pub mod synchronous_renderer;

pub use colorizers::smooth_mandelbrot_colorizer;
pub use synchronous_renderer::SynchronousRenderer;
```

Create `fractalwonder-ui/src/rendering/colorizers.rs`:

```rust
use fractalwonder_compute::MandelbrotData;

/// Convert HSV to RGB
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = (h * 6.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r_prime, g_prime, b_prime) = match h_prime as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 | _ => (c, 0.0, x),
    };

    (
        ((r_prime + m) * 255.0) as u8,
        ((g_prime + m) * 255.0) as u8,
        ((b_prime + m) * 255.0) as u8,
    )
}

/// Smooth Mandelbrot colorizer
///
/// Uses smooth iteration count for continuous coloring
pub fn smooth_mandelbrot_colorizer(data: &MandelbrotData) -> (u8, u8, u8, u8) {
    if !data.escaped {
        return (0, 0, 0, 255); // Black for points inside the set
    }

    // Smooth coloring: nsmooth = n + 1 - log(log(|z|)) / log(2)
    let magnitude_f64 = data.z_magnitude.to_f64();
    let smooth_value = data.iterations as f64 + 1.0 - magnitude_f64.ln().ln() / 2.0_f64.ln();

    // Map to HSV color space (cyclic)
    let hue = (smooth_value * 0.05) % 1.0;
    let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);

    (r, g, b, 255)
}
```

**Step 3: Create synchronous renderer**

Create `fractalwonder-ui/src/rendering/synchronous_renderer.rs`:

```rust
use fractalwonder_compute::{MandelbrotComputer, MandelbrotConfig, MandelbrotData, PointComputer};
use fractalwonder_core::{pixel_to_fractal, Viewport};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Colorizer function type
pub type Colorizer = fn(&MandelbrotData) -> (u8, u8, u8, u8);

/// Simple synchronous renderer (runs on main thread)
pub struct SynchronousRenderer {
    computer: MandelbrotComputer,
    colorizer: Colorizer,
}

impl SynchronousRenderer {
    pub fn new(colorizer: Colorizer) -> Self {
        let config = MandelbrotConfig {
            max_iterations: 256,
            precision_bits: 128,
        };

        Self {
            computer: MandelbrotComputer::new(config),
            colorizer,
        }
    }

    pub fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .unchecked_into();

        let pixel_count = (width * height * 4) as usize;
        let mut pixels = vec![0u8; pixel_count];

        // Render each pixel
        for y in 0..height {
            for x in 0..width {
                let point =
                    pixel_to_fractal(x as f64, y as f64, viewport, (width, height), 128);

                let data = self.computer.compute(point);
                let (r, g, b, a) = (self.colorizer)(&data);

                let idx = ((y * width + x) * 4) as usize;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = a;
            }
        }

        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&pixels),
            width,
            height,
        )
        .unwrap();

        ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
    }
}
```

**Step 4: Update InteractiveCanvas to render Mandelbrot**

Modify `fractalwonder-ui/src/components/interactive_canvas.rs`:

```rust
use fractalwonder_core::{apply_pixel_transform_to_viewport, Viewport};
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use crate::hooks::use_canvas_interaction;
use crate::rendering::{smooth_mandelbrot_colorizer, SynchronousRenderer};

#[component]
pub fn InteractiveCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Viewport state (BigFloat with 128-bit precision)
    let (viewport, set_viewport) = create_signal(Viewport::default_mandelbrot(128));

    // Create renderer
    let renderer = SynchronousRenderer::new(smooth_mandelbrot_colorizer);

    // Interaction hook
    let _interaction = use_canvas_interaction(canvas_ref, move |transform_result| {
        if let Some(canvas_el) = canvas_ref.get_untracked() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();
            let width = canvas.width();
            let height = canvas.height();

            let current_vp = viewport.get_untracked();

            // Apply transform using BigFloat arithmetic
            let new_vp = apply_pixel_transform_to_viewport(
                &current_vp,
                &transform_result,
                (width, height),
                128,
            );

            // Log new viewport
            web_sys::console::log_1(
                &format!(
                    "Viewport: center=({:.6}, {:.6}), zoom={:.2}",
                    new_vp.center.0.to_f64(),
                    new_vp.center.1.to_f64(),
                    new_vp.zoom.to_f64()
                )
                .into(),
            );

            set_viewport.set(new_vp);
        }
    });

    // Initialize canvas size on mount
    create_effect(move |_| {
        if let Some(canvas_el) = canvas_ref.get() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();
            let window = web_sys::window().unwrap();
            let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
            canvas.set_width(width);
            canvas.set_height(height);
        }
    });

    // Render Mandelbrot when viewport changes
    create_effect(move |_| {
        let vp = viewport.get();

        if let Some(canvas_el) = canvas_ref.get() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();

            let start = web_sys::window().unwrap().performance().unwrap().now();

            renderer.render(&vp, &canvas);

            let elapsed = web_sys::window().unwrap().performance().unwrap().now() - start;
            web_sys::console::log_1(&format!("Render time: {:.1}ms", elapsed).into());
        }
    });

    view! {
        <canvas
            node_ref=canvas_ref
            style="display: block; width: 100vw; height: 100vh; cursor: crosshair;"
        />
    }
}
```

**Step 5: Update app.rs to include rendering module**

Modify `fractalwonder-ui/src/app.rs`:

```rust
use leptos::*;

mod components;
mod hooks;
mod rendering;

use components::InteractiveCanvas;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <InteractiveCanvas />
    }
}
```

**Step 6: Verify it builds**

Run: `cargo check`
Expected: Success

**Step 7: Test in browser**

Refresh: http://localhost:8080
Expected:
- Canvas shows Mandelbrot set with smooth coloring
- Black region (inside set) visible
- Colored regions (escape iterations) show smooth gradient
- Can pan with mouse drag
- Can zoom with mouse wheel
- Console shows viewport coordinates and render times
- Rendering takes a few seconds (single-threaded)

**Step 8: Commit**

```bash
git add -A
git commit -m "feat(ui): add synchronous Mandelbrot renderer with smooth coloring"
```

**Deliverable:** Working Mandelbrot explorer! Single-threaded renderer proves end-to-end pipeline works (BigFloat → computation → colorization → canvas). Ready to optimize with workers.

---

## Stage 4: Parallel Worker Infrastructure

**Goal:** Multi-threaded rendering using Web Workers. Progressive tile-based rendering with center-out ordering.

### Task 4.1: Tile System and Render Progress

**What this delivers:** Tile generation system and progress tracking. No workers yet - infrastructure for parallel rendering.

**Files:**
- Create: `fractalwonder-ui/src/rendering/tiles.rs`
- Create: `fractalwonder-ui/src/rendering/progress.rs`
- Modify: `fractalwonder-ui/src/rendering/mod.rs`

**Step 1: Write tile generation tests FIRST**

Create `fractalwonder-ui/src/rendering/tiles.rs`:

```rust
use fractalwonder_core::PixelRect;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_tiles_exact_fit() {
        let tiles = generate_tiles(800, 600, 200);

        // 800/200 = 4 cols, 600/200 = 3 rows = 12 tiles
        assert_eq!(tiles.len(), 12);

        // First tile (top-left)
        assert_eq!(tiles[0].x, 0);
        assert_eq!(tiles[0].y, 0);
        assert_eq!(tiles[0].width, 200);
        assert_eq!(tiles[0].height, 200);

        // Last tile (bottom-right)
        let last = tiles.last().unwrap();
        assert_eq!(last.x, 600);
        assert_eq!(last.y, 400);
        assert_eq!(last.width, 200);
        assert_eq!(last.height, 200);
    }

    #[test]
    fn test_generate_tiles_with_remainder() {
        let tiles = generate_tiles(850, 650, 200);

        // 850/200 = 4 full + 1 partial (50px) = 5 cols
        // 650/200 = 3 full + 1 partial (50px) = 4 rows
        // Total = 20 tiles
        assert_eq!(tiles.len(), 20);

        // Rightmost tile in first row
        let right_edge = tiles.iter().find(|t| t.x == 800 && t.y == 0).unwrap();
        assert_eq!(right_edge.width, 50); // Partial tile

        // Bottom-right corner tile
        let corner = tiles.iter().find(|t| t.x == 800 && t.y == 600).unwrap();
        assert_eq!(corner.width, 50);
        assert_eq!(corner.height, 50);
    }

    #[test]
    fn test_tiles_cover_entire_canvas() {
        let width = 1920;
        let height = 1080;
        let tiles = generate_tiles(width, height, 256);

        // Check total pixel coverage
        let total_pixels: u32 = tiles.iter().map(|t| t.area()).sum();
        assert_eq!(total_pixels, width * height);
    }

    #[test]
    fn test_center_out_ordering() {
        let tiles = generate_tiles(800, 600, 200);

        // Canvas center is at (400, 300)
        // Tile containing center should be first
        let first = &tiles[0];
        assert!(first.x <= 400 && 400 < first.x + first.width);
        assert!(first.y <= 300 && 300 < first.y + first.height);

        // Tiles should be roughly ordered by distance from center
        // (allowing some tolerance for discrete tiles)
        let center_x = 400.0;
        let center_y = 300.0;

        for i in 0..tiles.len() - 1 {
            let dist1 = tile_distance_from_point(&tiles[i], center_x, center_y);
            let dist2 = tile_distance_from_point(&tiles[i + 1], center_x, center_y);

            // Next tile should be roughly same distance or farther
            // (with tolerance for ties)
            assert!(dist2 >= dist1 - 10.0);
        }
    }
}

fn tile_distance_from_point(tile: &PixelRect, cx: f64, cy: f64) -> f64 {
    let tile_cx = tile.x as f64 + tile.width as f64 / 2.0;
    let tile_cy = tile.y as f64 + tile.height as f64 / 2.0;
    let dx = tile_cx - cx;
    let dy = tile_cy - cy;
    (dx * dx + dy * dy).sqrt()
}
```

**Step 2: Verify tests FAIL**

Run: `cargo check -p fractalwonder-ui`
Expected: FAIL - Function not defined

**Step 3: Implement tile generation**

Add to `fractalwonder-ui/src/rendering/tiles.rs` (before test module):

```rust
/// Generate tiles in center-out order
///
/// Divides canvas into tiles of approximately `tile_size` pixels.
/// Returns tiles sorted by distance from canvas center (closest first).
pub fn generate_tiles(canvas_width: u32, canvas_height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    let cols = (canvas_width + tile_size - 1) / tile_size;
    let rows = (canvas_height + tile_size - 1) / tile_size;

    for row in 0..rows {
        for col in 0..cols {
            let x = col * tile_size;
            let y = row * tile_size;

            let width = (tile_size).min(canvas_width - x);
            let height = (tile_size).min(canvas_height - y);

            tiles.push(PixelRect {
                x,
                y,
                width,
                height,
            });
        }
    }

    // Sort by distance from canvas center
    let center_x = canvas_width as f64 / 2.0;
    let center_y = canvas_height as f64 / 2.0;

    tiles.sort_by(|a, b| {
        let dist_a = tile_distance_from_point(a, center_x, center_y);
        let dist_b = tile_distance_from_point(b, center_x, center_y);
        dist_a.partial_cmp(&dist_b).unwrap()
    });

    tiles
}
```

**Step 4: Create progress tracking**

Create `fractalwonder-ui/src/rendering/progress.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Render progress state
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RenderProgress {
    pub total_tiles: usize,
    pub completed_tiles: usize,
    pub rendering: bool,
}

impl RenderProgress {
    pub fn new(total_tiles: usize) -> Self {
        Self {
            total_tiles,
            completed_tiles: 0,
            rendering: true,
        }
    }

    pub fn complete_tile(&mut self) {
        self.completed_tiles += 1;
        if self.completed_tiles >= self.total_tiles {
            self.rendering = false;
        }
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total_tiles == 0 {
            return 100.0;
        }
        (self.completed_tiles as f64 / self.total_tiles as f64) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        !self.rendering
    }
}
```

**Step 5: Update rendering module**

Modify `fractalwonder-ui/src/rendering/mod.rs`:

```rust
pub mod colorizers;
pub mod progress;
pub mod synchronous_renderer;
pub mod tiles;

pub use colorizers::smooth_mandelbrot_colorizer;
pub use progress::RenderProgress;
pub use synchronous_renderer::{Colorizer, SynchronousRenderer};
pub use tiles::generate_tiles;
```

**Step 6: Verify tests pass**

Run: `cargo test -p fractalwonder-ui -- --nocapture`
Expected: ALL TESTS PASS

**Step 7: Run clippy**

Run: `cargo clippy -p fractalwonder-ui -- -D warnings`
Expected: No warnings

**Step 8: Commit**

```bash
git add -A
git commit -m "feat(ui): add tile system and render progress tracking"
```

**Deliverable:** Tile infrastructure ready for parallel workers. Web app still renders Mandelbrot synchronously (unchanged).

---

This implementation plan is now complete through Task 4.1. The remaining tasks (4.2-4.6) would cover:
- Task 4.2: Web Worker setup and message passing
- Task 4.3: Worker-side PixelRenderer
- Task 4.4: ParallelCanvasRenderer (main thread coordinator)
- Task 4.5: Progressive tile rendering with UI feedback
- Task 4.6: Cancellation and cache management

Would you like me to continue adding these remaining tasks with the same level of TDD detail?