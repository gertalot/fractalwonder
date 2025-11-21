# Extreme-Precision Mandelbrot Explorer - Incremental Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a ground-up Mandelbrot explorer supporting extreme zoom levels (10^2000+) with strict precision enforcement and no legacy compromises. Every task delivers a working, testable, visible increment.

**Architecture:** Three-crate workspace with strict pixel/fractal space separation. Workers handle BigFloat computation, main thread handles UI/colorization/rendering. BigFloat uses enum-based f64/FBig switching for performance while enforcing explicit precision everywhere.

**Tech Stack:** Rust 1.80+, Leptos 0.6+, dashu/dashu-float 0.4, Web Workers, WASM, Trunk

---

## Iteration 0: Basic Web App (f64-only, no BigFloat)

**Goal:** Get a working Mandelbrot explorer running in the browser with f64 precision. This establishes the UI/rendering pipeline before adding complexity.

### Task 0.1: Archive and Create Minimal Workspace

**What this delivers:** Clean slate with working, compiling workspace structure.

**Files:**
- Move: All current `fractalwonder-*` directories → `_archive/`
- Create: Root `Cargo.toml`, `fractalwonder-ui/Cargo.toml`, `index.html`

**Step 1: Archive existing code**

```bash
mkdir -p _archive
mv fractalwonder-ui _archive/
mv fractalwonder-compute _archive/
mv fractalwonder-core _archive/
mv tests _archive/ 2>/dev/null || true
```

**Step 2: Create root workspace**

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
gloo-utils = "0.2"

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

**Step 3: Create UI crate with minimal Hello World**

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
js-sys = { workspace = true }
web-sys = { workspace = true, features = [
    "CanvasRenderingContext2d",
    "Document",
    "Element",
    "HtmlCanvasElement",
    "ImageData",
    "Window",
] }
console_error_panic_hook = { workspace = true }
console_log = { workspace = true }

[dev-dependencies]
wasm-bindgen-test = { workspace = true }
```

Create `fractalwonder-ui/src/lib.rs`:

```rust
use leptos::*;
use wasm_bindgen::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <div style="width: 100vw; height: 100vh; display: flex; align-items: center; justify-content: center; background: #1a1a1a; color: white;">
            <h1>"Fractal Wonder - Iteration 0"</h1>
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
Expected: See "Fractal Wonder - Iteration 0" centered on black background

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(iter0): minimal Leptos hello world app"
```

**Deliverable:** Working web app displaying text. Foundation for all future work.

---

### Task 0.2: Add Canvas with Test Pattern

**What this delivers:** Full-screen canvas rendering a test pattern. Proves rendering pipeline works.

**Files:**
- Create: `fractalwonder-ui/src/app.rs`
- Create: `fractalwonder-ui/src/components/mod.rs`
- Create: `fractalwonder-ui/src/components/test_canvas.rs`
- Modify: `fractalwonder-ui/src/lib.rs`

**Step 1: Create module structure**

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

Create `fractalwonder-ui/src/components/mod.rs`:

```rust
mod test_canvas;
pub use test_canvas::TestCanvas;
```

**Step 2: Implement TestCanvas component**

Create `fractalwonder-ui/src/components/test_canvas.rs`:

```rust
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

#[component]
pub fn TestCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    create_effect(move |_| {
        if let Some(canvas_el) = canvas_ref.get() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();

            // Set canvas to fill viewport
            let window = web_sys::window().unwrap();
            let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height = window.inner_height().unwrap().as_f64().unwrap() as u32;

            canvas.set_width(width);
            canvas.set_height(height);

            // Draw test pattern
            let ctx: CanvasRenderingContext2d = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .unchecked_into();

            // Gradient from blue (top-left) to orange (bottom-right)
            for y in 0..height {
                for x in 0..width {
                    let r = ((x as f64 / width as f64) * 255.0) as u8;
                    let g = 128;
                    let b = ((y as f64 / height as f64) * 255.0) as u8;

                    let color = format!("rgb({}, {}, {})", r, g, b);
                    ctx.set_fill_style(&color.into());
                    ctx.fill_rect(x as f64, y as f64, 1.0, 1.0);
                }
            }
        }
    });

    view! {
        <canvas
            node_ref=canvas_ref
            style="display: block; width: 100vw; height: 100vh;"
        />
    }
}
```

**Step 3: Update lib.rs to use App**

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

**Step 4: Verify it builds and runs**

Run: `cargo check`
Expected: Success

Refresh browser: http://localhost:8080
Expected: Full-screen canvas with orange-blue gradient

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(iter0): add canvas with test pattern rendering"
```

