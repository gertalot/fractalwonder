# Mandelbrot Renderer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Implement a generic Mandelbrot set renderer with smooth gradient coloring to replace the test renderer.

**Architecture:** Follow existing `TestImageRenderer` pattern with `MandelbrotRenderer<T>` implementing `ImagePointComputer` and `RendererInfo` traits. Use generic coordinate type `T` with proper trait bounds for element-wise operations. Escape-time algorithm with smooth iteration coloring mapped to HSV gradient.

**Tech Stack:** Rust + Leptos, generic types over `Point<T>`, existing rendering infrastructure

---

## Task 1: Add Element-Wise Multiplication to Point<T>

**Files:**
- Modify: `src/rendering/points.rs`

**Step 1: Write the failing test**

Add to the `#[cfg(test)]` module in `src/rendering/points.rs`:

```rust
#[test]
fn test_point_mul() {
    let p1 = Point::new(3.0, 4.0);
    let p2 = Point::new(2.0, 5.0);
    let product = p1.mul(&p2);
    assert_eq!(*product.x(), 6.0);
    assert_eq!(*product.y(), 20.0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib rendering::points::tests::test_point_mul`
Expected: FAIL with "no method named `mul`"

**Step 3: Write minimal implementation**

Add to `impl<T> Point<T>` in `src/rendering/points.rs` (before the test module):

```rust
pub fn mul(&self, other: &Self) -> Self
where
    T: Mul<Output = T> + Clone,
{
    Self {
        x: self.x.clone() * other.x.clone(),
        y: self.y.clone() * other.y.clone(),
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib rendering::points::tests::test_point_mul`
Expected: PASS

**Step 5: Run full test suite**

Run: `cargo test --lib rendering::points`
Expected: All tests PASS

**Step 6: Commit**

```bash
git add src/rendering/points.rs
git commit -m "feat: add element-wise multiplication to Point<T>"
```

---

## Task 2: Create Mandelbrot Module with Basic Structure

**Files:**
- Create: `src/components/mandelbrot.rs`
- Modify: `src/components/mod.rs`

**Step 1: Write the module declaration**

Add to `src/components/mod.rs`:

```rust
pub mod mandelbrot;
```

**Step 2: Create basic module structure**

Create `src/components/mandelbrot.rs` with:

```rust
use crate::rendering::{
    point_compute::ImagePointComputer,
    points::{Point, Rect},
    renderer_info::{RendererInfo, RendererInfoData},
    viewport::Viewport,
};

#[derive(Clone)]
pub struct MandelbrotRenderer<T> {
    max_iterations: u32,
    escape_radius_squared: f64,
}

impl<T> MandelbrotRenderer<T> {
    pub fn new() -> Self
    where
        T: From<f64>,
    {
        Self {
            max_iterations: 1000,
            escape_radius_squared: 4.0, // radius 2.0, squared
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_construction() {
        let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
        assert_eq!(renderer.max_iterations, 1000);
        assert_eq!(renderer.escape_radius_squared, 4.0);
    }
}
```

**Step 3: Run test to verify structure compiles**

Run: `cargo test --lib components::mandelbrot::tests::test_renderer_construction`
Expected: PASS

**Step 4: Commit**

```bash
git add src/components/mod.rs src/components/mandelbrot.rs
git commit -m "feat: create mandelbrot renderer module structure"
```

---

## Task 3: Implement Complex Number Helpers

**Files:**
- Modify: `src/components/mandelbrot.rs`

**Step 1: Write failing tests for complex arithmetic**

Add to test module in `src/components/mandelbrot.rs`:

```rust
#[test]
fn test_complex_square() {
    // (3 + 4i)^2 = 9 + 24i - 16 = -7 + 24i
    let z = Point::new(3.0, 4.0);
    let z_squared = complex_square(&z);
    assert_eq!(*z_squared.x(), -7.0);
    assert_eq!(*z_squared.y(), 24.0);
}

#[test]
fn test_magnitude_squared() {
    // |3 + 4i|^2 = 9 + 16 = 25
    let z = Point::new(3.0, 4.0);
    let mag_sq = magnitude_squared(&z);
    assert_eq!(mag_sq, 25.0);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib components::mandelbrot`
Expected: FAIL with "cannot find function `complex_square`"

**Step 3: Write minimal implementation**

Add before test module in `src/components/mandelbrot.rs`:

