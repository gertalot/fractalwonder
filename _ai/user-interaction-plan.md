# Interactive Canvas with Fast Preview Architecture

## Overview

Implement user interactions (drag-to-pan, mousewheel-to-zoom) with real-time canvas imageData preview transformations.
The architecture separates concerns into: interaction handling, viewport state management, coordinate transformations,
and preview rendering.

## Architecture Design

### Core Principles

1. **Separation of Concerns**: Interaction logic, state management, coordinate math, and rendering are separate modules
2. **Pixel-Space Operations**: Preview transformations work in pixel space for performance; fractal-space calculations
   happen only after interaction ends
3. **Debounced Rendering**: Interactions trigger fast previews immediately; actual renders start 1.5s after interaction
   stops
4. **Precision Awareness**: Clearly distinguish pixel-space (f64) from fractal-space (arbitrary precision) values

### Module Structure

```
src/
├── state/
│   ├── mod.rs              # Module declarations
│   ├── viewport.rs         # Viewport state (center, zoom, dimensions)
│   └── interaction.rs      # Interaction state (dragging, preview offsets)
├── interaction/
│   ├── mod.rs              # Module declarations
│   ├── mouse_handler.rs    # Mouse event handlers (mousedown, mousemove, mouseup, wheel)
│   └── debounce.rs         # Debounce logic for render triggering
├── preview/
│   ├── mod.rs              # Module declarations
│   ├── transform.rs        # ImageData transformation (translate, scale)
│   └── renderer.rs         # Fast preview rendering to canvas
├── coordinates/
│   ├── mod.rs              # Module declarations
│   └── transform.rs        # Pixel ↔ Fractal coordinate conversions
└── components/
    ├── canvas.rs           # Enhanced with interaction hooks
    └── ...
```

## Component Responsibilities

### 1. State Management (`src/state/`)

**viewport.rs** - Viewport State

- Stores fractal center coordinates (f64 for now, arbitrary precision later)
- Stores zoom level (f64, range 1.0 to 1e8)
- Stores canvas dimensions (u32)
- Provides signals for reactive updates
- Default: center (-0.5, 0.0), zoom 1.0

**interaction.rs** - Interaction State

- Tracks interaction mode: Idle, Dragging, Zooming
- Stores preview offsets (pixel-space translation: dx, dy)
- Stores preview scale factor (for zoom preview)
- Stores last rendered imageData
- Stores debounce timer state

### 2. Interaction Handling (`src/interaction/`)

**mouse_handler.rs** - Event Handlers

- `on_mousedown`: Start drag, capture initial position, cancel any active render
- `on_mousemove`: If dragging, calculate pixel offset, trigger preview update
- `on_mouseup`: End drag, start debounce timer
- `on_wheel`: Calculate zoom delta, update preview scale, start debounce timer
- `on_resize`: Only cancel render if canvas size actually changes

**debounce.rs** - Render Debouncing

- Uses `leptos_use::use_debounce_fn` or custom timer
- 1.5 second delay after last interaction
- On trigger: convert preview state to fractal coordinates, update viewport, start render

### 3. Preview Rendering (`src/preview/`)

**transform.rs** - ImageData Transformation

- `translate_image_data(imageData, dx, dy)`: Shift pixels by offset
- `scale_image_data(imageData, scale, centerX, centerY)`: Scale around point
- Uses temporary canvas for transformations
- Optimized for performance (direct pixel manipulation where possible)

**renderer.rs** - Preview Display

- `render_preview(canvas, imageData, transform)`: Draw transformed imageData
- Clears canvas, applies transformation, draws image
- Handles edge cases (no imageData yet, invalid transforms)

### 4. Coordinate Transformations (`src/coordinates/`)

**transform.rs** - Coordinate Math

- `pixel_to_fractal(px, py, center, zoom, width, height) -> (fx, fy)`
- `fractal_to_pixel(fx, fy, center, zoom, width, height) -> (px, py)`
- `calculate_zoom_factor(wheel_delta) -> f64`: Exponential zoom (1.1 or 0.9 per tick)
- `calculate_new_center_after_zoom(mouse_px, mouse_py, old_center, old_zoom, new_zoom) -> new_center`

Key insight: For zoom, the fractal point under the mouse must remain at the same pixel position.

### 5. Enhanced Canvas Component (`src/components/canvas.rs`)

Responsibilities:

- Render checkerboard (current) or fractal (future)
- Store last rendered imageData in interaction state
- Attach mouse event listeners
- Trigger preview rendering during interactions
- Trigger full render after debounce

## Data Flow

### Drag Interaction Flow

```
1. User mousedown → Set interaction state to Dragging, store mouse position
2. User mousemove → Calculate pixel offset (current - initial)
                  → Update preview offset in state
                  → Call preview::renderer::render_preview()
                  → Draw translated imageData to canvas
3. User mouseup → Set interaction state to Idle
                → Start debounce timer (1.5s)
4. Timer fires → Convert pixel offset to fractal coordinate change
               → Update viewport center
               → Trigger full render (checkerboard for now)
```

### Zoom Interaction Flow

```
1. User wheel event → Calculate zoom factor (1.1^(delta))
                    → Get fractal point under mouse (pixel_to_fractal)
                    → Calculate new zoom level (clamped 1.0 to 1e8)
                    → Calculate new center (keep point under mouse fixed)
                    → Update preview scale in state
                    → Call preview::renderer::render_preview()
                    → Draw scaled imageData to canvas
2. Debounce timer (1.5s) → Update viewport state
                          → Trigger full render
```