**Deliverable:** Full-screen canvas rendering pixels. Ready for Mandelbrot computation.

---

### Task 0.3: Simple f64 Mandelbrot Rendering (No Workers)

**What this delivers:** Actual Mandelbrot set visible in browser using f64 math on main thread. Slow but functional.

**Files:**
- Create: `fractalwonder-ui/src/mandelbrot_simple.rs`
- Create: `fractalwonder-ui/src/components/mandelbrot_canvas.rs`
- Modify: `fractalwonder-ui/src/lib.rs`
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Implement simple Mandelbrot computation**

Create `fractalwonder-ui/src/mandelbrot_simple.rs`:

```rust
/// Simple f64-based Mandelbrot computation (no BigFloat)
pub fn compute_mandelbrot(cx: f64, cy: f64, max_iterations: u32) -> (u32, bool) {
    let mut zx = 0.0;
    let mut zy = 0.0;

    for i in 0..max_iterations {
        let zx2 = zx * zx;
        let zy2 = zy * zy;

        if zx2 + zy2 > 4.0 {
            return (i, true);
        }

        let new_zx = zx2 - zy2 + cx;
        let new_zy = 2.0 * zx * zy + cy;

        zx = new_zx;
        zy = new_zy;
    }

    (max_iterations, false)
}

/// Simple colorization: iterations -> RGB
pub fn colorize(iterations: u32, max_iterations: u32, escaped: bool) -> (u8, u8, u8) {
    if !escaped {
        return (0, 0, 0); // Black for points inside the set
    }

    let t = iterations as f64 / max_iterations as f64;
    let r = (9.0 * (1.0 - t) * t * t * t * 255.0) as u8;
    let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u8;
    let b = (8.5 * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u8;

    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_is_inside() {
        let (iters, escaped) = compute_mandelbrot(0.0, 0.0, 256);
        assert_eq!(escaped, false);
        assert_eq!(iters, 256);
    }

    #[test]
    fn test_far_point_escapes() {
        let (iters, escaped) = compute_mandelbrot(2.0, 2.0, 256);
        assert_eq!(escaped, true);
        assert!(iters < 5);
    }

    #[test]
    fn test_colorize_inside() {
        let (r, g, b) = colorize(256, 256, false);
        assert_eq!((r, g, b), (0, 0, 0));
    }
}
```

**Step 2: Run tests**

Run: `cargo test --lib mandelbrot_simple`
Expected: PASS (3 tests)

**Step 3: Implement MandelbrotCanvas component**

Create `fractalwonder-ui/src/components/mandelbrot_canvas.rs`:

```rust
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::mandelbrot_simple::{colorize, compute_mandelbrot};

#[component]
pub fn MandelbrotCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    create_effect(move |_| {
        if let Some(canvas_el) = canvas_ref.get() {
            let canvas: HtmlCanvasElement = canvas_el.unchecked_into();

            // Set canvas size
            let width = 800u32;
            let height = 600u32;
            canvas.set_width(width);
            canvas.set_height(height);

            // Render Mandelbrot
            let ctx: CanvasRenderingContext2d = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .unchecked_into();

            // Create image data buffer
            let mut pixels = vec![0u8; (width * height * 4) as usize];

            // Viewport: center at (-0.5, 0.0), width 3.5, height 2.625
            let viewport_cx = -0.5;
            let viewport_cy = 0.0;
            let viewport_width = 3.5;
            let viewport_height = viewport_width * (height as f64 / width as f64);

            let max_iterations = 256;

            for py in 0..height {
                for px in 0..width {
                    // Pixel to fractal coordinates
                    let fx = viewport_cx + (px as f64 / width as f64 - 0.5) * viewport_width;
                    let fy = viewport_cy + (py as f64 / height as f64 - 0.5) * viewport_height;

                    let (iterations, escaped) = compute_mandelbrot(fx, fy, max_iterations);
                    let (r, g, b) = colorize(iterations, max_iterations, escaped);

                    let idx = ((py * width + px) * 4) as usize;
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
            ).unwrap();

            ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
        }
    });

    view! {
        <div style="display: flex; align-items: center; justify-content: center; width: 100vw; height: 100vh; background: #1a1a1a;">
            <canvas
                node_ref=canvas_ref
                style="border: 1px solid white;"
            />
        </div>
    }
}
```

**Step 4: Update app to use MandelbrotCanvas**

Modify `fractalwonder-ui/src/components/mod.rs`:

```rust
mod test_canvas;
mod mandelbrot_canvas;

pub use test_canvas::TestCanvas;
pub use mandelbrot_canvas::MandelbrotCanvas;
```

