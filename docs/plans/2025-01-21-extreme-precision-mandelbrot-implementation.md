# Extreme-Precision Mandelbrot Explorer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a ground-up Mandelbrot explorer supporting extreme zoom levels (10^2000+) with strict precision enforcement and no legacy compromises.

**Architecture:** Three-crate workspace with strict pixel/fractal space separation. Workers handle BigFloat computation, main thread handles UI/colorization/rendering. BigFloat uses enum-based f64/FBig switching for performance while enforcing explicit precision everywhere.

**Tech Stack:** Rust 1.80+, Leptos 0.6+, dashu/dashu-float 0.4, Web Workers, WASM, Trunk

---

## Prerequisites

### Task 0: Archive Existing Code

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
mv tests _archive/
```

**Step 3: Verify structure**

Run: `ls -la`
Expected: Only `_archive/`, `Cargo.toml`, `Trunk.toml`, `docs/`, `.claude/`, `target/` remain

**Step 4: Commit**

```bash
git add -A
git commit -m "chore: archive existing implementation for ground-up rebuild"
```

---

## Stage 0: Project Structure

### Task 1: Create Workspace Structure

**Files:**
- Modify: `Cargo.toml` (root workspace)
- Create: `fractalwonder-core/Cargo.toml`
- Create: `fractalwonder-core/src/lib.rs`
- Create: `fractalwonder-compute/Cargo.toml`
- Create: `fractalwonder-compute/src/lib.rs`
- Create: `fractalwonder-ui/Cargo.toml`
- Create: `fractalwonder-ui/src/lib.rs`

**Step 1: Write root workspace manifest**

Create/replace `Cargo.toml`:

```toml
[workspace]
members = [
    "fractalwonder-core",
    "fractalwonder-compute",
    "fractalwonder-ui",
]
resolver = "2"

[workspace.dependencies]
# Shared core crate
fractalwonder-core = { path = "./fractalwonder-core" }
fractalwonder-compute = { path = "./fractalwonder-compute" }

# Arbitrary precision
dashu = "0.4"
dashu-float = "0.4"

# WASM/JS bindings
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3" }
gloo-utils = "0.2"

# Leptos framework
leptos = { version = "0.6", features = ["csr", "nightly"] }
leptos_meta = { version = "0.6", features = ["csr"] }

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

**Step 2: Create fractalwonder-core crate**

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

Create `fractalwonder-core/src/lib.rs`:

```rust
// Shared types and utilities for fractal rendering
// NO UI dependencies, NO compute logic - just types and transforms

pub mod bigfloat;
pub mod viewport;
pub mod pixel_rect;
pub mod transforms;

pub use bigfloat::BigFloat;
pub use viewport::Viewport;
pub use pixel_rect::PixelRect;
```

**Step 3: Create fractalwonder-compute crate**

Create `fractalwonder-compute/Cargo.toml`:

```toml
[package]
name = "fractalwonder-compute"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
fractalwonder-core = { workspace = true }
wasm-bindgen = { workspace = true }
js-sys = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
console_error_panic_hook = { workspace = true }

[dev-dependencies]
wasm-bindgen-test = { workspace = true }
```

Create `fractalwonder-compute/src/lib.rs`:

```rust
// Compute engine for workers
// Renderer trait, PointComputer trait, Mandelbrot implementation

pub mod point_computer;
pub mod pixel_renderer;
pub mod mandelbrot;
pub mod worker;

pub use point_computer::PointComputer;
pub use pixel_renderer::PixelRenderer;
pub use mandelbrot::{MandelbrotComputer, MandelbrotConfig, MandelbrotData};
```

**Step 4: Create fractalwonder-ui crate**

Create `fractalwonder-ui/Cargo.toml`:

```toml
[package]
name = "fractalwonder-ui"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
fractalwonder-core = { workspace = true }
fractalwonder-compute = { workspace = true }
leptos = { workspace = true }
leptos_meta = { workspace = true }
wasm-bindgen = { workspace = true }
js-sys = { workspace = true }
web-sys = { workspace = true }
gloo-utils = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
console_error_panic_hook = { workspace = true }
console_log = { workspace = true }

[dev-dependencies]
wasm-bindgen-test = { workspace = true }
```

Create `fractalwonder-ui/src/lib.rs`:

