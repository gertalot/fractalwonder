# Robust Coordinate Types Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Refactor coordinate system to prevent precision loss by enforcing compile-time separation between pixel-space (f64) and image-space (generic Coord<T>) coordinates.

**Architecture:** Replace `PixelCoord` with plain f64, rename `ImageCoord<T>` to `Coord<T>`, implement explicit arithmetic methods (add, sub, mul_scalar, div_scalar) instead of operator overloading for clarity and better error messages.

**Tech Stack:** Rust, Leptos, WASM, existing test infrastructure

---

## Task 1: Rewrite coords.rs with new Coord<T> type

**Files:**
- Modify: `src/rendering/coords.rs` (complete rewrite)

**Step 1: Write failing tests for new Coord type**

Add to `src/rendering/coords.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_construction_and_accessors() {
        let coord = Coord::new(10.5, 20.5);
        assert_eq!(*coord.x(), 10.5);
        assert_eq!(*coord.y(), 20.5);
    }

    #[test]
    fn test_coord_into_parts() {
        let coord = Coord::new(10.5, 20.5);
        let (x, y) = coord.into_parts();
        assert_eq!(x, 10.5);
        assert_eq!(y, 20.5);
    }

    #[test]
    fn test_coord_add() {
        let c1 = Coord::new(1.0, 2.0);
        let c2 = Coord::new(3.0, 4.0);
        let sum = c1.add(&c2);
        assert_eq!(*sum.x(), 4.0);
        assert_eq!(*sum.y(), 6.0);
    }

    #[test]
    fn test_coord_sub() {
        let c1 = Coord::new(5.0, 7.0);
        let c2 = Coord::new(2.0, 3.0);
        let diff = c1.sub(&c2);
        assert_eq!(*diff.x(), 3.0);
        assert_eq!(*diff.y(), 4.0);
    }

    #[test]
    fn test_coord_mul_scalar() {
        let c = Coord::new(2.0, 3.0);
        let scaled = c.mul_scalar(2.5);
        assert_eq!(*scaled.x(), 5.0);
        assert_eq!(*scaled.y(), 7.5);
    }

    #[test]
    fn test_coord_div_scalar() {
        let c = Coord::new(10.0, 20.0);
        let divided = c.div_scalar(2.0);
        assert_eq!(*divided.x(), 5.0);
        assert_eq!(*divided.y(), 10.0);
    }

    #[test]
    fn test_coord_generic_with_i32() {
        let coord_f64 = Coord::new(10.5, 20.5);
        let coord_i32 = Coord::new(10, 20);
        assert_eq!(*coord_f64.x(), 10.5);
        assert_eq!(*coord_i32.x(), 10);
    }

    #[test]
    fn test_coord_precision_maintained() {
        let coord = Coord::new(1.0, 2.0);
        let scaled = coord.mul_scalar(3.0);
        let divided = scaled.div_scalar(3.0);
        assert_eq!(*divided.x(), 1.0);
        assert_eq!(*divided.y(), 2.0);
    }

    #[test]
    fn test_rect_construction() {
        let rect = Rect::new(
            Coord::new(0.0, 0.0),
            Coord::new(100.0, 50.0)
        );
        assert_eq!(*rect.min.x(), 0.0);
        assert_eq!(*rect.max.x(), 100.0);
    }

    #[test]
    fn test_rect_dimensions() {
        let rect = Rect::new(
            Coord::new(0.0, 0.0),
            Coord::new(100.0, 50.0)
        );
        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 50.0);
    }

    #[test]
    fn test_rect_generic_with_i32() {
        let rect_f64 = Rect::new(
            Coord::new(0.0, 0.0),
            Coord::new(100.0, 50.0)
        );
        let rect_i32 = Rect::new(
            Coord::new(0, 0),
            Coord::new(100, 50)
        );
        assert_eq!(rect_f64.width(), 100.0);
        assert_eq!(rect_i32.width(), 100);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --package fractalwonder --lib rendering::coords::tests`

