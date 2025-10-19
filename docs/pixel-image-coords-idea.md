# Robust types for coordinates in image space to avoid loss of precision

## Overview

This app provides an architecture for arbitrary rendering engines that display images on a canvas, where the user can
pan and zoom the image (with realtime previews), triggering a (potentially expensive) re-render.

User interactions and showing the pixels on screen happens in _pixel space_, while the calculations in the renderers
happen in _image space_. EXAMPLE: In the future we will implement a mandelbrot fractal renderer, that calculates points
in the mandelbrot set, typically with center (-0.5,0.0) and width,height (3.0,2.0). To facilitate panning and zooming,
functions exist in `src/rendering/transforms.rs` that map between pixel space and image space.

Currently we have a `src/components/test_image.rs` test image renderer which is very basic and fast and exists during
development to help us iteratively develop features, but in the future we will implement the fractal renderer, which
uses image coordinates that can be extremely small (e.g. hundreds of decimal places).

Pan and zoom with realtime preview is implemented in `src/hooks/use_canvas_interaction.rs`.

## Goal

Provide a robust type for coordinates in image space and an architecture that enforces the use of image coordinates and
specialised arithmetic that potentially supports arbitrary precision. The idea is that we make it completely clear at
compile time which arithmetic uses pixel space (for which `f64` types are fine) and which arithmetic uses image space,
which **potentially** uses an arbitrary precision type.

The ultimate goal of this architecture is to prevent accidental loss of precision, where pixel space and image space
numbers are used in the same calculation using ordinary arithmetic, causing loss of precision.

## Requirements

- Create a `Coord` type with (x,y) fieldsthat implements standard arithmetic operations:
  1. addition, "add": given A(x1,y1) and B(x2,y2), then A + B = (x1+x2, y1+y2)
  2. subtraction "sub": given A(x1,y1) and B(x2,y2) then A - B = (x1-x2, y1-y2)
  3. scalar multiplication "mul": given A(x1,y1) and k, then A * k = (x1*k, y1\*k)
  4. scalar division "div": given A(x1,y1) and k, then A / k = (x1/k, y1/k)
  5. dot product "dot": given A(x1,y1) and B(x2,y2), then A • B = x1*x2 + y1*y2
  6. cross product "cross": given A(x1,y1) and B(x2,y2), then A × B = x1*y2 - y1*x2
- `Coord` is generic, so we can implement x,y as `f64` or an arbitrary precision type
- arithmetic operations should NOT accidentally lose precision, e.g. if A is a point with 100 decimal places,
  A.div(2.0) should **NOT** return a point of `f64` type.
- We must be able to create type `T` numbers from "ordinary" (`f64`, `u32`, etc) numbers
- The code MUST support the WASM target, so any arbitrary precision libraries must be pure Rust
- In the future we MAY want to support WebGL as a target which MAY require small, dedicated, custom high precision
  arithmetic

## Current implementation

`src/rendering/coords.rs` provides two coordinate types, `PixelCoord` and `ImageCoord`, and they are **NOT** used
consistently throughout the codebase.

Proposal:

- get rid of `PixelCoord` altogether and just use `f64` operations.
- rename `ImageCoord` to `Coord`
- implement the arithmetic operations as above on `Coord`.
- refactor the code to **consistently** use ordinary `f64` arithmetic on all operations that happen in pixel space
- refactor the code to **consistently** use `Coord<T>` arithmetic on all operations that happen in image space
- ANY ACCIDENTAL LOSS OF PRECISION **MUST** BE ELIMINATED

NOTE: The test image renderer uses `f64` which is fine. Future renderers might use `rug`, `dasha`, or a custom
arbirtrary precision type. We don't know yet, that's why `Coord` is generic.

## TASKS

1. refactor out `PixelCoord` and use `f64` instead
2. rename `ImageCoord` to `Coord`
3. implement the arithmetic operations as above on `Coord`.
4. ensure ALL calculations in image space use `Coord` types.
5. THOROUGHLY clean up the codebase and do NOT leave any legacy code.