```rust
use std::ops::{Add, Mul};

/// Compute z^2 for complex number represented as Point
/// (a + bi)^2 = a^2 - b^2 + 2abi
fn complex_square<T>(z: &Point<T>) -> Point<T>
where
    T: Clone + Mul<Output = T> + Add<Output = T> + std::ops::Sub<Output = T> + From<f64>,
{
    let a = z.x().clone();
    let b = z.y().clone();
    let real = a.clone() * a.clone() - b.clone() * b.clone();
    let imag = a * b.clone() * T::from(2.0);
    Point::new(real, imag)
}

/// Compute |z|^2 for complex number
fn magnitude_squared<T>(z: &Point<T>) -> f64
where
    T: Clone + Mul<Output = T> + Add<Output = T>,
    f64: std::ops::Mul<T, Output = f64>,
{
    let x_sq = z.x().clone() * z.x().clone();
    let y_sq = z.y().clone() * z.y().clone();
    let sum = x_sq + y_sq;
    1.0 * sum
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib components::mandelbrot`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/components/mandelbrot.rs
git commit -m "feat: add complex number arithmetic helpers"
```

---

## Task 4: Implement Escape-Time Algorithm

**Files:**
- Modify: `src/components/mandelbrot.rs`

**Step 1: Write failing test**

Add to test module:

```rust
#[test]
fn test_escape_time_origin() {
    let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
    // Point (0, 0) is in the Mandelbrot set
    let c = Point::new(0.0, 0.0);
    let iterations = renderer.escape_time(&c);
    assert_eq!(iterations, renderer.max_iterations);
}