Expected: FAIL with compilation errors (types don't exist yet)

**Step 3: Implement new Coord<T> and Rect<T> types**

Replace entire contents of `src/rendering/coords.rs` with:

```rust
use std::ops::{Add, Sub, Mul, Div};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord<T> {
    x: T,
    y: T,
}

impl<T> Coord<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> &T {
        &self.x
    }

    pub fn y(&self) -> &T {
        &self.y
    }

    pub fn into_parts(self) -> (T, T) {
        (self.x, self.y)
    }

    pub fn add(&self, other: &Self) -> Self
    where
        T: Add<Output = T> + Clone,
    {
        Self {
            x: self.x.clone() + other.x.clone(),
            y: self.y.clone() + other.y.clone(),
        }
    }

    pub fn sub(&self, other: &Self) -> Self
    where
        T: Sub<Output = T> + Clone,
    {
        Self {
            x: self.x.clone() - other.x.clone(),
            y: self.y.clone() - other.y.clone(),
        }
    }

    pub fn mul_scalar(&self, scalar: f64) -> Self
    where
        T: Mul<f64, Output = T> + Clone,
    {
        Self {
            x: self.x.clone() * scalar,
            y: self.y.clone() * scalar,
        }
    }

    pub fn div_scalar(&self, scalar: f64) -> Self
    where
        T: Div<f64, Output = T> + Clone,
    {
        Self {
            x: self.x.clone() / scalar,
            y: self.y.clone() / scalar,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rect<T> {
    pub min: Coord<T>,
    pub max: Coord<T>,
}

impl<T> Rect<T> {
    pub fn new(min: Coord<T>, max: Coord<T>) -> Self {
        Self { min, max }
    }

    pub fn width(&self) -> T
    where
        T: Sub<Output = T> + Clone,
    {
        self.max.x().clone() - self.min.x().clone()
    }

    pub fn height(&self) -> T
    where
        T: Sub<Output = T> + Clone,
    {
        self.max.y().clone() - self.min.y().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_construction_and_accessors() {
        let coord = Coord::new(10.5, 20.5);
        assert_eq!(*coord.x(), 10.5);
        assert_eq!(*coord.y(), 20.5);
    }

    #[test]
    fn test_coord_into_parts() {
        let coord = Coord::new(10.5, 20.5);
        let (x, y) = coord.into_parts();
        assert_eq!(x, 10.5);
        assert_eq!(y, 20.5);
    }

    #[test]
    fn test_coord_add() {
        let c1 = Coord::new(1.0, 2.0);
        let c2 = Coord::new(3.0, 4.0);
        let sum = c1.add(&c2);
        assert_eq!(*sum.x(), 4.0);
        assert_eq!(*sum.y(), 6.0);
    }

    #[test]
    fn test_coord_sub() {
        let c1 = Coord::new(5.0, 7.0);
        let c2 = Coord::new(2.0, 3.0);
        let diff = c1.sub(&c2);
        assert_eq!(*diff.x(), 3.0);
        assert_eq!(*diff.y(), 4.0);
    }

    #[test]
    fn test_coord_mul_scalar() {
        let c = Coord::new(2.0, 3.0);
        let scaled = c.mul_scalar(2.5);
        assert_eq!(*scaled.x(), 5.0);
        assert_eq!(*scaled.y(), 7.5);
    }

    #[test]
    fn test_coord_div_scalar() {
        let c = Coord::new(10.0, 20.0);
        let divided = c.div_scalar(2.0);
        assert_eq!(*divided.x(), 5.0);
        assert_eq!(*divided.y(), 10.0);
    }

    #[test]
    fn test_coord_generic_with_i32() {
        let coord_f64 = Coord::new(10.5, 20.5);
        let coord_i32 = Coord::new(10, 20);
        assert_eq!(*coord_f64.x(), 10.5);
        assert_eq!(*coord_i32.x(), 10);
    }

    #[test]
    fn test_coord_precision_maintained() {
        let coord = Coord::new(1.0, 2.0);
        let scaled = coord.mul_scalar(3.0);
        let divided = scaled.div_scalar(3.0);
        assert_eq!(*divided.x(), 1.0);
        assert_eq!(*divided.y(), 2.0);
    }

    #[test]
    fn test_rect_construction() {
        let rect = Rect::new(
            Coord::new(0.0, 0.0),
            Coord::new(100.0, 50.0)
        );
        assert_eq!(*rect.min.x(), 0.0);
        assert_eq!(*rect.max.x(), 100.0);
    }

    #[test]
    fn test_rect_dimensions() {
        let rect = Rect::new(
            Coord::new(0.0, 0.0),
            Coord::new(100.0, 50.0)
        );
        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 50.0);
    }

    #[test]
    fn test_rect_generic_with_i32() {
        let rect_f64 = Rect::new(
            Coord::new(0.0, 0.0),
            Coord::new(100.0, 50.0)
        );
        let rect_i32 = Rect::new(
            Coord::new(0, 0),
            Coord::new(100, 50)
        );
        assert_eq!(rect_f64.width(), 100.0);
        assert_eq!(rect_i32.width(), 100);
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --package fractalwonder --lib rendering::coords::tests`

Expected: All 11 tests PASS

**Step 5: Commit**

```bash
git add src/rendering/coords.rs
git commit -m "refactor: implement new Coord<T> and Rect<T> types with explicit arithmetic

- Replace PixelCoord and ImageCoord with new Coord<T>
- Replace ImageRect with Rect<T>
- Add explicit arithmetic methods: add, sub, mul_scalar, div_scalar
- Prevent accidental precision loss with type safety
- Add comprehensive tests for f64 and i32 types"
```

---

## Task 2: Update mod.rs exports

**Files:**
- Modify: `src/rendering/mod.rs`

**Step 1: Update exports to use new type names**

In `src/rendering/mod.rs`, change:

```rust
pub mod coords;
pub use coords::{ImageCoord, ImageRect, PixelCoord};
```

To:

```rust
pub mod coords;
pub use coords::{Coord, Rect};
```

**Step 2: Verify compilation**

Run: `cargo check --package fractalwonder`

Expected: FAIL with errors in transforms.rs, viewport.rs, test_image.rs (they still use old types)

**Step 3: Commit**

```bash
git add src/rendering/mod.rs
git commit -m "refactor: update exports to use new Coord and Rect types"
```

---

## Task 3: Update viewport.rs

**Files:**
- Modify: `src/rendering/viewport.rs`

**Step 1: Update imports and type references**

In `src/rendering/viewport.rs`, change line 1:

```rust
use crate::rendering::coords::{ImageCoord, ImageRect};
```

To:

```rust
use crate::rendering::coords::{Coord, Rect};
```

Change lines 4-7:

```rust
pub struct Viewport<T> {
    pub center: ImageCoord<T>,
    pub zoom: f64,
    pub natural_bounds: ImageRect<T>,
}
```

To:

```rust
pub struct Viewport<T> {
    pub center: Coord<T>,
    pub zoom: f64,
    pub natural_bounds: Rect<T>,
}
```

Change line 11:

```rust
pub fn new(center: ImageCoord<T>, zoom: f64, natural_bounds: ImageRect<T>) -> Self {
```

To:

```rust
pub fn new(center: Coord<T>, zoom: f64, natural_bounds: Rect<T>) -> Self {
```

**Step 2: Update tests to use new type names**

In the tests section, change all occurrences:
- `ImageCoord::new` → `Coord::new`
- `ImageRect::new` → `Rect::new`

**Step 3: Verify tests pass**

Run: `cargo test --package fractalwonder --lib rendering::viewport::tests`

Expected: All 2 tests PASS

**Step 4: Commit**

```bash
git add src/rendering/viewport.rs
git commit -m "refactor: update viewport to use Coord and Rect types"
```

---

## Task 4: Refactor transforms.rs - Part 1 (pixel_to_image)

**Files:**
- Modify: `src/rendering/transforms.rs`

**Step 1: Update imports**

In `src/rendering/transforms.rs`, change lines 1-4:

```rust
use crate::rendering::{
    coords::{ImageCoord, ImageRect, PixelCoord},
    viewport::Viewport,
};
```

To:

```rust
use crate::rendering::{
    coords::{Coord, Rect},
    viewport::Viewport,
};
```

**Step 2: Refactor pixel_to_image function**

Change the function signature and implementation at lines 52-71:

```rust
pub fn pixel_to_image<T>(
    pixel: PixelCoord,
    target_rect: &ImageRect<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> ImageCoord<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Mul<f64, Output = T>
        + std::ops::Add<Output = T>,
{
    let bounds_width = target_rect.max.x().clone() - target_rect.min.x().clone();
    let bounds_height = target_rect.max.y().clone() - target_rect.min.y().clone();

    ImageCoord::new(
        target_rect.min.x().clone() + bounds_width * (pixel.x() / canvas_width as f64),
        target_rect.min.y().clone() + bounds_height * (pixel.y() / canvas_height as f64),
    )
}
```

To:

```rust
pub fn pixel_to_image<T>(
    pixel_x: f64,
    pixel_y: f64,
    target_rect: &Rect<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> Coord<T>
where
    T: Clone + std::ops::Mul<f64, Output = T>,
{
    let bounds_width = target_rect.width();
    let bounds_height = target_rect.height();

    Coord::new(
        target_rect.min.x().clone() + bounds_width * (pixel_x / canvas_width as f64),
        target_rect.min.y().clone() + bounds_height * (pixel_y / canvas_height as f64),
    )
}
```

**Step 3: Verify compilation**

Run: `cargo check --package fractalwonder`

Expected: FAIL with errors in remaining functions and tests

**Step 4: Commit**

```bash
git add src/rendering/transforms.rs
git commit -m "refactor: update pixel_to_image to use f64 and new Coord type"
```

---

## Task 5: Refactor transforms.rs - Part 2 (image_to_pixel)

**Files:**
- Modify: `src/rendering/transforms.rs`

**Step 1: Refactor image_to_pixel function**

Change the function signature and implementation at lines 73-93:

```rust
pub fn image_to_pixel<T>(
    image: &ImageCoord<T>,
    target_rect: &ImageRect<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> PixelCoord
where
    T: Clone + std::ops::Sub<Output = T> + std::ops::Div<Output = T>,
    f64: std::ops::Mul<T, Output = f64>,
{
    let bounds_width = target_rect.max.x().clone() - target_rect.min.x().clone();
    let bounds_height = target_rect.max.y().clone() - target_rect.min.y().clone();

    let normalized_x = (image.x().clone() - target_rect.min.x().clone()) / bounds_width;
    let normalized_y = (image.y().clone() - target_rect.min.y().clone()) / bounds_height;

    PixelCoord::new(
        canvas_width as f64 * normalized_x,
        canvas_height as f64 * normalized_y,
    )
}
```

To:

```rust
pub fn image_to_pixel<T>(
    image: &Coord<T>,
    target_rect: &Rect<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> (f64, f64)
where
    T: Clone + std::ops::Sub<Output = T> + std::ops::Div<Output = T>,
    f64: std::ops::Mul<T, Output = f64>,
{
    let bounds_width = target_rect.width();
    let bounds_height = target_rect.height();

    let normalized_x = (image.x().clone() - target_rect.min.x().clone()) / bounds_width;
    let normalized_y = (image.y().clone() - target_rect.min.y().clone()) / bounds_height;

    (
        canvas_width as f64 * normalized_x,
        canvas_height as f64 * normalized_y,
    )
}
```

**Step 2: Verify compilation**

Run: `cargo check --package fractalwonder`

Expected: FAIL with errors in calculate_visible_bounds and tests

**Step 3: Commit**

```bash
git add src/rendering/transforms.rs
git commit -m "refactor: update image_to_pixel to return (f64, f64) tuple"
```

---

## Task 6: Refactor transforms.rs - Part 3 (calculate_visible_bounds)

**Files:**
- Modify: `src/rendering/transforms.rs`

**Step 1: Refactor calculate_visible_bounds to use Coord arithmetic**

Change the function at lines 6-50:

```rust
pub fn calculate_visible_bounds<T>(
    viewport: &Viewport<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> ImageRect<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Div<f64, Output = T>
        + std::ops::Mul<f64, Output = T>
        + std::ops::Add<Output = T>,
{
    let natural_width =
        viewport.natural_bounds.max.x().clone() - viewport.natural_bounds.min.x().clone();
    let natural_height =
        viewport.natural_bounds.max.y().clone() - viewport.natural_bounds.min.y().clone();

    // Apply zoom (1.0 = show entire natural bounds)
    let view_width = natural_width / viewport.zoom;
    let view_height = natural_height / viewport.zoom;

    // Adjust for canvas aspect ratio - extend the wider dimension
    let canvas_aspect = canvas_width as f64 / canvas_height as f64;

    // Simplified: assume T can be multiplied by f64
    let (final_width, final_height) = if canvas_aspect > 1.0 {
        // Landscape - extend width
        (view_height.clone() * canvas_aspect, view_height)
    } else {
        // Portrait - extend height
        (view_width.clone(), view_width / canvas_aspect)
    };

    // Calculate bounds centered on viewport.center
    ImageRect::new(
        ImageCoord::new(
            viewport.center.x().clone() - final_width.clone() / 2.0,
            viewport.center.y().clone() - final_height.clone() / 2.0,
        ),
        ImageCoord::new(
            viewport.center.x().clone() + final_width / 2.0,
            viewport.center.y().clone() + final_height / 2.0,
        ),
    )
}
```

To:

```rust
pub fn calculate_visible_bounds<T>(
    viewport: &Viewport<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> Rect<T>
where
    T: Clone + std::ops::Div<f64, Output = T> + std::ops::Mul<f64, Output = T>,
{
    let natural_width = viewport.natural_bounds.width();
    let natural_height = viewport.natural_bounds.height();

    // Apply zoom (1.0 = show entire natural bounds)
    let view_width = natural_width / viewport.zoom;
    let view_height = natural_height / viewport.zoom;

    // Adjust for canvas aspect ratio - extend the wider dimension
    let canvas_aspect = canvas_width as f64 / canvas_height as f64;

    let (final_width, final_height) = if canvas_aspect > 1.0 {
        // Landscape - extend width
        (view_height.clone() * canvas_aspect, view_height)
    } else {
        // Portrait - extend height
        (view_width.clone(), view_width / canvas_aspect)
    };

    // Calculate bounds centered on viewport.center
    let half_width = final_width.clone() / 2.0;
    let half_height = final_height.clone() / 2.0;

    Rect::new(
        Coord::new(
            viewport.center.x().clone() - half_width.clone(),
            viewport.center.y().clone() - half_height.clone(),
        ),
        Coord::new(
            viewport.center.x().clone() + half_width,
            viewport.center.y().clone() + half_height,
        ),
    )
}
```

**Step 2: Verify compilation**

Run: `cargo check --package fractalwonder`

Expected: FAIL with errors only in tests now

**Step 3: Commit**

```bash
git add src/rendering/transforms.rs
git commit -m "refactor: update calculate_visible_bounds to use new Coord/Rect types"
```

---

## Task 7: Update transforms.rs tests

**Files:**
- Modify: `src/rendering/transforms.rs` (tests section)

**Step 1: Update all test imports and type names**

In the tests section (lines 95-199), update:
- `ImageCoord::new` → `Coord::new`
- `ImageRect::new` → `Rect::new`
- `PixelCoord::new(...)` → direct f64 values

**Step 2: Update test_pixel_to_image_center**

Change lines 154-160:

```rust
#[test]
fn test_pixel_to_image_center() {
    let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
    let pixel = PixelCoord::new(500.0, 500.0); // Center of 1000x1000 canvas
    let image = pixel_to_image(pixel, &bounds, 1000, 1000);

    assert_eq!(*image.x(), 0.0);
    assert_eq!(*image.y(), 0.0);
}
```

To:

```rust
#[test]
fn test_pixel_to_image_center() {
    let bounds = Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0));
    let image = pixel_to_image(500.0, 500.0, &bounds, 1000, 1000);

    assert_eq!(*image.x(), 0.0);
    assert_eq!(*image.y(), 0.0);
}
```

**Step 3: Update test_pixel_to_image_corners**

Change lines 163-176:

```rust
#[test]
fn test_pixel_to_image_corners() {
    let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));

    // Top-left corner
    let image = pixel_to_image(PixelCoord::new(0.0, 0.0), &bounds, 1000, 1000);
    assert_eq!(*image.x(), -50.0);
    assert_eq!(*image.y(), -50.0);

    // Bottom-right corner
    let image = pixel_to_image(PixelCoord::new(1000.0, 1000.0), &bounds, 1000, 1000);
    assert_eq!(*image.x(), 50.0);
    assert_eq!(*image.y(), 50.0);
}
```

To:

```rust
#[test]
fn test_pixel_to_image_corners() {
    let bounds = Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0));

    // Top-left corner
    let image = pixel_to_image(0.0, 0.0, &bounds, 1000, 1000);
    assert_eq!(*image.x(), -50.0);
    assert_eq!(*image.y(), -50.0);

    // Bottom-right corner
    let image = pixel_to_image(1000.0, 1000.0, &bounds, 1000, 1000);
    assert_eq!(*image.x(), 50.0);
    assert_eq!(*image.y(), 50.0);
}
```

**Step 4: Update test_image_to_pixel_center**

Change lines 178-185:

```rust
#[test]
fn test_image_to_pixel_center() {
    let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
    let image = ImageCoord::new(0.0, 0.0);
    let pixel = image_to_pixel(&image, &bounds, 1000, 1000);

    assert_eq!(pixel.x(), 500.0);
    assert_eq!(pixel.y(), 500.0);
}
```

To:

```rust
#[test]
fn test_image_to_pixel_center() {
    let bounds = Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0));
    let image = Coord::new(0.0, 0.0);
    let (px, py) = image_to_pixel(&image, &bounds, 1000, 1000);

    assert_eq!(px, 500.0);
    assert_eq!(py, 500.0);
}
```

**Step 5: Update test_round_trip_pixel_image_pixel**

Change lines 188-198:

```rust
#[test]
fn test_round_trip_pixel_image_pixel() {
    let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
    let original = PixelCoord::new(123.0, 456.0);

    let image = pixel_to_image(original, &bounds, 1000, 1000);
    let result = image_to_pixel(&image, &bounds, 1000, 1000);

    assert!((result.x() - original.x()).abs() < 0.001);
    assert!((result.y() - original.y()).abs() < 0.001);
}
```

To:

```rust
#[test]
fn test_round_trip_pixel_image_pixel() {
    let bounds = Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0));
    let (orig_x, orig_y) = (123.0, 456.0);

    let image = pixel_to_image(orig_x, orig_y, &bounds, 1000, 1000);
    let (result_x, result_y) = image_to_pixel(&image, &bounds, 1000, 1000);

    assert!((result_x - orig_x).abs() < 0.001);
    assert!((result_y - orig_y).abs() < 0.001);
}
```

**Step 6: Update viewport construction tests (lines 100-135)**

Change all:
- `ImageCoord::new` → `Coord::new`
- `ImageRect::new` → `Rect::new`

**Step 7: Run tests**

Run: `cargo test --package fractalwonder --lib rendering::transforms::tests`

Expected: All 6 tests PASS

**Step 8: Commit**

```bash
git add src/rendering/transforms.rs
git commit -m "refactor: update transforms tests to use new coordinate types"
```

---

## Task 8: Update test_image.rs imports and usage

**Files:**
- Modify: `src/components/test_image.rs`

**Step 1: Update imports**

Change lines 2-6:

```rust
use crate::rendering::{
    coords::{ImageCoord, ImageRect, PixelCoord},
    renderer_trait::CanvasRenderer,
    transforms::pixel_to_image,
};
```

To:

```rust
use crate::rendering::{
    coords::{Coord, Rect},
    renderer_trait::CanvasRenderer,
    transforms::pixel_to_image,
};
```

**Step 2: Update CanvasRenderer trait implementation**

Change line 51:

```rust
fn natural_bounds(&self) -> ImageRect<f64> {
    ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0))
}
```

To:

```rust
fn natural_bounds(&self) -> Rect<f64> {
    Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0))
}
```

**Step 3: Update render method**

Change line 55:

```rust
fn render(&self, target_rect: &ImageRect<f64>, width: u32, height: u32) -> Vec<u8> {
```

To:

```rust
fn render(&self, target_rect: &Rect<f64>, width: u32, height: u32) -> Vec<u8> {
```

Change lines 60-62:

```rust
// Map pixel to image coordinates using centralized transform
let pixel = PixelCoord::new(px as f64, py as f64);
let image_coord = pixel_to_image(pixel, target_rect, width, height);
```

To:

```rust
// Map pixel to image coordinates using centralized transform
let image_coord = pixel_to_image(px as f64, py as f64, target_rect, width, height);
```

**Step 4: Update render_test_pattern function**

Change line 225:

```rust
let viewport = Viewport {
    center: ImageCoord::new(0.0, 0.0),
    zoom: 1.0,
    natural_bounds: bounds,
};
```

To:

```rust
let viewport = Viewport {
    center: Coord::new(0.0, 0.0),
    zoom: 1.0,
    natural_bounds: bounds,
};
```

**Step 5: Update test functions**

In test section, change line 264:

```rust
let bounds = ImageRect::new(ImageCoord::new(-10.0, -10.0), ImageCoord::new(10.0, 10.0));
```

To:

```rust
let bounds = Rect::new(Coord::new(-10.0, -10.0), Coord::new(10.0, 10.0));
```

**Step 6: Run tests**

Run: `cargo test --package fractalwonder --lib components::test_image::tests`

Expected: All 5 tests PASS

**Step 7: Commit**

```bash
git add src/components/test_image.rs
git commit -m "refactor: update test_image to use new coordinate types and f64 pixels"
```

---

## Task 9: Update renderer_trait.rs

**Files:**
- Modify: `src/rendering/renderer_trait.rs`

**Step 1: Check current imports and update if needed**

Run: `cat src/rendering/renderer_trait.rs`

If it imports `ImageRect`, change to `Rect`. If it imports `ImageCoord`, change to `Coord`.

Expected signature in trait:

```rust
fn natural_bounds(&self) -> Rect<Self::Coord>;
fn render(&self, target_rect: &Rect<Self::Coord>, width: u32, height: u32) -> Vec<u8>;
```

**Step 2: Update imports**

If the file contains:

```rust
use crate::rendering::coords::ImageRect;
```

Change to:

```rust
use crate::rendering::coords::Rect;
```

**Step 3: Update trait definition**

If the file contains references to `ImageRect<Self::Coord>`, change to `Rect<Self::Coord>`.

**Step 4: Verify compilation**

Run: `cargo check --package fractalwonder`

Expected: SUCCESS (no errors)

**Step 5: Commit**

```bash
git add src/rendering/renderer_trait.rs
git commit -m "refactor: update renderer trait to use Rect type"
```

---

## Task 10: Run full test suite

**Files:**
- None (verification only)

**Step 1: Format code**

Run: `cargo fmt --all`

Expected: Code formatted successfully

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Expected: No warnings or errors

If there are clippy warnings, fix them before proceeding.

**Step 3: Run cargo check**

Run: `cargo check --workspace --all-targets --all-features`

Expected: SUCCESS

**Step 4: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: All tests PASS

**Step 5: Run WASM browser tests**

Run: `wasm-pack test --headless --chrome`

Expected: All tests PASS

**Step 6: Build release**

Run: `trunk build --release`

Expected: Build succeeds, outputs to dist/

---

## Task 11: Browser testing with chrome-devtools

**Files:**
- None (manual browser testing)

**Step 1: Ensure trunk serve is running**

Verify trunk is serving at http://localhost:8080

**Step 2: Navigate to application**

Use chrome-devtools MCP to navigate to http://localhost:8080

**Step 3: Take initial screenshot**

Take screenshot of the rendered test pattern

Expected: Checkerboard pattern with red circles visible

**Step 4: Test pan interaction**

Simulate drag interaction on canvas

Expected: "Interacting..." indicator appears, canvas pans smoothly

**Step 5: Test zoom interaction**

Simulate mouse wheel zoom

Expected: "Interacting..." indicator appears, canvas zooms at cursor position

**Step 6: Check console for errors**

List console messages

Expected: No errors, only the "Interaction ended" messages with transformation data

**Step 7: Verify visual correctness**

Take final screenshot after interactions

Expected: Pattern still renders correctly, no visual artifacts or precision issues

---

## Task 12: Final cleanup and documentation

**Files:**
- Modify: `docs/pixel-image-coords-idea.md`

**Step 1: Update documentation to reflect completion**

Add to the end of `docs/pixel-image-coords-idea.md`:

```markdown

---

## Implementation Complete

**Date:** 2025-10-19

**Implementation Plan:** [docs/plans/2025-10-19-robust-coordinate-types.md](plans/2025-10-19-robust-coordinate-types.md)

**Changes:**
- ✅ Removed `PixelCoord` - replaced with plain f64
- ✅ Renamed `ImageCoord<T>` to `Coord<T>`
- ✅ Renamed `ImageRect<T>` to `Rect<T>`
- ✅ Implemented arithmetic operations: add, sub, mul_scalar, div_scalar
- ✅ All calculations in image space use `Coord<T>` types
- ✅ All calculations in pixel space use plain f64
- ✅ Thoroughly cleaned up codebase - no legacy code remains

**Files Modified:**
- src/rendering/coords.rs (complete rewrite)
- src/rendering/mod.rs
- src/rendering/viewport.rs
- src/rendering/transforms.rs
- src/rendering/renderer_trait.rs
- src/components/test_image.rs

**Testing:**
- All unit tests passing
- All integration tests passing
- WASM browser tests passing
- Manual browser testing with chrome-devtools verified
```

**Step 2: Commit**

```bash
git add docs/pixel-image-coords-idea.md
git commit -m "docs: mark coordinate type refactor as complete"
```

---

## Task 13: Create final PR or merge

**Files:**
- None (git operations only)

**Step 1: Review all changes**

Run: `git log --oneline develop..HEAD`

Expected: See all 13 commits from this implementation

**Step 2: Run final verification**

Run all tests one more time:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features -- --nocapture
wasm-pack test --headless --chrome
```

Expected: All pass with no warnings

**Step 3: Decide on integration strategy**

Options:
1. Merge directly to develop (if working in main project directory)
2. Create PR if on feature branch

If creating PR, follow CLAUDE.md instructions for PR creation.

**Step 4: Clean up**

If feature complete and merged, remove the implementation plan:

```bash
git rm docs/plans/2025-10-19-robust-coordinate-types.md
git commit -m "chore: remove completed implementation plan"
```

---

## Summary

This plan implements robust coordinate types that prevent precision loss through:

1. **Type safety** - `Coord<T>` for image space, plain f64 for pixel space
2. **Explicit arithmetic** - Clear method names prevent accidental operations
3. **Generic design** - Supports f64 now, arbitrary precision types later
4. **Zero legacy code** - Complete refactor with thorough cleanup
5. **Comprehensive testing** - Unit, integration, WASM, and browser tests

**Total time estimate:** 2-3 hours for careful implementation following TDD

**Skills referenced:**
- @test-driven-development - Write tests first, watch fail, implement minimal code
- @verification-before-completion - Run full test suite before claiming done
