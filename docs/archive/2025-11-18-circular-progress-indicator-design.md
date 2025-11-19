# Circular Progress Indicator Design

## Overview

Add a circular pie chart progress indicator in the bottom-left corner that shows rendering progress when the UI panel is hidden. The indicator fades in/out inversely to the UI panel visibility.

## Requirements

- 24×24px circular pie chart
- Position: Aligned with info icon (28px from left, 24px from bottom)
- Progress displays clockwise from 12 o'clock (0% = top)
- Styling: `bg-black/50 backdrop-blur-sm` (matches UI panel), `rgb(244,244,244)` foreground
- Visibility: Fades in when UI panel fades out (inverse visibility, same 300ms transition)
- Shows only when: rendering in progress AND UI is hidden

## Design Decisions

### Approach: Pure SVG Pie Chart

**Selected approach:** SVG `<path>` element with calculated arc based on percentage

**Alternatives considered:**
1. CSS conic-gradient - simpler but less browser support and control
2. Canvas rendering - overkill for simple indicator

**Rationale:** SVG provides crisp rendering at any resolution, full styling control, and fits naturally into Leptos component model.

## Architecture

### Data Flow

```
RenderProgress (existing signal from app.rs)
  ↓
CircularProgress component
  ↓
progress_percent memo (0-100 calculation)
  ↓
create_pie_path() helper (SVG arc math)
  ↓
SVG <path> element rendering
```

### Component Structure

**File:** `fractalwonder-ui/src/components/circular_progress.rs`

**Props:**
```rust
#[component]
pub fn CircularProgress(
    progress: Signal<RenderProgress>,
    is_ui_visible: ReadSignal<bool>,
) -> impl IntoView
```

**Key computations:**
```rust
// Progress percentage
let progress_percent = create_memo(move |_| {
    let p = progress.get();
    if p.total_tiles > 0 {
        (p.completed_tiles as f64 / p.total_tiles as f64 * 100.0).min(100.0)
    } else {
        0.0
    }
});

// Visibility logic: show when rendering AND UI hidden
let should_show = create_memo(move |_| {
    let p = progress.get();
    p.total_tiles > 0 && !p.is_complete && !is_ui_visible.get()
});

let opacity_class = move || {
    if should_show.get() { "opacity-100" } else { "opacity-0" }
};
```

## Implementation Details

### SVG Path Generation

**Helper function:**
```rust
fn create_pie_path(percent: f64) -> String {
    if percent <= 0.0 {
        return String::new(); // No path for 0%
    }

    let angle = (percent / 100.0) * 360.0;
    let radians = (angle - 90.0).to_radians(); // -90 to start at 12 o'clock
    let end_x = 12.0 + 10.0 * radians.cos();
    let end_y = 12.0 + 10.0 * radians.sin();
    let large_arc = if angle > 180.0 { 1 } else { 0 };

    format!(
        "M 12 12 L 12 2 A 10 10 0 {} 1 {:.2} {:.2} Z",
        large_arc, end_x, end_y
    )
}
```

**Path explanation:**
- `M 12 12` - Move to center of 24×24 viewBox
- `L 12 2` - Line to top (12 o'clock starting point)
- `A 10 10 0 {large_arc} 1 {end_x} {end_y}` - Arc with radius 10
- `Z` - Close path back to center
- Large arc flag handles >180° correctly

### Component Rendering

```rust
view! {
    <div
        class=move || format!(
            "fixed left-[28px] bottom-[24px] transition-opacity duration-300 pointer-events-none {}",
            opacity_class()
        )
    >
        <div class="w-6 h-6 bg-black/50 backdrop-blur-sm rounded-full flex items-center justify-center">
            <svg width="24" height="24" viewBox="0 0 24 24" class="transform">
                // Background circle (unfilled portion)
                <circle
                    cx="12"
                    cy="12"
                    r="10"
                    fill="none"
                    stroke="rgb(100,100,100)"
                    stroke-width="1"
                    opacity="0.2"
                />

                // Progress pie slice
                <path
                    d={move || create_pie_path(progress_percent.get())}
                    fill="rgb(244,244,244)"
                />
            </svg>
        </div>
    </div>
}
```

### Positioning Logic

- `fixed` - Position relative to viewport (not UI panel)
- `left-[28px]` - Aligns with info icon center (16px panel padding + 12px half-icon)
- `bottom-[24px]` - Aligns with info icon center (12px panel padding + 12px half-icon)
- `pointer-events-none` - Don't block interactions

**Rationale:** When UI is hidden, position relative to viewport to align where the info icon would be if UI were visible.

### Styling

**Outer container:**
- Positioning and fade transition only

**Inner div (24×24px):**
- `bg-black/50` - Matches UI panel background
- `backdrop-blur-sm` - Matches UI panel blur
- `rounded-full` - Perfect circle
- `flex items-center justify-center` - Center SVG

**Fade transition:**
- `transition-opacity duration-300` - Same timing as UI panel
- Inverted visibility: shows when `!is_ui_visible`

## Integration

### Module Exports

**`fractalwonder-ui/src/components/mod.rs`:**
```rust
pub mod circular_progress;
// ... other modules

pub use circular_progress::CircularProgress;
// ... other exports
```

### App Integration

**`fractalwonder-ui/src/app.rs`:**
```rust
// Add import
use crate::components::CircularProgress;

// In view! macro:
<CircularProgress
    progress=progress_signal
    is_ui_visible=ui_visibility.is_visible
/>
```

**No changes needed to:**
- `use_ui_visibility` hook (already provides needed signal)
- Progress tracking system (already implemented)
- `RenderProgress` struct (already has needed data)

## Testing Strategy

### Unit Tests

**SVG path generation:**
- `create_pie_path(0.0)` → empty string
- `create_pie_path(25.0)` → quarter circle (3 o'clock)
- `create_pie_path(50.0)` → half circle (6 o'clock)
- `create_pie_path(100.0)` → full circle (large_arc=1)

**Progress calculation:**
- 0/100 tiles → 0%
- 50/100 tiles → 50%
- 100/100 tiles → 100%
- Edge case: 0 total → 0%

### Browser Verification

**Positioning:**
- Verify alignment with info icon location
- Check distance from viewport edges

**Fade transitions:**
- Render starts with UI visible → indicator hidden
- After 2s UI fades out → indicator fades in (both 300ms)
- Mouse movement → UI fades in → indicator fades out
- Verify smooth coordinated transitions

**Progress accuracy:**
- Visual fill matches percentage
- Clockwise from 12 o'clock
- Disappears when render completes

**Styling:**
- Background matches UI panel (black/50, backdrop-blur)
- Foreground color: rgb(244,244,244)
- Size: 24×24px

## Files Modified

- `fractalwonder-ui/src/components/circular_progress.rs` - New component
- `fractalwonder-ui/src/components/mod.rs` - Export component
- `fractalwonder-ui/src/app.rs` - Integrate component

## Edge Cases

**Fast renders (<2s):**
- UI doesn't fade out, so indicator never appears
- Expected behavior

**UI toggling during render:**
- Indicator and UI can both briefly be visible during 300ms transition
- Acceptable visual overlap

**Render completes:**
- `is_complete` flag causes indicator to fade out immediately
- Clean transition

**No progress data:**
- `total_tiles = 0` → indicator stays hidden
- Safe default behavior
