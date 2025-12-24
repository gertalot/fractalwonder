# Palette Editor UI Architecture

This document describes the React/TypeScript mockup created with Figma Make, intended as design guidance for implementing a palette editor in the Fractal Wonder application (Rust/Leptos/WASM).

## Overview

The palette editor is a slide-out panel that provides comprehensive control over fractal coloring, including:
- Color gradient editing with multiple stops
- Transfer curve editing for color distribution
- 3D lighting controls (Blinn-Phong shading)
- Light direction visualization

## Data Model

### Palette

The core data structure representing a complete palette configuration:

```typescript
interface Palette {
  id: string;
  name: string;
  stops: ColorStop[];           // Gradient color stops
  histogram: boolean;           // Histogram equalization toggle
  smooth: boolean;              // Smooth coloring toggle
  use3D: boolean;               // 3D lighting toggle
  transferCurve: Point[];       // Color distribution curve
  falloffCurve: Point[];        // 3D lighting falloff curve
  lighting: LightingParams;     // Blinn-Phong parameters
}
```

### ColorStop

Individual color stops in the gradient:

```typescript
interface ColorStop {
  position: number;  // 0-1, position along gradient
  color: string;     // Hex color (e.g., "#004e92")
}
```

### Point

Control points for curves:

```typescript
interface Point {
  x: number;  // 0-1, horizontal position
  y: number;  // 0-1, vertical position (output value)
}
```

### LightingParams

Blinn-Phong lighting parameters:

```typescript
interface LightingParams {
  ambient: number;    // 0-1, ambient light level
  diffuse: number;    // 0-1, diffuse reflection strength
  specular: number;   // 0-1, specular reflection strength
  shininess: number;  // 1-128, specular exponent
  strength: number;   // 0-2, overall shading strength
  azimuth: number;    // 0-360, light direction angle (degrees)
  elevation: number;  // 0-90, light elevation angle (degrees)
}
```

## Component Hierarchy

```
App
├── MandelbrotCanvas          # Full-screen fractal canvas (background)
├── BottomControlBar          # Auto-hiding toolbar at bottom
│   ├── Info button
│   ├── Home button
│   ├── Palette button        # Opens palette editor
│   ├── Settings button
│   └── Fullscreen toggle
└── PaletteEditor             # Slide-out panel (right side)
    ├── Header
    │   ├── Editable palette name
    │   ├── Cancel / Apply buttons
    │   └── Duplicate / Delete buttons
    ├── Palette Section (collapsible)
    │   ├── Histogram Equalization checkbox
    │   ├── Smooth Coloring checkbox
    │   ├── GradientEditor
    │   └── CurveEditor (Transfer Curve)
    └── Light Effects Section (collapsible)
        ├── 3D Lighting checkbox
        ├── CurveEditor (Falloff Curve) [shown when 3D enabled]
        ├── Lighting sliders [shown when 3D enabled]
        │   ├── Ambient
        │   ├── Diffuse
        │   ├── Specular
        │   ├── Shininess
        │   └── Strength
        └── LightingControl (direction) [shown when 3D enabled]
```

## Component Specifications

### PaletteEditor

**Location:** `src/components/PaletteEditor.tsx`

The main container panel for palette editing. Slides in from the right edge of the screen.

**Appearance:**
- Fixed width: 380px
- Full height of viewport
- Semi-transparent black background (90% opacity) with backdrop blur
- Left border: 1px white at 10% opacity
- Slide animation: 300ms transition

**Sections:**
1. **Header** - Palette name (click-to-edit), action buttons
2. **Palette Section** - Collapsible, contains gradient and curve editors
3. **Light Effects Section** - Collapsible, contains 3D lighting controls

**State:**
- `isEditingName: boolean` - Whether name is being edited
- `paletteExpanded: boolean` - Palette section collapsed state
- `lightEffectsExpanded: boolean` - Light effects section collapsed state

**Props:**
- `visible: boolean` - Controls slide-in/out animation
- `palette: Palette` - Current palette data
- `onChange: (palette: Palette) => void` - Called on any change
- `onApply: () => void` - Apply and close
- `onCancel: () => void` - Discard changes and close

---

### GradientEditor

**Location:** `src/components/GradientEditor.tsx`

Interactive gradient bar with draggable color stops and midpoint controls.

**Features:**
- Visual gradient bar showing current colors
- Draggable color stop markers (squares above the bar)
- Midpoint diamonds between stops (controls gradient interpolation center)
- Click on gradient bar to add new stops
- Zoom controls (1x-10x) for fine-tuning
- Horizontal scroll when zoomed
- Color picker popup for selected stop

