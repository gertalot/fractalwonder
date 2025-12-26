# Transfer Curve Editor Design

## Overview

Add a CurveEditor component to the Palette section of the palette editor, placed below the GradientEditor. The transfer curve maps iteration values to palette positions, controlling color distribution.

## Component Interface

**File:** `fractalwonder-ui/src/components/curve_editor.rs`

```rust
#[component]
pub fn CurveEditor(
    /// The curve being edited (None when editor closed)
    curve: Signal<Option<Curve>>,
    /// Called when curve changes
    on_change: Callback<Curve>,
    /// Canvas size in logical pixels (default 320)
    #[prop(default = 320)]
    size: u32,
) -> impl IntoView
```

## Canvas Rendering

**Dimensions:** 320×320 logical pixels (rendered at device pixel ratio for sharpness)

**Drawing layers (back to front):**

1. **Background:** `rgba(255, 255, 255, 0.05)` fill
2. **Grid:** 4×4 subdivision, `rgba(255, 255, 255, 0.1)` lines
3. **Diagonal reference:** Dashed line bottom-left → top-right, `rgba(255, 255, 255, 0.2)`
4. **Curve:** Sample 100 points via `Curve::evaluate()`, draw as polyline, `rgba(255, 255, 255, 0.8)`, 2px width
5. **Control points:** White filled circles (5px radius, 6px on hover), black stroke

**Coordinate system:**

- X: 0=left, 1=right
- Y: 0=bottom, 1=top (canvas Y inverted)

## Interactions

| Action | Behavior |
|--------|----------|
| Click empty area | Add new control point at position |
| Click existing point | Select for potential drag |
| Drag point | Move position (first/last points locked to x=0/x=1) |
| Double-click point | Delete (except first/last, minimum 2 points) |
| Mouse hover | Highlight nearest point within 10px |

**Document-level drag tracking:** Register mouseup/mousemove on document during drag to handle cursor leaving canvas.

## Integration with PaletteEditor

Add to `palette_editor.rs` inside the Palette `CollapsibleSection`, after `GradientEditor`:

```rust
// Derived signal for transfer curve
let transfer_curve_signal = Signal::derive(move || {
    state.get().map(|s| s.working_palette.transfer_curve.clone())
});

// Callback for curve changes
let on_transfer_curve_change = Callback::new(move |new_curve: Curve| {
    state.update(|opt| {
        if let Some(s) = opt {
            s.working_palette.transfer_curve = new_curve;
        }
    });
});

// In view, after GradientEditor:
<CurveEditor
    curve=transfer_curve_signal
    on_change=on_transfer_curve_change
/>
```

## Files to Create/Modify

| File | Action |
|------|--------|
| `fractalwonder-ui/src/components/curve_editor.rs` | Create - new component |
| `fractalwonder-ui/src/components/mod.rs` | Modify - export CurveEditor |
| `fractalwonder-ui/src/components/palette_editor.rs` | Modify - integrate CurveEditor |

## Design Decisions

1. **Cubic spline display:** Show actual interpolated curve (100 sample points), not line segments between control points
2. **Placement:** Below GradientEditor in Palette section - natural workflow of defining colors then distribution
3. **Pattern:** Follows GradientEditor's Signal/Callback pattern for consistency
