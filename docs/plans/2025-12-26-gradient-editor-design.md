# Gradient Editor Design

## Overview

Add an interactive gradient editor to the palette editor's "Palette" section. Users can visually edit color stops, adjust midpoints, and see an OKLAB-correct preview matching the fractal rendering.

## Features

- Color gradient bar (32px height, canvas, OKLAB interpolation)
- Draggable color stops (12x12px squares, `ew-resize` cursor)
- Draggable midpoint diamonds (10x10px, between stops)
- Click gradient bar to add stops (samples current gradient color)
- Double-click stop to delete (min 2 stops, silently ignored at limit)
- Zoom controls: +/- buttons (1x-10x range)
- Horizontal scroll when zoomed
- Color picker panel below gradient (native input + hex text input)

## Visual Layout

```
┌─────────────────────────────────────────────────┐
│  [−]  Zoom: 2.5x                          [+]   │  ← Zoom controls (hidden at 1x)
├─────────────────────────────────────────────────┤
│  ■        ◇        ■        ◇        ■          │  ← Stops + Midpoints
│  ████████████████████████████████████████████   │  ← Gradient bar (32px, canvas)
├─────────────────────────────────────────────────┤
│  [🎨]  #ff4400                                  │  ← Color picker (when stop selected)
└─────────────────────────────────────────────────┘
```

## Interactions

| Element | Action | Result |
|---------|--------|--------|
| Color stop | Click | Select, show color picker |
| Color stop | Drag | Move position (0-1) |
| Color stop | Double-click | Delete (if >2 stops) |
| Midpoint | Drag | Adjust blend bias |
| Gradient bar | Click | Add stop (color sampled) |
| +/- buttons | Click | Zoom in/out |
| Container | Scroll | Pan when zoomed |

**Active stop indicator:** Subtle glow (`box-shadow: 0 0 6px 2px rgba(255,255,255,0.7)`)

**Update behavior:** Changes apply on mouse release (not continuous).

## Component API

```rust
#[component]
pub fn GradientEditor(
    gradient: Signal<Option<Gradient>>,
    on_change: Callback<Gradient>,
) -> impl IntoView
```

**Internal state:**
- `selected_stop: RwSignal<Option<usize>>`
- `zoom: RwSignal<f64>` (1.0 - 10.0)
- `is_dragging: RwSignal<bool>`

## Canvas Rendering

The gradient bar uses `<canvas>` with OKLAB interpolation to match the fractal renderer.

```rust
impl Gradient {
    /// Generate LUT sized to canvas width for editor preview
    pub fn to_preview_lut(&self, width: usize) -> Vec<[u8; 3]>
}
```

New stops sample color from this LUT at the click position.

## Utility Functions

New module `src/utils/color.rs`:

```rust
/// Convert RGB to hex string: [255, 68, 0] → "#ff4400"
pub fn rgb_to_hex(rgb: [u8; 3]) -> String

/// Parse hex string to RGB: "#ff4400" → Some([255, 68, 0])
pub fn hex_to_rgb(hex: &str) -> Option<[u8; 3]>
```

## Files

**New:**
- `src/components/gradient_editor.rs`
- `src/utils/color.rs`

**Modified:**
- `src/components/mod.rs` - export GradientEditor
- `src/components/palette_editor.rs` - add GradientEditor to Palette section
- `src/rendering/colorizers/gradient.rs` - add `to_preview_lut()`
- `src/utils/mod.rs` - export color utilities

## Integration

GradientEditor is placed inside the Palette collapsible section, after the checkboxes. Changes go through `state.update()` to trigger dirty detection.