**Appearance:**
- Gradient bar: 8px height (32px total with markers), rounded corners
- Color stops: 12x12px squares, positioned above bar
- Midpoints: 10x10px diamonds, rotated 45 degrees
- Border: 1px white at 20% opacity

**Interactions:**
- Click gradient bar → Add new color stop at click position
- Click color stop → Select and show color picker
- Drag color stop → Move position (0-1)
- Drag midpoint → Adjust gradient bias between two stops
- Ctrl+scroll → Zoom in/out
- Delete button → Remove selected stop (minimum 2 stops required)

**State:**
- `selectedStopIndex: number | null` - Currently selected stop
- `showColorPicker: boolean` - Color picker visibility
- `midpoints: { [key: string]: number }` - Midpoint values per segment
- `zoom: number` - Current zoom level (1-10)

---

### CurveEditor

**Location:** `src/components/CurveEditor.tsx`

Interactive bezier/linear curve editor rendered on canvas.

**Purpose:**
- **Transfer Curve**: Maps iteration values to palette positions (color distribution)
- **Falloff Curve**: Controls 3D lighting falloff based on distance from set

**Appearance:**
- Canvas area: Configurable size (default 320x320, rendered at 100% width)
- Background: Semi-transparent white (5% opacity)
- Border: 1px white at 10% opacity, rounded corners
- Grid: 4x4 subdivision, white lines at 10% opacity
- Diagonal reference: Dashed line from bottom-left to top-right (20% opacity)
- Curve: White line at 80% opacity, 2px width
- Points: White circles (5px radius), black stroke

**Coordinate System:**
- X axis: Input value (0 = left, 1 = right)
- Y axis: Output value (0 = bottom, 1 = top)
- Canvas Y is inverted (0 at top, size at bottom)

**Interactions:**
- Click empty area → Add new control point
- Drag point → Move point (first/last points constrained to x=0/x=1)
- Double-click point → Delete point (except first and last)
- Mouse hover → Visual highlight on nearest point

**Constraints:**
- Minimum 2 points
- First point locked to x=0
- Last point locked to x=1
- Points stored sorted by x-coordinate

---

### LightingControl

**Location:** `src/components/LightingControl.tsx`

Circular control for setting light direction (azimuth and elevation).

**Purpose:**
Maps 2D circular interaction to 3D light direction:
- Angle from center → Azimuth (0-360 degrees)
- Distance from center → Elevation (center = 90° overhead, edge = 0° horizon)

**Appearance:**
- Circular container (aspect-ratio: 1:1)
- Concentric guide circles at 25%, 50%, 75%, 100% radius
- Center dot indicator
- White circular marker showing current light position
- Display of current azimuth and elevation values below

**Interactions:**
- Click/drag within circle → Set light direction
- Position calculated relative to circle center
- Drag continues tracking even when cursor leaves element

**Coordinate Mapping:**
```
azimuth = atan2(dy, dx) + 90° (normalized to 0-360)
elevation = 90 - (distance_from_center / radius) * 90
```

---

### BottomControlBar

**Location:** `src/components/BottomControlBar.tsx`

Auto-hiding toolbar at the bottom of the screen.

**Appearance:**
- Full width, fixed at bottom
- Semi-transparent black (70% opacity) with backdrop blur
- Icons from Lucide React library
- Fade in/out animation (300ms)

**Auto-hide Behavior:**
- Shows on mouse movement
- Hides after 2 seconds of inactivity
- Hidden when palette editor is open

**Buttons:**
- Info, Home, Palette, Settings (left side)
- Status text (center)
- Fullscreen toggle (right side)

---

### MandelbrotCanvas

**Location:** `src/components/MandelbrotCanvas.tsx`

Full-screen canvas for rendering the Mandelbrot set. This is a simplified mockup renderer.

**Note:** This component is for mockup purposes only. The actual Fractal Wonder application has a sophisticated WASM-based renderer with web workers. This mockup demonstrates the visual relationship between the palette editor and the fractal display.

## Visual Design System

### Colors

The UI uses a dark theme with white text and controls:

| Element | Color |
|---------|-------|
| Panel background | `rgba(0, 0, 0, 0.9)` |
| Control bar background | `rgba(0, 0, 0, 0.7)` |
| Primary text | `#ffffff` |
| Secondary text | `rgba(255, 255, 255, 0.7)` |
| Muted text | `rgba(255, 255, 255, 0.5)` |
| Borders | `rgba(255, 255, 255, 0.1)` - `rgba(255, 255, 255, 0.2)` |
| Hover backgrounds | `rgba(255, 255, 255, 0.1)` |
| Active/selected | `rgba(255, 255, 255, 0.2)` |