```rust
use leptos::*;
use wasm_bindgen::prelude::*;

mod app;
mod components;
mod hooks;
mod rendering;

pub use app::App;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");

    leptos::mount_to_body(App);
}
```

**Step 5: Create index.html**

Create `index.html` in root:

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Fractal Wonder</title>
    <link data-trunk rel="rust" data-wasm-opt="z" data-bin="fractalwonder-ui" />
    <link data-trunk rel="tailwind-css" href="input.css" />
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

**Step 6: Create Tailwind CSS input**

Create `input.css` in root:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

**Step 7: Verify workspace builds**

Run: `cargo check --workspace --all-targets`
Expected: Success (empty crates compile)

**Step 8: Commit**

```bash
git add -A
git commit -m "feat: create workspace structure for ground-up rebuild"
```

---

## Stage 1: BigFloat Implementation

### Task 2: BigFloat Core Structure

**Files:**
- Create: `fractalwonder-core/src/bigfloat.rs`
- Test: `fractalwonder-core/src/bigfloat.rs` (inline tests)

**Step 1: Write test for BigFloat creation with explicit precision**

Add to `fractalwonder-core/src/bigfloat.rs`:

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
    fn test_one_with_precision() {
        let bf = BigFloat::one(64);
        assert_eq!(bf.precision_bits(), 64);
        assert_eq!(bf.to_f64(), 1.0);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-core --lib bigfloat`
Expected: FAIL - BigFloat not defined

**Step 3: Implement BigFloat enum structure**

Add before tests in `fractalwonder-core/src/bigfloat.rs`:

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
enum BigFloatValue {
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
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core --lib bigfloat`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/bigfloat.rs
git commit -m "feat(core): add BigFloat creation with explicit precision"
```

### Task 3: BigFloat Arithmetic Operations

**Files:**
- Modify: `fractalwonder-core/src/bigfloat.rs`

**Step 1: Write tests for arithmetic operations**

Add to test module in `fractalwonder-core/src/bigfloat.rs`:

```rust
#[test]
fn test_add_preserves_max_precision() {
    let a = BigFloat::with_precision(2.5, 128);
    let b = BigFloat::with_precision(1.5, 256);
    let result = a.add(&b);
    assert_eq!(result.precision_bits(), 256); // Max precision
    assert!((result.to_f64() - 4.0).abs() < 1e-10);
}

#[test]
fn test_sub() {
    let a = BigFloat::with_precision(5.0, 128);
    let b = BigFloat::with_precision(3.0, 128);
    let result = a.sub(&b);
    assert!((result.to_f64() - 2.0).abs() < 1e-10);
}

#[test]
fn test_mul() {
    let a = BigFloat::with_precision(3.0, 128);
    let b = BigFloat::with_precision(4.0, 128);
    let result = a.mul(&b);
    assert!((result.to_f64() - 12.0).abs() < 1e-10);
}

#[test]
fn test_div() {
    let a = BigFloat::with_precision(10.0, 128);
    let b = BigFloat::with_precision(2.0, 128);
    let result = a.div(&b);
    assert!((result.to_f64() - 5.0).abs() < 1e-10);
}

#[test]
fn test_sqrt() {
    let a = BigFloat::with_precision(16.0, 128);
    let result = a.sqrt();
    assert!((result.to_f64() - 4.0).abs() < 1e-10);
}

#[test]
fn test_f64_fast_path() {
    // When precision <= 64, should use f64 internally
    let a = BigFloat::with_precision(2.0, 64);
    let b = BigFloat::with_precision(3.0, 64);
    let result = a.mul(&b);

    // Verify it used f64 path
    if let BigFloatValue::F64(_) = result.value {
        // Good - used fast path
    } else {
        panic!("Should have used f64 fast path");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-core --lib bigfloat`
Expected: FAIL - methods not defined

**Step 3: Implement arithmetic operations**

Add to `impl BigFloat` block:

```rust
/// Add two BigFloats, preserving max precision
pub fn add(&self, other: &Self) -> Self {
    let result_precision = self.precision_bits.max(other.precision_bits);

    let result_value = match (&self.value, &other.value) {
        (BigFloatValue::F64(a), BigFloatValue::F64(b)) if result_precision <= 64 => {
            BigFloatValue::F64(a + b)
        }
        _ => {
            // Use arbitrary precision
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core --lib bigfloat`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/bigfloat.rs
git commit -m "feat(core): implement BigFloat arithmetic with f64 fast path"
```

### Task 4: BigFloat Comparison and Serialization

**Files:**
- Modify: `fractalwonder-core/src/bigfloat.rs`

**Step 1: Write tests for comparison**

Add to test module:

```rust
#[test]
fn test_partial_ord() {
    let a = BigFloat::with_precision(2.5, 128);
    let b = BigFloat::with_precision(3.5, 128);
    assert!(a < b);
    assert!(b > a);
}

#[test]
fn test_partial_eq() {
    let a = BigFloat::with_precision(2.5, 128);
    let b = BigFloat::with_precision(2.5, 256);
    assert_eq!(a, b); // Values equal, precision doesn't affect equality
}
```

**Step 2: Write tests for serialization**

Add to test module:

```rust
#[test]
fn test_serialization_roundtrip() {
    let original = BigFloat::with_precision(3.14159, 256);
    let json = serde_json::to_string(&original).expect("serialize failed");
    let deserialized: BigFloat = serde_json::from_str(&json).expect("deserialize failed");

    assert_eq!(deserialized.precision_bits(), 256);
    assert!((deserialized.to_f64() - 3.14159).abs() < 1e-5);
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-core --lib bigfloat`
Expected: FAIL - traits not implemented

**Step 4: Implement PartialEq and PartialOrd**

Add after `impl BigFloat`:

```rust
impl PartialEq for BigFloat {
    fn eq(&self, other: &Self) -> bool {
        // Compare values, not precision
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
```

**Step 5: Implement Serialize and Deserialize**

Add after PartialOrd:

```rust
#[derive(Serialize, Deserialize)]
struct BigFloatSerde {
    value: String,  // String representation preserves precision
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

**Step 6: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core --lib bigfloat`
Expected: PASS

**Step 7: Commit**

```bash
git add fractalwonder-core/src/bigfloat.rs
git commit -m "feat(core): add BigFloat comparison and serialization"
```

---

## Stage 2: Core Types

### Task 5: Viewport Type

**Files:**
- Create: `fractalwonder-core/src/viewport.rs`

**Step 1: Write tests for Viewport**

Create `fractalwonder-core/src/viewport.rs`:

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BigFloat;

    #[test]
    fn test_viewport_creation() {
        let center = (
            BigFloat::with_precision(0.0, 128),
            BigFloat::with_precision(0.0, 128),
        );
        let width = BigFloat::with_precision(4.0, 128);
        let height = BigFloat::with_precision(3.0, 128);

        let viewport = Viewport::new(center, width, height);

        assert_eq!(viewport.center.0.to_f64(), 0.0);
        assert_eq!(viewport.width.to_f64(), 4.0);
    }

    #[test]
    fn test_viewport_serialization() {
        let center = (
            BigFloat::with_precision(-0.5, 256),
            BigFloat::with_precision(0.0, 256),
        );
        let width = BigFloat::with_precision(2.0, 256);
        let height = BigFloat::with_precision(1.5, 256);

        let viewport = Viewport::new(center, width, height);

        let json = serde_json::to_string(&viewport).expect("serialize failed");
        let deserialized: Viewport<BigFloat> = serde_json::from_str(&json)
            .expect("deserialize failed");

        assert_eq!(deserialized.center.0.to_f64(), -0.5);
        assert_eq!(deserialized.width.to_f64(), 2.0);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-core --lib viewport`
Expected: FAIL - Viewport not defined

**Step 3: Implement Viewport**

Add before tests:

```rust
/// Viewport defines visible region in fractal space
///
/// Uses width/height instead of zoom factor to support extreme zoom levels.
/// At 10^2000 zoom, width might be ~10^-2000 (representable in BigFloat).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Viewport<T> {
    pub center: (T, T),
    pub width: T,
    pub height: T,
}

impl<T> Viewport<T> {
    pub fn new(center: (T, T), width: T, height: T) -> Self {
        Self {
            center,
            width,
            height,
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core --lib viewport`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/viewport.rs
git commit -m "feat(core): add Viewport with width/height for extreme zoom"
```

### Task 6: PixelRect Type

**Files:**
- Create: `fractalwonder-core/src/pixel_rect.rs`

**Step 1: Write tests for PixelRect**

Create `fractalwonder-core/src/pixel_rect.rs`:

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_rect_creation() {
        let rect = PixelRect::new(10, 20, 100, 50);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 50);
    }

    #[test]
    fn test_full_canvas() {
        let rect = PixelRect::full_canvas(800, 600);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 800);
        assert_eq!(rect.height, 600);
    }

    #[test]
    fn test_pixel_count() {
        let rect = PixelRect::new(0, 0, 10, 20);
        assert_eq!(rect.pixel_count(), 200);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-core --lib pixel_rect`
Expected: FAIL - PixelRect not defined

**Step 3: Implement PixelRect**

Add before tests:

```rust
/// Rectangle in pixel space (canvas coordinates)
///
/// Always uses u32 - sufficient for screen dimensions.
/// Distinct from fractal-space coordinates which use BigFloat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Create rect covering entire canvas
    pub fn full_canvas(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Total number of pixels in this rect
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core --lib pixel_rect`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/pixel_rect.rs
git commit -m "feat(core): add PixelRect for pixel-space coordinates"
```

---

## Stage 3: Coordinate Transformations

### Task 7: pixel_to_fractal Transform

**Files:**
- Create: `fractalwonder-core/src/transforms.rs`

**Step 1: Write test for pixel_to_fractal**

Create `fractalwonder-core/src/transforms.rs`:

```rust
use crate::{BigFloat, Viewport};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_to_fractal_center() {
        // Viewport centered at (0, 0), width=4, height=3
        let viewport = Viewport::new(
            (BigFloat::with_precision(0.0, 128), BigFloat::with_precision(0.0, 128)),
            BigFloat::with_precision(4.0, 128),
            BigFloat::with_precision(3.0, 128),
        );

        // Center pixel of 800x600 canvas
        let (fx, fy) = pixel_to_fractal(400.0, 300.0, &viewport, (800, 600), 128);

        // Should map to viewport center (0, 0)
        assert!((fx.to_f64() - 0.0).abs() < 1e-10);
        assert!((fy.to_f64() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_pixel_to_fractal_corners() {
        let viewport = Viewport::new(
            (BigFloat::with_precision(0.0, 128), BigFloat::with_precision(0.0, 128)),
            BigFloat::with_precision(4.0, 128),
            BigFloat::with_precision(3.0, 128),
        );

        // Top-left pixel (0, 0)
        let (fx, fy) = pixel_to_fractal(0.0, 0.0, &viewport, (800, 600), 128);
        assert!((fx.to_f64() - (-2.0)).abs() < 1e-10); // -width/2
        assert!((fy.to_f64() - (-1.5)).abs() < 1e-10); // -height/2

        // Bottom-right pixel (800, 600)
        let (fx, fy) = pixel_to_fractal(800.0, 600.0, &viewport, (800, 600), 128);
        assert!((fx.to_f64() - 2.0).abs() < 1e-10); // +width/2
        assert!((fy.to_f64() - 1.5).abs() < 1e-10); // +height/2
    }

    #[test]
    fn test_pixel_to_fractal_precision() {
        let viewport = Viewport::new(
            (BigFloat::with_precision(0.0, 256), BigFloat::with_precision(0.0, 256)),
            BigFloat::with_precision(4.0, 256),
            BigFloat::with_precision(3.0, 256),
        );

        let (fx, _) = pixel_to_fractal(400.0, 300.0, &viewport, (800, 600), 256);

        // Verify result has requested precision
        assert_eq!(fx.precision_bits(), 256);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-core --lib transforms`
Expected: FAIL - pixel_to_fractal not defined

**Step 3: Implement pixel_to_fractal**

Add before tests:

```rust
/// Convert pixel coordinates to fractal-space coordinates
///
/// Maps canvas pixel (f64) to fractal coordinates (BigFloat).
/// Requires explicit precision_bits for result.
///
/// # Coordinate system
/// - Pixel (0, 0) = top-left corner
/// - Fractal space origin = viewport center
/// - Maps linearly: pixel position → fractal position
pub fn pixel_to_fractal(
    pixel_x: f64,
    pixel_y: f64,
    viewport: &Viewport<BigFloat>,
    canvas_size: (u32, u32),
    precision_bits: usize,
) -> (BigFloat, BigFloat) {
    let (canvas_width, canvas_height) = canvas_size;

    // Normalize pixel coords to [0, 1]
    let norm_x = pixel_x / canvas_width as f64;
    let norm_y = pixel_y / canvas_height as f64;

    // Convert to [-0.5, 0.5] range (centered)
    let centered_x = norm_x - 0.5;
    let centered_y = norm_y - 0.5;

    // Scale by viewport dimensions and add to center
    let offset_x = viewport.width.mul(&BigFloat::with_precision(centered_x, precision_bits));
    let offset_y = viewport.height.mul(&BigFloat::with_precision(centered_y, precision_bits));

    let fractal_x = viewport.center.0.add(&offset_x);
    let fractal_y = viewport.center.1.add(&offset_y);

    (fractal_x, fractal_y)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core --lib transforms`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-core/src/transforms.rs
git commit -m "feat(core): add pixel_to_fractal coordinate transformation"
```

### Task 8: TransformResult and apply_pixel_transform_to_viewport

**Files:**
- Modify: `fractalwonder-core/src/transforms.rs`

**Step 1: Write test for TransformResult**

Add to test module in `transforms.rs`:

```rust
#[test]
fn test_transform_result_creation() {
    let transform = TransformResult {
        offset_x: 100.0,
        offset_y: 50.0,
        zoom_factor: 2.0,
        matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    assert_eq!(transform.zoom_factor, 2.0);
}
```

**Step 2: Write test for apply_pixel_transform_to_viewport**

Add to test module:

```rust
#[test]
fn test_apply_zoom_only() {
    let viewport = Viewport::new(
        (BigFloat::with_precision(0.0, 128), BigFloat::with_precision(0.0, 128)),
        BigFloat::with_precision(4.0, 128),
        BigFloat::with_precision(3.0, 128),
    );

    let transform = TransformResult {
        offset_x: 0.0,
        offset_y: 0.0,
        zoom_factor: 2.0, // Zoom in 2x
        matrix: [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]],
    };

    let new_viewport = apply_pixel_transform_to_viewport(
        &viewport,
        &transform,
        (800, 600),
        128,
    );

    // Width/height should be halved (zoomed in 2x)
    assert!((new_viewport.width.to_f64() - 2.0).abs() < 1e-10);
    assert!((new_viewport.height.to_f64() - 1.5).abs() < 1e-10);

    // Center should be unchanged (zoom around center)
    assert!((new_viewport.center.0.to_f64() - 0.0).abs() < 1e-10);
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-core --lib transforms`
Expected: FAIL - types/functions not defined

**Step 4: Implement TransformResult**

Add before tests in `transforms.rs`:

```rust
/// Result from user interaction (from use_canvas_interaction hook)
///
/// Contains relative zoom factor and pixel offsets.
/// The zoom_factor is RELATIVE (f64 sufficient even at extreme zoom levels).
#[derive(Debug, Clone, PartialEq)]
pub struct TransformResult {
    /// Horizontal offset in pixels (center-relative)
    pub offset_x: f64,
    /// Vertical offset in pixels (center-relative)
    pub offset_y: f64,
    /// Relative zoom factor (2.0 = zoom in 2x from current)
    pub zoom_factor: f64,
    /// Affine transformation matrix (for rendering preview)
    pub matrix: [[f64; 3]; 3],
}
```

**Step 5: Implement apply_pixel_transform_to_viewport stub**

Add after pixel_to_fractal:

```rust
/// Apply user interaction transform to viewport
///
/// Converts pixel-space gestures into fractal-space viewport changes.
/// Handles both zoom and pan with correct fixed-point behavior.
pub fn apply_pixel_transform_to_viewport(
    viewport: &Viewport<BigFloat>,
    transform: &TransformResult,
    canvas_size: (u32, u32),
    precision_bits: usize,
) -> Viewport<BigFloat> {
    // Calculate new dimensions from relative zoom_factor
    let zoom_bf = BigFloat::with_precision(transform.zoom_factor, precision_bits);
    let new_width = viewport.width.div(&zoom_bf);
    let new_height = viewport.height.div(&zoom_bf);

    // For now, just handle zoom (pan calculation is complex)
    // TODO: Add pan calculation from offset_x/offset_y

    Viewport::new(
        viewport.center.clone(),
        new_width,
        new_height,
    )
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-core --lib transforms`
Expected: PASS (basic zoom test passes)

**Step 7: Commit**

```bash
git add fractalwonder-core/src/transforms.rs
git commit -m "feat(core): add TransformResult and basic apply_pixel_transform_to_viewport"
```

---

## Stage 4: Mandelbrot Computation

### Task 9: MandelbrotData Type

**Files:**
- Create: `fractalwonder-compute/src/mandelbrot.rs`

**Step 1: Write test for MandelbrotData**

Create `fractalwonder-compute/src/mandelbrot.rs`:

```rust
use fractalwonder_core::BigFloat;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandelbrot_data_creation() {
        let data = MandelbrotData {
            iterations: 42,
            escaped: true,
            z_magnitude: BigFloat::with_precision(4.5, 128),
        };

        assert_eq!(data.iterations, 42);
        assert_eq!(data.escaped, true);
        assert!((data.z_magnitude.to_f64() - 4.5).abs() < 1e-10);
    }

    #[test]
    fn test_mandelbrot_data_serialization() {
        let data = MandelbrotData {
            iterations: 100,
            escaped: false,
            z_magnitude: BigFloat::with_precision(1.5, 256),
        };

        let json = serde_json::to_string(&data).expect("serialize failed");
        let deserialized: MandelbrotData = serde_json::from_str(&json)
            .expect("deserialize failed");

        assert_eq!(deserialized.iterations, 100);
        assert_eq!(deserialized.escaped, false);
        assert_eq!(deserialized.z_magnitude.precision_bits(), 256);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-compute --lib mandelbrot`
Expected: FAIL - MandelbrotData not defined

**Step 3: Implement MandelbrotData**

Add before tests:

```rust
/// Computation result for a single Mandelbrot set point
///
/// Contains raw iteration data (NOT colors).
/// z_magnitude uses BigFloat for extreme zoom levels.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MandelbrotData {
    /// Number of iterations before escape (or max_iterations)
    pub iterations: u32,
    /// Whether the point escaped
    pub escaped: bool,
    /// Magnitude of z when escaped (or final magnitude if didn't escape)
    /// BigFloat for precision at extreme zoom levels
    pub z_magnitude: BigFloat,
}

impl Default for MandelbrotData {
    fn default() -> Self {
        Self {
            iterations: 0,
            escaped: false,
            z_magnitude: BigFloat::zero(64), // Default precision for empty data
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute --lib mandelbrot`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/mandelbrot.rs
git commit -m "feat(compute): add MandelbrotData type"
```

### Task 10: PointComputer Trait

**Files:**
- Create: `fractalwonder-compute/src/point_computer.rs`

**Step 1: Write test for PointComputer trait**

Create `fractalwonder-compute/src/point_computer.rs`:

```rust
use fractalwonder_core::BigFloat;

#[cfg(test)]
mod tests {
    use super::*;

    // Mock PointComputer for testing
    struct MockComputer {
        precision_bits: usize,
    }

    impl PointComputer for MockComputer {
        type Data = u32; // Just return iterations for test
        type Config = usize; // Just precision_bits

        fn configure(&mut self, config: Self::Config) {
            self.precision_bits = config;
        }

        fn precision_bits(&self) -> usize {
            self.precision_bits
        }

        fn compute(&self, _point: (BigFloat, BigFloat)) -> Self::Data {
            42 // Mock value
        }
    }

    #[test]
    fn test_point_computer_configure() {
        let mut computer = MockComputer { precision_bits: 128 };
        assert_eq!(computer.precision_bits(), 128);

        computer.configure(256);
        assert_eq!(computer.precision_bits(), 256);
    }

    #[test]
    fn test_point_computer_compute() {
        let computer = MockComputer { precision_bits: 128 };
        let point = (
            BigFloat::with_precision(0.0, 128),
            BigFloat::with_precision(0.0, 128),
        );

        let result = computer.compute(point);
        assert_eq!(result, 42);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-compute --lib point_computer`
Expected: FAIL - PointComputer trait not defined

**Step 3: Implement PointComputer trait**

Add before tests:

```rust
/// Trait for computing data for a single point in fractal space
///
/// Generic over Data (MandelbrotData, JuliaData, etc.) and Config.
/// Supports runtime configuration via configure() method.
pub trait PointComputer {
    /// Data type returned by compute (e.g., MandelbrotData)
    type Data: Clone + Send;

    /// Configuration type (e.g., max_iterations, precision_bits)
    type Config: Clone + Send;

    /// Reconfigure computer with new parameters
    fn configure(&mut self, config: Self::Config);

    /// Get current precision bits (needed for coordinate conversions)
    fn precision_bits(&self) -> usize;

    /// Compute data for a single point in fractal space
    fn compute(&self, point: (BigFloat, BigFloat)) -> Self::Data;
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute --lib point_computer`
Expected: PASS

**Step 5: Commit**

```bash
git add fractalwonder-compute/src/point_computer.rs
git commit -m "feat(compute): add PointComputer trait"
```

### Task 11: MandelbrotComputer Implementation

**Files:**
- Modify: `fractalwonder-compute/src/mandelbrot.rs`

**Step 1: Write test for MandelbrotComputer**

Add to test module in `mandelbrot.rs`:

```rust
use crate::PointComputer;

#[test]
fn test_mandelbrot_computer_creation() {
    let computer = MandelbrotComputer::new(256, 128);
    assert_eq!(computer.precision_bits(), 128);
}

#[test]
fn test_mandelbrot_computer_configure() {
    let mut computer = MandelbrotComputer::new(256, 128);

    let config = MandelbrotConfig {
        max_iterations: 512,
        precision_bits: 256,
    };

    computer.configure(config);
    assert_eq!(computer.precision_bits(), 256);
}

#[test]
fn test_mandelbrot_origin_is_inside() {
    // Point (0, 0) is inside the Mandelbrot set
    let computer = MandelbrotComputer::new(256, 128);
    let point = (
        BigFloat::with_precision(0.0, 128),
        BigFloat::with_precision(0.0, 128),
    );

    let data = computer.compute(point);

    assert_eq!(data.escaped, false);
    assert_eq!(data.iterations, 256);
}

#[test]
fn test_mandelbrot_far_point_escapes_quickly() {
    // Point (2, 2) is far outside, should escape in 1 iteration
    let computer = MandelbrotComputer::new(256, 128);
    let point = (
        BigFloat::with_precision(2.0, 128),
        BigFloat::with_precision(2.0, 128),
    );

    let data = computer.compute(point);

    assert_eq!(data.escaped, true);
    assert!(data.iterations < 5); // Should escape very quickly
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p fractalwonder-compute --lib mandelbrot`
Expected: FAIL - MandelbrotComputer/MandelbrotConfig not defined

**Step 3: Implement MandelbrotConfig**

Add before tests in `mandelbrot.rs`:

```rust
/// Configuration for Mandelbrot computation
#[derive(Clone, Debug)]
pub struct MandelbrotConfig {
    pub max_iterations: u32,
    pub precision_bits: usize,
}
```

**Step 4: Implement MandelbrotComputer structure**

Add after MandelbrotConfig:

```rust
/// Mandelbrot set point computer
///
/// Computes z = z^2 + c iteration with BigFloat throughout.
pub struct MandelbrotComputer {
    max_iterations: u32,
    precision_bits: usize,
}

impl MandelbrotComputer {
    pub fn new(max_iterations: u32, precision_bits: usize) -> Self {
        Self {
            max_iterations,
            precision_bits,
        }
    }
}
```

**Step 5: Implement PointComputer for MandelbrotComputer**

Add after MandelbrotComputer:

```rust
use crate::PointComputer;

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
        // z starts at (0, 0)
        let mut z = (
            BigFloat::zero(self.precision_bits),
            BigFloat::zero(self.precision_bits),
        );

        let mut iterations = 0;
        let threshold = BigFloat::with_precision(4.0, self.precision_bits);

        for _ in 0..self.max_iterations {
            // Calculate z^2 = (z.real^2 - z.imag^2, 2 * z.real * z.imag)
            let z_real_sq = z.0.mul(&z.0);
            let z_imag_sq = z.1.mul(&z.1);

            // Check escape before computing new z: |z|^2 = z.real^2 + z.imag^2 > 4
            let magnitude_sq = z_real_sq.add(&z_imag_sq);

            if magnitude_sq > threshold {
                // Escaped
                let magnitude = magnitude_sq.sqrt();
                return MandelbrotData {
                    iterations,
                    escaped: true,
                    z_magnitude: magnitude,
                };
            }

            // Compute new z = z^2 + c
            let new_real = z_real_sq.sub(&z_imag_sq).add(&c.0);

            let two = BigFloat::with_precision(2.0, self.precision_bits);
            let new_imag = two.mul(&z.0).mul(&z.1).add(&c.1);

            z = (new_real, new_imag);
            iterations += 1;
        }

        // Didn't escape
        let z_real_sq = z.0.mul(&z.0);
        let z_imag_sq = z.1.mul(&z.1);
        let magnitude_sq = z_real_sq.add(&z_imag_sq);
        let magnitude = magnitude_sq.sqrt();

        MandelbrotData {
            iterations,
            escaped: false,
            z_magnitude: magnitude,
        }
    }
}
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p fractalwonder-compute --lib mandelbrot`
Expected: PASS

**Step 7: Commit**

```bash
git add fractalwonder-compute/src/mandelbrot.rs
git commit -m "feat(compute): implement MandelbrotComputer with BigFloat"
```

---

## Stage 5: Stage 0 - Basic Web App Skeleton

(Due to length constraints, I'll provide the remaining tasks in abbreviated form. Each follows the same TDD pattern.)

### Task 12: Copy UI Components from Archive

**Files:**
- Copy: `_archive/fractalwonder-ui/src/components/ui.rs` → `fractalwonder-ui/src/components/ui.rs`
- Copy: `_archive/fractalwonder-ui/src/components/circular_progress.rs` → (if needed)
- Copy: `_archive/fractalwonder-ui/src/hooks/use_canvas_interaction.rs` → `fractalwonder-ui/src/hooks/use_canvas_interaction.rs`
- Copy: `_archive/fractalwonder-ui/src/hooks/fullscreen.rs` → `fractalwonder-ui/src/hooks/fullscreen.rs`
- Modify: Remove any dependencies on old types, update imports to new crate structure

**Step 1-5:** Copy files, update imports, verify compilation, commit

### Task 13: Basic App with Test Pattern Canvas

**Files:**
- Create: `fractalwonder-ui/src/app.rs`
- Create: `fractalwonder-ui/src/components/test_canvas.rs`

Implement basic Leptos app that:
- Renders canvas filling the screen
- Draws simple test pattern (gradient or checkerboard)
- Uses `use_canvas_interaction` hook
- On interaction end: redraw same pattern (no transform applied)

### Task 14: Manual Test - Stage 0 Complete

**Steps:**
1. Run: `trunk serve`
2. Open: http://localhost:8080
3. Verify: Test pattern visible
4. Verify: Can drag/zoom (preview works)
5. Verify: After interaction ends, pattern redraws
6. Commit: "feat: Stage 0 complete - basic web app with interaction"

---

## Stage 6: Worker Infrastructure

### Task 15: PixelRenderer Implementation
### Task 16: Worker Message Types
### Task 17: Worker Entry Point
### Task 18: Worker Pool on Main Thread
### Task 19: ParallelCanvasRenderer
### Task 20: Colorizer Functions
### Task 21: InteractiveCanvas Component
### Task 22: Full Integration Test

---

## Verification

**Final checks before completion:**

1. All tests pass: `cargo test --workspace --all-targets`
2. No warnings: `cargo clippy --workspace --all-targets -- -D warnings`
3. Formatted: `cargo fmt --all --check`
4. Builds for WASM: `trunk build --release`
5. Manual test: Open in browser, can explore Mandelbrot set
6. Deep zoom test: Zoom in 100+ times, verify no precision loss artifacts

---

## Notes for Engineer

- **DRY**: Reuse existing coordinate transform logic from archive where possible
- **YAGNI**: Don't add features not in this plan (no URL persistence, no bookmarks yet)
- **TDD**: Write test first, watch it fail, implement, watch it pass, commit
- **Commits**: Small, frequent commits after each passing test
- **Precision**: NEVER create BigFloat without explicit precision_bits
- **Ask**: If anything is unclear, ask before implementing