#[test]
fn test_escape_time_outside() {
    let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
    // Point (2, 2) escapes quickly
    let c = Point::new(2.0, 2.0);
    let iterations = renderer.escape_time(&c);
    assert!(iterations < 10);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib components::mandelbrot`
Expected: FAIL with "no method named `escape_time`"

**Step 3: Write minimal implementation**

Add to `impl<T> MandelbrotRenderer<T>`:

```rust
fn escape_time(&self, c: &Point<T>) -> u32
where
    T: Clone
        + From<f64>
        + Mul<Output = T>
        + Add<Output = T>
        + std::ops::Sub<Output = T>,
    f64: std::ops::Mul<T, Output = f64>,
{
    let mut z = Point::new(T::from(0.0), T::from(0.0));

    for iteration in 0..self.max_iterations {
        if magnitude_squared(&z) > self.escape_radius_squared {
            return iteration;
        }

        // z = z^2 + c
        z = complex_square(&z).add(c);
    }

    self.max_iterations
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib components::mandelbrot`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/components/mandelbrot.rs
git commit -m "feat: implement escape-time algorithm for Mandelbrot set"
```

---

## Task 5: Implement Smooth Coloring Function

**Files:**
- Modify: `src/components/mandelbrot.rs`

**Step 1: Write failing tests**

Add to test module:

```rust
#[test]
fn test_smooth_iteration_count() {
    let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
    let c = Point::new(0.5, 0.5);
    let (iterations, smooth) = renderer.smooth_iteration_count(&c);

    // Should escape (not in set)
    assert!(iterations < renderer.max_iterations);

    // Smooth value should be between iterations and iterations + 1
    assert!(smooth >= iterations as f64);
    assert!(smooth < (iterations + 2) as f64);
}

#[test]
fn test_smooth_iteration_count_in_set() {
    let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
    let c = Point::new(0.0, 0.0);
    let (iterations, smooth) = renderer.smooth_iteration_count(&c);

    // Point in set returns max iterations
    assert_eq!(iterations, renderer.max_iterations);
    assert_eq!(smooth, renderer.max_iterations as f64);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib components::mandelbrot`
Expected: FAIL with "no method named `smooth_iteration_count`"

**Step 3: Write minimal implementation**

Add to `impl<T> MandelbrotRenderer<T>`:

```rust
fn smooth_iteration_count(&self, c: &Point<T>) -> (u32, f64)
where
    T: Clone
        + From<f64>
        + Mul<Output = T>
        + Add<Output = T>
        + std::ops::Sub<Output = T>,
    f64: std::ops::Mul<T, Output = f64>,
{
    let mut z = Point::new(T::from(0.0), T::from(0.0));

    for iteration in 0..self.max_iterations {
        let mag_sq = magnitude_squared(&z);

        if mag_sq > self.escape_radius_squared {
            // Smooth iteration count using continuous coloring formula
            // smooth = iteration + 1 - log2(log(|z|) / log(escape_radius))
            let log_zn = (mag_sq.sqrt()).ln();
            let log_escape = self.escape_radius_squared.sqrt().ln();
            let smooth = iteration as f64 + 1.0 - (log_zn / log_escape).log2();
            return (iteration, smooth);
        }

        z = complex_square(&z).add(c);
    }

    (self.max_iterations, self.max_iterations as f64)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib components::mandelbrot`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/components/mandelbrot.rs
git commit -m "feat: add smooth iteration count for gradient coloring"
```

---

## Task 6: Implement HSV to RGB Color Conversion

**Files:**
- Modify: `src/components/mandelbrot.rs`

**Step 1: Write failing test**

Add to test module:

```rust
#[test]
fn test_hsv_to_rgb_red() {
    let (r, g, b) = hsv_to_rgb(0.0, 1.0, 1.0);
    assert_eq!(r, 255);
    assert_eq!(g, 0);
    assert_eq!(b, 0);
}

#[test]
fn test_hsv_to_rgb_green() {
    let (r, g, b) = hsv_to_rgb(120.0, 1.0, 1.0);
    assert_eq!(r, 0);
    assert_eq!(g, 255);
    assert_eq!(b, 0);
}

#[test]
fn test_hsv_to_rgb_blue() {
    let (r, g, b) = hsv_to_rgb(240.0, 1.0, 1.0);
    assert_eq!(r, 0);
    assert_eq!(g, 0);
    assert_eq!(b, 255);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib components::mandelbrot`
Expected: FAIL with "cannot find function `hsv_to_rgb`"

**Step 3: Write minimal implementation**

Add before test module:

```rust
/// Convert HSV color to RGB (0-255 range)
/// h: hue in degrees (0-360)
/// s: saturation (0-1)
/// v: value/brightness (0-1)
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());

    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let m = v - c;
    let r = ((r1 + m) * 255.0).round() as u8;
    let g = ((g1 + m) * 255.0).round() as u8;
    let b = ((b1 + m) * 255.0).round() as u8;

    (r, g, b)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib components::mandelbrot`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/components/mandelbrot.rs
git commit -m "feat: add HSV to RGB color conversion"
```

---

## Task 7: Implement ImagePointComputer Trait

**Files:**
- Modify: `src/components/mandelbrot.rs`

**Step 1: Write failing test**

Add to test module:

```rust
#[test]
fn test_natural_bounds() {
    let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
    let bounds = renderer.natural_bounds();
    assert_eq!(*bounds.min.x(), -2.5);
    assert_eq!(*bounds.min.y(), -1.25);
    assert_eq!(*bounds.max.x(), 1.0);
    assert_eq!(*bounds.max.y(), 1.25);
}

#[test]
fn test_compute_point_in_set() {
    let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
    // Point (0, 0) is in the set, should be black
    let color = renderer.compute(Point::new(0.0, 0.0));
    assert_eq!(color, (0, 0, 0, 255));
}

#[test]
fn test_compute_point_outside_set() {
    let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
    // Point (2, 0) escapes quickly, should have color
    let color = renderer.compute(Point::new(2.0, 0.0));
    assert_ne!(color, (0, 0, 0, 255));
    assert_eq!(color.3, 255); // Alpha should be 255
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib components::mandelbrot`
Expected: FAIL with "the trait bound `MandelbrotRenderer<f64>: ImagePointComputer` is not satisfied"

**Step 3: Write minimal implementation**

Add implementation after `impl<T> MandelbrotRenderer<T>`:

```rust
impl<T> ImagePointComputer for MandelbrotRenderer<T>
where
    T: Clone
        + From<f64>
        + Mul<Output = T>
        + Add<Output = T>
        + std::ops::Sub<Output = T>,
    f64: std::ops::Mul<T, Output = f64>,
{
    type Coord = T;

    fn natural_bounds(&self) -> Rect<T> {
        // Classic Mandelbrot view: real axis from -2.5 to 1.0, imaginary from -1.25 to 1.25
        Rect::new(
            Point::new(T::from(-2.5), T::from(-1.25)),
            Point::new(T::from(1.0), T::from(1.25)),
        )
    }

    fn compute(&self, coord: Point<T>) -> (u8, u8, u8, u8) {
        let (iterations, smooth) = self.smooth_iteration_count(&coord);

        // Points in the set are black
        if iterations == self.max_iterations {
            return (0, 0, 0, 255);
        }

        // Map smooth iteration to hue (0-360 degrees)
        // Cycle through spectrum multiple times for visual interest
        let hue = (smooth * 10.0) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);

        (r, g, b, 255)
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib components::mandelbrot`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/components/mandelbrot.rs
git commit -m "feat: implement ImagePointComputer trait for Mandelbrot renderer"
```

---

## Task 8: Implement RendererInfo Trait

**Files:**
- Modify: `src/components/mandelbrot.rs`

**Step 1: Write failing test**

Add to test module:

```rust
#[test]
fn test_renderer_info() {
    let renderer: MandelbrotRenderer<f64> = MandelbrotRenderer::new();
    let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
    let info = renderer.info(&viewport);

    assert_eq!(info.name, "Mandelbrot Set");
    assert!(info.center_display.contains("0.00"));
    assert!(info.zoom_display.contains("1.00"));
    assert_eq!(info.custom_params.len(), 2);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib components::mandelbrot`
Expected: FAIL with "the trait bound `MandelbrotRenderer<f64>: RendererInfo` is not satisfied"

**Step 3: Write minimal implementation**

Add implementation after `impl<T> ImagePointComputer`:

```rust
impl<T> RendererInfo for MandelbrotRenderer<T>
where
    T: Clone + std::fmt::Display,
{
    type Coord = T;

    fn info(&self, viewport: &Viewport<T>) -> RendererInfoData {
        RendererInfoData {
            name: "Mandelbrot Set".to_string(),
            center_display: format!(
                "x: {:.6}, y: {:.6}",
                viewport.center.x(),
                viewport.center.y()
            ),
            zoom_display: format!("{:.2}x", viewport.zoom),
            custom_params: vec![
                (
                    "Max iterations".to_string(),
                    format!("{}", self.max_iterations),
                ),
                (
                    "Escape radius²".to_string(),
                    format!("{:.1}", self.escape_radius_squared),
                ),
            ],
            render_time_ms: None,
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib components::mandelbrot`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/components/mandelbrot.rs
git commit -m "feat: implement RendererInfo trait for Mandelbrot renderer"
```

---

## Task 9: Create MandelbrotView Component

**Files:**
- Modify: `src/components/mandelbrot.rs`

**Step 1: Add Leptos imports**

Add to top of `src/components/mandelbrot.rs`:

```rust
use crate::components::interactive_canvas::InteractiveCanvas;
use crate::components::ui::UI;
use crate::hooks::fullscreen::toggle_fullscreen;
use crate::hooks::ui_visibility::use_ui_visibility;
use crate::rendering::PixelRenderer;
use leptos::*;
```

**Step 2: Write component implementation**

Add after all trait implementations:

```rust
#[component]
pub fn MandelbrotView() -> impl IntoView {
    let renderer = PixelRenderer::new(MandelbrotRenderer::<f64>::new());
    let canvas_with_info = InteractiveCanvas(renderer);

    let ui_visibility = use_ui_visibility();

    let reset_fn = canvas_with_info.reset_viewport;
    let on_home_click = move || {
        (reset_fn)();
    };

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

Run: `cargo check --lib`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/mandelbrot.rs
git commit -m "feat: add MandelbrotView component"
```

---

## Task 10: Run Full Test Suite and Format

**Files:**
- All modified files

**Step 1: Format code**

Run: `cargo fmt --all`
Expected: Code formatted

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests PASS

**Step 4: Check build**

Run: `cargo check --workspace --all-targets --all-features`
Expected: No errors

**Step 5: Commit if any formatting changes**

```bash
git add -u
git commit -m "chore: format and lint"
```

---

## Task 11: Manual Testing (Optional)

**Purpose:** Verify the Mandelbrot renderer displays correctly in the browser

**Note:** This requires updating the app routing to use `MandelbrotView` instead of `TestImageView`. This can be done in a follow-up task if needed.

**Steps:**
1. Ensure `trunk serve` is running
2. Update main app component to use `<MandelbrotView/>`
3. Open browser to `http://localhost:8080`
4. Verify classic Mandelbrot shape appears
5. Test zoom/pan interactions
6. Verify UI displays correct renderer info

---

## Completion Checklist

- [ ] All tests passing
- [ ] Code formatted (cargo fmt)
- [ ] No Clippy warnings
- [ ] All commits follow conventional commit format
- [ ] Generic type constraints compile correctly
- [ ] Mandelbrot renderer implements both required traits
- [ ] Component follows existing architecture pattern