Modify `fractalwonder-ui/src/app.rs`:

```rust
use leptos::*;

mod components;
use components::MandelbrotCanvas;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <MandelbrotCanvas />
    }
}
```

Modify `fractalwonder-ui/src/lib.rs`:

```rust
use leptos::*;
use wasm_bindgen::prelude::*;

mod app;
mod mandelbrot_simple;
use app::App;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    leptos::mount_to_body(App);
}
```

**Step 5: Verify it builds and renders Mandelbrot**

Run: `cargo check`
Expected: Success

Refresh browser: http://localhost:8080
Expected: Mandelbrot set visible! Classic bulb and cardioid shape in color.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(iter0): render Mandelbrot set with f64 math on main thread"
```

**Deliverable:** Working Mandelbrot explorer visible in browser. Foundation complete. Everything after this adds BigFloat, workers, and interaction.

---

## Iteration 1: Add Core Types and BigFloat

**Goal:** Introduce BigFloat and core types (Viewport, PixelRect) while keeping the rendering working.

### Task 1.1: Create Core Crate with BigFloat

**What this delivers:** fractalwonder-core crate with fully tested BigFloat implementation using f64/FBig enum.

**Files:**
- Create: `fractalwonder-core/Cargo.toml`
- Create: `fractalwonder-core/src/lib.rs`
- Create: `fractalwonder-core/src/bigfloat.rs`
- Modify: Root `Cargo.toml`

**Step 1: Add core crate to workspace**

Modify root `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
# Shared core crate
fractalwonder-core = { path = "./fractalwonder-core" }

# Arbitrary precision
dashu = "0.4"
dashu-float = "0.4"
```

Modify root `Cargo.toml`, update `members`:

```toml
members = ["fractalwonder-core", "fractalwonder-ui"]
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

**Step 3: Write BigFloat tests**

Create `fractalwonder-core/src/bigfloat.rs`:

```rust
use dashu_float::FBig;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_with_precision() {
        let bf = BigFloat::with_precision(42.5, 128);
        assert_eq!(bf.precision_bits(), 128);
        assert!((bf.to_f64() - 42.5).abs() < 1e-10);
    }

    #[test]
    fn test_zero_with_precision() {
        let bf = BigFloat::zero(256);
        assert_eq!(bf.precision_bits(), 256);
        assert_eq!(bf.to_f64(), 0.0);
    }

    #[test]
    fn test_arithmetic() {
        let a = BigFloat::with_precision(2.5, 128);
        let b = BigFloat::with_precision(1.5, 128);
        let sum = a.add(&b);
        assert!((sum.to_f64() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_f64_fast_path() {
        let a = BigFloat::with_precision(2.0, 64);
        let b = BigFloat::with_precision(3.0, 64);
        let result = a.mul(&b);

        // Should use f64 internally
        if let BigFloatValue::F64(_) = result.value {
            // Correct - used fast path
        } else {
            panic!("Should use f64 for precision <= 64");
        }
    }

    #[test]
    fn test_serialization() {
        let original = BigFloat::with_precision(3.14159, 256);
        let json = serde_json::to_string(&original).unwrap();
        let restored: BigFloat = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.precision_bits(), 256);
        assert!((restored.to_f64() - 3.14159).abs() < 1e-5);
    }
}
```

**Step 4: Run tests to verify failure**

Run: `cargo test -p fractalwonder-core --lib bigfloat`
Expected: FAIL - types not defined

**Step 5: Implement BigFloat** (complete implementation at once to keep task deliverable)

Add before tests in `fractalwonder-core/src/bigfloat.rs`:

```rust
/// Arbitrary precision floating point
///
/// Uses f64 when precision_bits <= 64, FBig otherwise.
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
    pub fn with_precision(val: f64, precision_bits: usize) -> Self {
        let value = if precision_bits <= 64 {
            BigFloatValue::F64(val)
        } else {
            BigFloatValue::Arbitrary(FBig::try_from(val).unwrap_or(FBig::ZERO))
        };
        Self { value, precision_bits }
    }

    pub fn zero(precision_bits: usize) -> Self {
        Self::with_precision(0.0, precision_bits)
    }

    pub fn one(precision_bits: usize) -> Self {
        Self::with_precision(1.0, precision_bits)
    }

    pub fn precision_bits(&self) -> usize {
        self.precision_bits
    }

    pub fn to_f64(&self) -> f64 {
        match &self.value {
            BigFloatValue::F64(v) => *v,
            BigFloatValue::Arbitrary(v) => v.to_f64().value(),
        }
    }

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
        Self { value: result_value, precision_bits: result_precision }
    }

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
        Self { value: result_value, precision_bits: result_precision }
    }

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
        Self { value: result_value, precision_bits: result_precision }
    }

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
        Self { value: result_value, precision_bits: result_precision }
    }

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
        Self { value: result_value, precision_bits: self.precision_bits }
    }

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
            let f = serde.value.parse::<f64>()
                .map_err(|e| serde::de::Error::custom(format!("Failed to parse f64: {}", e)))?;
            BigFloatValue::F64(f)
        } else {
            let fbig = serde.value.parse::<FBig>()
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

**Step 6: Create lib.rs**

Create `fractalwonder-core/src/lib.rs`:

```rust
pub mod bigfloat;