### Typography

- Font family: `system-ui, -apple-system, sans-serif`
- Small text (labels, hints): 12px (`text-xs`)
- Normal text (buttons, inputs): 14px (`text-sm`)
- Headings: Inherited from base (medium weight)

### Spacing

Based on Tailwind's spacing scale (0.25rem base):
- Compact spacing: 4px (`space-y-1`)
- Normal spacing: 8px (`space-y-2`)
- Section spacing: 12px (`space-y-3`)
- Panel padding: 16px (`p-4`)

### Interactive States

- Hover: Background lightens (`bg-white/10`)
- Focus: Border brightens (`border-white/40`)
- Disabled: Reduced opacity (30%)
- Selected: Ring indicator (`ring-1 ring-white`)

### Transitions

- Duration: 150ms (default), 300ms (panels)
- Timing: `cubic-bezier(0.4, 0, 0.2, 1)` (Tailwind default)

## Mapping to Existing Rust Types

The mockup data structures map to existing Rust types in `settings.rs`:

| Mockup | Rust Type | Notes |
|--------|-----------|-------|
| `palette.histogram` | `ColorOptions.histogram_enabled` | Direct mapping |
| `palette.smooth` | `ColorOptions.smooth_enabled` | Direct mapping |
| `palette.use3D` | `ColorOptions.shading_enabled` | Direct mapping |
| `palette.stops` | `Palette` | Needs gradient stop representation |
| `palette.transferCurve` | `transfer_bias` | Currently simple power function; mockup uses curve |
| `palette.falloffCurve` | `ShadingSettings.distance_falloff` | Currently single value; mockup uses curve |
| `palette.lighting.*` | `ShadingSettings.*` | Direct mapping (convert degrees to radians) |

### Required Extensions

To fully implement this UI, the Rust types may need:

1. **Gradient stops representation** - Currently palettes are predefined; need runtime-editable gradient stops
2. **Transfer curve points** - Replace simple `transfer_bias: f32` with control points
3. **Falloff curve points** - Replace simple `distance_falloff: f64` with control points

## Implementation Notes

### Leptos/WASM Considerations

1. **Canvas Rendering**: The curve editor and gradient preview use HTML canvas. In Leptos, use `web_sys` bindings or `leptos_canvas` crate.

2. **Mouse Events**: Drag operations use document-level event listeners for smooth tracking. In Leptos, manage these with `on_cleanup` for proper disposal.

3. **Animation**: CSS transitions work natively. For JS-driven animations, consider `request_animation_frame` via `web_sys`.

4. **State Management**: The mockup uses React's useState. In Leptos, use signals (`create_signal`, `create_memo`).

### Recommended Component Structure (Leptos)

```rust
// palette_editor.rs
#[component]
pub fn PaletteEditor(
    visible: ReadSignal<bool>,
    palette: RwSignal<Palette>,
    on_apply: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView

// gradient_editor.rs
#[component]
pub fn GradientEditor(
    stops: RwSignal<Vec<ColorStop>>,
) -> impl IntoView

// curve_editor.rs
#[component]
pub fn CurveEditor(
    points: RwSignal<Vec<Point>>,
    size: u32,
) -> impl IntoView

// lighting_control.rs
#[component]
pub fn LightingControl(
    azimuth: RwSignal<f64>,
    elevation: RwSignal<f64>,
) -> impl IntoView
```

## Files Reference

| File | Purpose |
|------|---------|
| `src/App.tsx` | Main application, state management |
| `src/components/PaletteEditor.tsx` | Main palette panel |
| `src/components/GradientEditor.tsx` | Gradient bar with color stops |
| `src/components/CurveEditor.tsx` | Bezier/linear curve canvas |
| `src/components/LightingControl.tsx` | Circular light direction control |
| `src/components/BottomControlBar.tsx` | Auto-hiding toolbar |
| `src/components/MandelbrotCanvas.tsx` | Mockup fractal renderer |
| `src/styles/globals.css` | CSS variables and base styles |
| `src/index.css` | Compiled Tailwind CSS |

## Running the Mockup

```bash
cd docs/ux-palette-editor
npm install
npm run dev
```

Opens at http://localhost:5173 (Vite dev server).
