# Pluggable Renderer Architecture

## Overview

Type-safe, pluggable renderer system with compile-time coordinate space separation.

## Core Abstractions

### Coordinate Types (`src/rendering/coords.rs`)

**PixelCoord** - Screen-space coordinates (always f64)

- Private fields prevent raw access
- Used for canvas pixel positions

**ImageCoord<T>** - Image-space coordinates (generic over precision)

- T = f64 for test images
- T = rug::Float for high-precision fractals
- Private fields enforce transformation through utilities

**ImageRect<T>** - Rectangular region in image space

- Defines rendering target area

### Viewport (`src/rendering/viewport.rs`)

```rust
struct Viewport<T> {
    center: ImageCoord<T>,
    zoom: f64,
    natural_bounds: ImageRect<T>,
}
```

- `zoom = 1.0` displays entire natural_bounds
- `zoom = 2.0` displays half the area (2x magnification)

### Transformations (`src/rendering/transforms.rs`)

**calculate_visible_bounds** - Computes visible ImageRect from viewport + canvas size

- Handles aspect ratio by extending wider dimension
- Ensures natural_bounds fits in constraint dimension at zoom 1.0

**pixel_to_image** - Converts PixelCoord → ImageCoord<T>

- Requires viewport context
- Type system prevents conversion without context

**image_to_pixel** - Converts ImageCoord<T> → PixelCoord

- Inverse transformation
- Round-trip guarantees precision

### CanvasRenderer Trait (`src/rendering/renderer_trait.rs`)

```rust
trait CanvasRenderer {
    type Coord: Clone;
    fn natural_bounds(&self) -> ImageRect<Self::Coord>;
    fn render(&self, target_rect: &ImageRect<Self::Coord>, width: u32, height: u32) -> Vec<u8>;
}
```

- Associated type `Coord` declares precision requirements
- `render()` receives arbitrary ImageRect for future tiling support
- Returns raw RGBA pixel data

## Type Safety Guarantees

1. **Cannot mix coordinate spaces** - PixelCoord and ImageCoord<T> are distinct types
2. **Cannot access raw coordinates without explicit call** - Private fields require `.x()`, `.y()`
3. **Cannot convert without context** - Transformations require viewport + canvas dimensions
4. **Cannot mix precision types** - f64 and rug::Float renderers use different ImageCoord<T>

## Component Architecture

**Canvas<R: CanvasRenderer>** - Generic rendering component

- Calculates visible bounds from viewport
- Calls renderer.render()
- Puts pixels on HTML canvas

**TestImageView** - Wrapper owning viewport state

- Creates renderer instance
- Manages viewport signal
- Composes with Canvas

**App** - Top-level with dynamic switching

- RendererType enum for selection
- Component-level match expression
- Each renderer branch fully typed

## Adding New Renderers

1. Implement `CanvasRenderer` trait
2. Declare `type Coord = f64` or `rug::Float`
3. Implement `natural_bounds()` and `render()`
4. Create wrapper component owning viewport
5. Add variant to `RendererType` enum
6. Add branch to App match expression

## Future Extensions

- Tiling system: Renderer already receives arbitrary ImageRect
- Progressive rendering: Wrapper component manages tile queue
- Pan/zoom: Wrapper component modifies viewport signal
- URL persistence: Serialize viewport to URL params