### Resize Interaction Flow

```
1. Window resize event → Check if canvas dimensions actually changed
2. If changed → Cancel active render
              → Scale imageData to maintain center
              → Update canvas dimensions
              → Start debounce timer
              → Trigger full render after debounce
3. If unchanged → Do nothing (don't cancel render)
```

## Implementation Details

### Mouse Event Handling

Use `leptos_use::use_event_listener` for:

- `mousedown` on canvas
- `mousemove` on window (to track drag outside canvas)
- `mouseup` on window
- `wheel` on canvas (with `prevent_default` to stop page scroll)

Store event listener cleanup in component effects.

### ImageData Transformation Strategy

**For Translation (Drag):**

- Create temporary canvas same size as original
- Get context, draw original imageData at offset (dx, dy)
- Extract transformed imageData
- Draw to main canvas

**For Scaling (Zoom):**

- Use canvas `drawImage` with scaling parameters
- Scale around mouse position: `translate(-mx, -my), scale(s, s), translate(mx, my)`
- More complex: may need to calculate visible region and only draw that portion

### State Signals

Use Leptos signals for reactivity:

```rust
// Viewport state
let (center, set_center) = create_signal((−0.5, 0.0));
let (zoom, set_zoom) = create_signal(1.0);
let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

// Interaction state
let (interaction_mode, set_interaction_mode) = create_signal(InteractionMode::Idle);
let (preview_offset, set_preview_offset) = create_signal((0.0, 0.0));
let (preview_scale, set_preview_scale) = create_signal(1.0);
let (last_image_data, set_last_image_data) = create_signal(None::<web_sys::ImageData>);
```

### Coordinate Transformation Math

**Pixel to Fractal:**

```rust
fn pixel_to_fractal(px: f64, py: f64, center: (f64, f64), zoom: f64, width: u32, height: u32) -> (f64, f64) {
    let view_width = 3.0 / zoom;  // 3.0 = default real range for Mandelbrot
    let pixel_scale = view_width / width as f64;
    
    let dx_pixels = px - (width as f64 / 2.0);
    let dy_pixels = py - (height as f64 / 2.0);
    
    let fractal_x = center.0 + dx_pixels * pixel_scale;
    let fractal_y = center.1 - dy_pixels * pixel_scale;  // Y inverted
    
    (fractal_x, fractal_y)
}
```

**New Center After Zoom:**

```rust
fn calculate_new_center_after_zoom(
    mouse_px: f64, mouse_py: f64,
    old_center: (f64, f64), old_zoom: f64,
    new_zoom: f64,
    width: u32, height: u32
) -> (f64, f64) {
    // Get fractal point under mouse at old zoom
    let fractal_point = pixel_to_fractal(mouse_px, mouse_py, old_center, old_zoom, width, height);
    
    // Calculate what center would place that fractal point at the mouse pixel with new zoom
    let view_width = 3.0 / new_zoom;
    let pixel_scale = view_width / width as f64;
    
    let dx_pixels = mouse_px - (width as f64 / 2.0);
    let dy_pixels = mouse_py - (height as f64 / 2.0);
    
    let new_center_x = fractal_point.0 - dx_pixels * pixel_scale;
    let new_center_y = fractal_point.1 + dy_pixels * pixel_scale;
    
    (new_center_x, new_center_y)
}
```

## Testing Strategy

### Unit Tests

1. **Coordinate transformations**: Round-trip tests (pixel → fractal → pixel)
2. **Zoom calculations**: Verify point-under-mouse invariant
3. **Debounce logic**: Verify timer behavior
4. **ImageData transforms**: Verify translation/scaling correctness

### Integration Tests

1. **Drag interaction**: Simulate mousedown → mousemove → mouseup, verify state changes
2. **Zoom interaction**: Simulate wheel events, verify zoom and center updates
3. **Resize handling**: Verify only actual size changes trigger re-render

## Future Considerations

### Arbitrary Precision (Later)

When fractal rendering is added:

- Replace f64 center with arbitrary precision type (e.g., `rug::Float`)
- Keep preview operations in f64 (pixel space)
- Only convert to arbitrary precision when updating viewport after interaction ends
- Add precision calculation based on zoom level

### Performance Optimizations (Later)

- Use `requestAnimationFrame` for smooth preview updates
- Throttle mousemove events if needed
- Consider OffscreenCanvas for preview transformations
- Cache transformed imageData if transformation hasn't changed

### Multi-touch Support (Later)

- Detect pinch gestures for zoom
- Calculate center point between touches
- Handle rotation (if desired)

## File Changes Summary

**New files:**

- `src/state/mod.rs`, `src/state/viewport.rs`, `src/state/interaction.rs`
- `src/interaction/mod.rs`, `src/interaction/mouse_handler.rs`, `src/interaction/debounce.rs`
- `src/preview/mod.rs`, `src/preview/transform.rs`, `src/preview/renderer.rs`
- `src/coordinates/mod.rs`, `src/coordinates/transform.rs`

**Modified files:**

- `src/lib.rs`: Declare new modules
- `src/components/canvas.rs`: Add interaction hooks, preview rendering
- `src/app.rs`: Wire up state management
- `Cargo.toml`: Add any needed dependencies (likely none, using existing leptos-use)

## Implementation Order

1. Create state modules (viewport, interaction)
2. Create coordinate transformation module with tests
3. Create preview transformation module with tests
4. Create mouse handler module
5. Create debounce module
6. Integrate into canvas component
7. Test end-to-end interaction flow
8. Add comprehensive tests for edge cases