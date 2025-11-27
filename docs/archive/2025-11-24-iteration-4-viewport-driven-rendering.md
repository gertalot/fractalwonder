# Iteration 4: Viewport-Driven Rendering

## Goal

Prove coordinate transforms work end-to-end with an interactive test pattern.

## Design Decisions

### 1. Viewport Update Flow

**Callback-based** (matching archive pattern):

```rust
#[component]
pub fn InteractiveCanvas(
    viewport: Signal<Viewport>,
    on_viewport_change: Callback<Viewport>,
    on_resize: Callback<(u32, u32)>,
) -> impl IntoView
```

App owns `RwSignal<Viewport>`. InteractiveCanvas receives read-only signal and fires callback on interaction end.

### 2. Test Pattern Design

A **dynamic ruler** that adapts to zoom level:

- Horizontal ruler line through y=0
- Vertical ruler line through x=0
- Tick marks at major (10x), medium (5x), minor (1x) intervals
- Light grey checkerboard background aligned to major grid
- Origin marker where axes cross

### 3. Tick Scaling Algorithm

All parameters derive from one calculation:

```rust
let width_f64 = viewport.width.to_f64();  // OK for scale calculation only
let log_width = width_f64.log10();
let major_exp = (log_width - 0.5).floor() as i32;
let major_spacing = 10.0_f64.powi(major_exp);
```

At viewport width 4.0: major ticks every 1.0 unit
At viewport width 0.04: major ticks every 0.01 unit

### 4. Tick Thickness

Proportional to spacing (constant visual thickness across all zoom levels):

```rust
let axis_thickness = major_spacing / 100.0;
let major_tick_threshold = major_spacing / 50.0;
let medium_tick_threshold = major_spacing / 75.0;
let minor_tick_threshold = major_spacing / 100.0;
```

A pixel is "on a tick" if its distance to the nearest tick line is less than the threshold.

### 5. Code Location

Test pattern rendering stays in **UI crate** for this iteration. Will move to compute crate in Iteration 6.

## Component Architecture

```
App (owns RwSignal<Viewport>)
  |
  +-> InteractiveCanvas
  |     - Receives viewport: Signal (read-only)
  |     - Receives on_viewport_change: Callback
  |     - Wires use_canvas_interaction hook
  |     - Render effect reacts to viewport changes
  |
  +-> UIPanel
        - Receives viewport: Signal (read-only)
        - Displays center, width/height
```

## Rendering Flow

```
User interacts (drag/pinch/scroll)
    |
    v
use_canvas_interaction captures ImageData, provides preview
    |
    v
1.5s idle timeout
    |
    v
Hook fires on_interaction_end(PixelTransform)
    |
    v
InteractiveCanvas callback:
    1. current_vp = viewport.get_untracked()
    2. new_vp = apply_pixel_transform_to_viewport(&current_vp, &transform, canvas_size)
    3. on_viewport_change(new_vp)
    |
    v
App updates RwSignal<Viewport>
    |
    v
InteractiveCanvas render effect triggers
    |
    v
For each pixel:
    1. (fx, fy) = pixel_to_fractal(px, py, &viewport, canvas_size)
    2. color = test_pattern_color(fx, fy, &tick_params)
    3. Write to ImageData buffer
    |
    v
putImageData to canvas
```

## File Structure

```
fractalwonder-ui/src/
  components/
    interactive_canvas.rs  # Modify: add viewport prop, wire hook
  rendering/
    mod.rs                 # Add test_pattern module
    test_pattern.rs        # NEW: tick calculation, color logic
  app.rs                   # Modify: pass viewport signal + callback
```

## Test Strategy

Pure math functions (no DOM) can be unit tested:

```rust
#[test]
fn tick_spacing_at_width_4() {
    let params = calculate_tick_params(&BigFloat::from_f64(4.0, 64));
    assert!((params.major_spacing - 1.0).abs() < 0.001);
}

#[test]
fn checkerboard_alternates_at_integer_boundaries() {
    let params = calculate_tick_params(&BigFloat::from_f64(4.0, 64));
    let c1 = test_pattern_color(&bf(0.5), &bf(0.5), &params);
    let c2 = test_pattern_color(&bf(1.5), &bf(0.5), &params);
    assert_ne!(c1, c2);
}

#[test]
fn axis_line_detected_near_zero() {
    let params = calculate_tick_params(&BigFloat::from_f64(4.0, 64));
    let color = test_pattern_color(&bf(0.5), &bf(0.001), &params);
    assert_eq!(color, AXIS_COLOR);
}
```

## Browser Test Checklist

- [ ] See checkerboard with visible origin marker
- [ ] Ruler lines visible at x=0 and y=0
- [ ] Tick marks at regular intervals
- [ ] Drag canvas, pattern moves (preview)
- [ ] Release, wait 1.5s, pattern re-renders at new position
- [ ] Zoom with scroll wheel, ticks rescale appropriately
- [ ] Deep zoom shows smaller tick intervals
- [ ] UI panel shows current center (x, y) and width/height