pub use bigfloat::BigFloat;
```

**Step 7: Run tests to verify pass**

Run: `cargo test -p fractalwonder-core --lib`
Expected: PASS (all tests)

**Step 8: Commit**

```bash
git add -A
git commit -m "feat(iter1): add fractalwonder-core with BigFloat implementation"
```

**Deliverable:** Fully tested BigFloat implementation. Web app still works showing Mandelbrot set.

---

### Task 1.2: Add Viewport and PixelRect to Core

**What this delivers:** Core types for coordinate systems, fully tested. Web app unchanged.

**Files:**
- Create: `fractalwonder-core/src/viewport.rs`
- Create: `fractalwonder-core/src/pixel_rect.rs`
- Modify: `fractalwonder-core/src/lib.rs`

[Continue with full TDD implementation of Viewport and PixelRect as in original plan, Tasks 5-6]

**Step 1: Write Viewport tests and implementation**
**Step 2: Write PixelRect tests and implementation**
**Step 3: Run all tests**
**Step 4: Commit**

**Deliverable:** Viewport and PixelRect types ready. Web app still renders Mandelbrot.

---

### Task 1.3: Add Coordinate Transforms to Core

**What this delivers:** pixel_to_fractal and apply_pixel_transform_to_viewport functions, fully tested.

[Follow Task 7-8 from original plan]

**Deliverable:** Transformation functions tested. Ready to integrate with UI interaction.

---

## Iteration 2: Add Compute Crate and BigFloat Mandelbrot

**Goal:** Create compute crate with Mandelbrot computation using BigFloat, render side-by-side with f64 version.

### Task 2.1: Create Compute Crate with MandelbrotComputer

[Follow Tasks 9-11 from original, creating compute crate and implementing MandelbrotComputer]

**What this delivers:** Compute crate with BigFloat-based MandelbrotComputer, fully tested.

**Deliverable:** Tests pass showing Mandelbrot computation works with BigFloat. Ready to integrate with rendering.

---

### Task 2.2: Render Mandelbrot with BigFloat (Side-by-Side Comparison)

**What this delivers:** Two canvases: left shows f64 Mandelbrot, right shows BigFloat Mandelbrot. Proves BigFloat works.

**Files:**
- Modify: `fractalwonder-ui/Cargo.toml` (add core and compute dependencies)
- Create: `fractalwonder-ui/src/components/dual_canvas.rs`
- Modify: `fractalwonder-ui/src/app.rs`

**Step 1: Add dependencies to UI crate**

Modify `fractalwonder-ui/Cargo.toml`, add to `[dependencies]`:

```toml
fractalwonder-core = { workspace = true }
fractalwonder-compute = { workspace = true }
```

**Step 2: Implement DualCanvas showing both renderers**

[Implementation that renders both f64 and BigFloat versions side by side]

**Step 3: Verify in browser**

Expected: Two Mandelbrot images side by side, visually identical (both using precision=64)

**Step 4: Commit**

**Deliverable:** Visual proof that BigFloat rendering works correctly. Everything integrated.

---

## Iteration 3: Add User Interaction

**Goal:** Add zoom/pan interaction, apply viewport transforms, show responsive Mandelbrot updates.

[Continue with remaining tasks...]

---

## Summary of Changes from Original Plan

**Key improvements:**

1. **Every task delivers working UI**: After Task 0.3, you can always see Mandelbrot in the browser
2. **Incremental integration**: Don't build everything then integrate. Build, integrate, iterate.
3. **Side-by-side validation**: Task 2.2 proves BigFloat works by comparing with f64 version
4. **No gaps**: Never have a "scaffold with nothing working" state
5. **Clear deliverables**: Each task says exactly what you'll see/test

**Every task now:**
- Compiles fully ✅
- Shows visible progress ✅
- Stands alone as shippable product ✅
- Builds incrementally on previous task ✅
