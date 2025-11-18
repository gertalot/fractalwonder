# Live UI Coordinate Updates During Interaction

**Date:** 2025-11-18
**Status:** Approved Design
**Author:** Claude Code with Gert

## Problem Statement

The UI panel displays zoom and center coordinates, but these only update after interaction stops (1.5 second timeout). Users want to see coordinates update in real-time during drag and zoom interactions.

## Current Behavior

**Data flow:**
```
User interacts (drag/zoom)
    ↓
[use_canvas_interaction hook]
    ├─ Transforms pixels for preview (ImageData)
    ├─ Does NOT update viewport signal
    └─ After 1.5s timeout → calls on_interaction_end
         ↓
[InteractiveCanvas]
    └─ Updates viewport signal
         ↓
[App renderer_info effect]
    └─ Recalculates renderer_info
         ↓
[UI Component]
    └─ Displays updated coordinates
```

**Result:** Coordinates are stale during interaction, only update 1.5 seconds after user stops.

## Design Goals

1. **Live coordinate updates** - UI shows current coordinates during interaction
2. **No performance degradation** - Avoid expensive renders during interaction
3. **Preserve hook genericity** - Keep `use_canvas_interaction` reusable (not viewport-specific)
4. **Minimal changes** - Additive API changes, no logic modifications

## Solution: Decouple Viewport Updates from Rendering

**Key insight:** Updating viewport is cheap (signal + string formatting). Rendering fractals is expensive. Currently these are coupled. We need to decouple them.

### Architecture

**Two callbacks instead of one:**
- `on_interaction` - fires during interaction with current transform
- `on_interaction_end` - fires when interaction stops (existing behavior)

**Both update viewport, only end triggers render:**
- Viewport stays live throughout interaction
- Rendering is gated by `is_interacting` flag
- UI always displays current state

### Reactive Flow

```
User drags/zooms
    ↓
[use_canvas_interaction]
    ├─ Updates internal transform state (unchanged)
    ├─ Renders pixel preview (unchanged)
    └─ Calls on_interaction(current_transform)  ← NEW
         ↓
[InteractiveCanvas]
    └─ update_viewport_from_transform runs
         ↓
         └─ set_viewport.update() fires
              ↓
              ├─ [App renderer_info effect]
              │   └─ Recalculates renderer_info (~microseconds)
              │        ↓
              │   [UI Component]
              │        └─ Displays updated coordinates ← LIVE
              │
              └─ [InteractiveCanvas render effect]
                  └─ Sees is_interacting = true
                  └─ Skips expensive render ← OPTIMIZATION

User stops (1.5s timeout)
    ↓
[use_canvas_interaction]
    ├─ Sets is_interacting = false
    └─ Calls on_interaction_end(final_transform)
         ↓
[InteractiveCanvas render effect]
    └─ Sees is_interacting = false
    └─ Renders final viewport ← FINAL RENDER
```

## Implementation Details

### 1. Hook API Change (additive)

**Before:**
```rust
pub fn use_canvas_interaction(
    canvas_ref: NodeRef<Canvas>,
    on_interaction_end: impl Fn(TransformResult) + 'static + Clone,
) -> CanvasInteractionHandle
```

**After:**
```rust
pub fn use_canvas_interaction(
    canvas_ref: NodeRef<Canvas>,
    on_interaction: impl Fn(TransformResult) + 'static + Clone,      // NEW
    on_interaction_end: impl Fn(TransformResult) + 'static + Clone,  // EXISTING
) -> CanvasInteractionHandle
```

**Hook implementation:**
- Call `on_interaction` whenever transform state changes during interaction
- Call `on_interaction_end` after timeout (existing behavior)
- No changes to interaction logic or transform calculations

### 2. InteractiveCanvas Changes

**Change 1: Extract viewport update logic**

```rust
let update_viewport_from_transform = move |transform_result| {
    if let Some(canvas_el) = canvas_ref.get_untracked() {
        let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();
        let width = canvas.width();
        let height = canvas.height();

        set_viewport.update(|vp| {
            *vp = crate::rendering::apply_pixel_transform_to_viewport(
                vp,
                &natural_bounds.get_untracked(),
                &transform_result,
                width,
                height,
            );
        });
    }
};
```

**Change 2: Pass both callbacks**

```rust
let interaction = use_canvas_interaction(
    canvas_ref,
    update_viewport_from_transform.clone(), // During interaction
    update_viewport_from_transform,          // At end
);
```

**Change 3: Gate rendering**

```rust
create_effect(move |_| {
    let vp = viewport.get();
    canvas_renderer.track();
    let interacting = interaction.is_interacting.get();

    // Only render when not interacting
    if !interacting {
        if let Some(canvas_el) = canvas_ref.get() {
            let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();
            let start = web_sys::window().unwrap().performance().unwrap().now();
            canvas_renderer.with(|cr| cr.render(&vp, canvas));
            let elapsed = web_sys::window().unwrap().performance().unwrap().now() - start;
            set_render_time_ms.set(Some(elapsed));
        }
    }
});
```

### 3. App - No Changes Required

The existing effect (lines 101-110 in `app.rs`) already listens to viewport changes and updates `renderer_info`. This will now fire during interaction automatically.

## Performance Analysis

**Viewport update cost (runs during interaction):**
- Create info provider struct (~nanoseconds)
- Format 2 strings (center_display, zoom_display)
- One `log10()` calculation for max_iterations
- Total: ~microseconds

**Render cost (runs at end only):**
- Fractal computation across entire canvas
- Total: milliseconds to seconds (depending on zoom)

**Conclusion:** Updating viewport at 60fps is negligible compared to a single render.

## Key Properties

1. **UI updates live** - Viewport changes during interaction trigger renderer_info recalculation
2. **No redundant renders** - Expensive fractal rendering only happens when interaction ends
3. **Hook stays generic** - Just fires callbacks, doesn't know about viewports
4. **Clean separation** - InteractiveCanvas is the pixel↔viewport boundary layer
5. **Single source of truth** - One viewport signal, one render effect, one update logic

## Testing Strategy

**Manual testing:**
1. Start dragging - verify coordinates update in real-time
2. Continue dragging - verify no renders happen (check render time doesn't update)
3. Release - verify final render happens after 1.5s
4. Zoom with wheel - verify coordinates update during zoom
5. Verify pixel preview still works (existing behavior)

**Edge cases:**
- Rapid interaction (drag, release, drag again within 1.5s)
- Canvas resize during interaction
- Switching renderers during interaction

## Files Modified

1. `fractalwonder-ui/src/hooks/use_canvas_interaction.rs`
   - Add second callback parameter
   - Call `on_interaction` during interaction

2. `fractalwonder-ui/src/components/interactive_canvas.rs`
   - Extract viewport update logic
   - Pass two callbacks to hook
   - Gate rendering in effect

## Alternatives Considered

**Option A: Duplicate renderer_info effect in App**
- Con: App needs to understand pixel transforms (abstraction violation)
- Con: Duplicates viewport calculation logic

**Option B: Single effect with conditional logic in App**
- Con: Same abstraction violations as Option A
- Con: Complex effect with mixed concerns

**Option C (Chosen): Two callbacks in InteractiveCanvas**
- Pro: Clean abstraction boundaries
- Pro: No logic duplication
- Pro: Minimal hook change (additive only)

## Conclusion

This design achieves real-time UI updates with minimal changes, no performance cost, and clean separation of concerns. The hook remains generic and reusable, while InteractiveCanvas properly serves as the pixel↔viewport translation layer.
