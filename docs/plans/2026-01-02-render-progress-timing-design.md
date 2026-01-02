# Render Progress and Timing Improvements

## Problem

The UI only shows progress and timing for the tile/row-set rendering phase, ignoring:
- Reference orbit computation (can take 18+ seconds at deep zoom)
- BLA table computation
- Final colorization pipeline (can take 5+ seconds on large canvases)

Users see nothing happening during orbit computation, then get a final time that doesn't reflect their actual wait.

## Solution

Track four explicit render phases with accumulating elapsed time.

### Render Phases

```
Idle → ComputingOrbit → BuildingBla → Rendering → Colorizing → Complete
       (indeterminate)  (indeterminate) (determinate) (indeterminate)
```

**Phase transitions:**
- `Idle` → `ComputingOrbit`: Render triggered
- `ComputingOrbit` → `BuildingBla`: Worker sends `ReferenceOrbitComplete`
- `BuildingBla` → `Rendering`: BLA table built (GPU) or tiles dispatched (CPU, skip BLA phase if disabled)
- `Rendering` → `Colorizing`: All tiles/row-sets complete
- `Colorizing` → `Complete`: `colorize_final()` finished

### Data Model

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RenderPhase {
    #[default]
    Idle,
    ComputingOrbit,
    BuildingBla,
    Rendering,
    Colorizing,
    Complete,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderProgress {
    pub phase: RenderPhase,
    pub completed_steps: u32,           // Only during Rendering phase
    pub total_steps: u32,               // Only during Rendering phase
    pub elapsed_ms: f64,                // Accumulating from render start
    pub render_start_time: Option<f64>, // Captured at render trigger
}
```

### UI Display

**UI panel text (during render):**
- `"Computing orbit... (12.3s)"`
- `"Building BLA table... (18.1s)"`
- `"Rendering: 5/16 (19.5s)"`
- `"Colorizing... (22.0s)"`

**UI panel text (after completion):**
- `"Rendered in 22.2s"`

**Circular progress indicator:**
- Indeterminate phases: rotating arc animation (modern, smooth)
- Determinate phase: pie chart (current behavior)
- Same size and shape as current component

### Animation Specification

The rotating arc for indeterminate progress:
- Single arc spanning ~90° of the circle
- Continuous rotation around the circle
- Smooth CSS animation
- Matches existing component dimensions

## Files to Modify

| File | Changes |
|------|---------|
| `render_progress.rs` | Add `RenderPhase` enum, update struct |
| `worker_pool.rs` | Set phases at transitions, capture start time |
| `parallel_renderer.rs` | Set phases at GPU transitions, colorization |
| `ui_panel.rs` | Display phase-aware text with elapsed time |
| `circular_progress.rs` | Add indeterminate mode with rotating arc |

## Testing

- Verify phase transitions occur in correct order
- Verify elapsed time accumulates across phases
- Verify final time matches user's actual wait
- Verify circular progress shows correct mode per phase
- Test both CPU and GPU render paths
- Test cancel behavior resets to Idle
