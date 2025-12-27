# Light Effects Section Design

Add the "Light Effects" collapsible section to the palette editor, matching the prototype at `docs/ux-palette-editor/`.

## Components

### New Components

**`LightingSlider`** (`fractalwonder-ui/src/components/lighting_slider.rs`)

Reusable slider for lighting parameters.

Props:
- `label: &'static str` - Label text
- `value: Signal<f64>` - Current value
- `on_change: Callback<f64>` - Value change handler
- `min: f64`, `max: f64`, `step: f64` - Range configuration
- `precision: u8` (default 2) - Decimal places for display

Layout: `flex items-center gap-2` with label (w-20), range input (flex-1), value display (w-10 text-right).

**`LightingControl`** (`fractalwonder-ui/src/components/lighting_control.rs`)

Circular picker for light direction (azimuth and elevation).

Props:
- `azimuth: Signal<f64>` - Angle in radians
- `elevation: Signal<f64>` - Elevation in radians
- `on_change: Callback<(f64, f64)>` - Change handler (radians)

Features:
- Circular container with aspect-ratio 1:1
- Concentric guide circles at 25%, 50%, 75%, 100% radius
- Center dot indicator
- Draggable white circle showing light position
- Azimuth/elevation display in degrees below
- Document-level mouse handlers for smooth dragging

Coordinate mapping:
- Azimuth: angle from top, clockwise (0 = top, PI/2 = right)
- Elevation: center = PI/2 (90 deg, overhead), edge = 0 (horizon)

### Modified Components

**`CurveEditor`** - Remove hardcoded "Transfer Curve" label (line 118). Parent provides label externally.

**`PaletteEditor`** - Add Light Effects section after Palette section.

## Data Flow

Derived signals from `state.working_palette`:
- `shading_enabled` - Boolean for 3D toggle
- `falloff_curve_signal` - For CurveEditor
- `ambient`, `diffuse`, `specular`, `shininess`, `strength` - For LightingSliders
- `azimuth`, `elevation` - For LightingControl

Each has a corresponding callback that updates `state.working_palette.lighting.*`.

Conditional rendering: Falloff curve, sliders, and LightingControl only visible when `shading_enabled` is true.

## View Structure

```
<CollapsibleSection title="Light Effects" expanded=light_effects_expanded>
    // 3D Lighting checkbox
    <label>
        <input type="checkbox" checked=shading_enabled />
        "3D Lighting"
    </label>

    <Show when=shading_enabled>
        // Falloff Curve
        <div>"3D Falloff Curve"</div>
        <CurveEditor curve=falloff_curve_signal on_change=on_falloff_change />

        // Lighting Parameters
        <div>"Lighting Parameters"</div>
        <LightingSlider label="Ambient" min=0 max=1 step=0.01 precision=2 />
        <LightingSlider label="Diffuse" min=0 max=1 step=0.01 precision=2 />
        <LightingSlider label="Specular" min=0 max=1 step=0.01 precision=2 />
        <LightingSlider label="Shininess" min=1 max=128 step=1 precision=0 />
        <LightingSlider label="Strength" min=0 max=2 step=0.01 precision=2 />

        // Light Direction
        <div>"Light Direction"</div>
        <LightingControl azimuth=azimuth elevation=elevation on_change=on_direction_change />
    </Show>
</CollapsibleSection>
```

## Module Updates

Add to `fractalwonder-ui/src/components/mod.rs`:
- `mod lighting_slider;`
- `mod lighting_control;`
- `pub use lighting_slider::LightingSlider;`
- `pub use lighting_control::LightingControl;`

## Implementation Order

1. Create `LightingSlider` component
2. Create `LightingControl` component
3. Modify `CurveEditor` to remove hardcoded label
4. Add Light Effects section to `PaletteEditor`
5. Wire up signals and callbacks
6. Test with prototype comparison
