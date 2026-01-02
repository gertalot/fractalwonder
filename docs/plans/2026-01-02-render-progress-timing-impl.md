# Render Progress and Timing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Track all render phases (orbit, BLA, tiles, colorization) with accumulating elapsed time and phase-aware UI indicators.

**Architecture:** Add `RenderPhase` enum to `RenderProgress`, update phase at each transition point in worker_pool.rs and parallel_renderer.rs. Enhance `CircularProgress` with indeterminate rotating-arc animation. Update `UIPanel` to show phase-specific messages.

**Tech Stack:** Rust, Leptos 0.6, WebAssembly, Tailwind CSS, SVG animations

---

## Task 1: Update RenderProgress Data Model

**Files:**
- Modify: `fractalwonder-ui/src/rendering/render_progress.rs`

**Step 1: Add RenderPhase enum and update struct**

Replace the entire file content:

```rust
use crate::utils::canvas_utils::performance_now;

/// The current phase of a render operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderPhase {
    /// No render in progress.
    #[default]
    Idle,
    /// Computing reference orbit (single-threaded, can be slow at deep zoom).
    ComputingOrbit,
    /// Building BLA acceleration table.
    BuildingBla,
    /// Rendering tiles (CPU) or row-sets (GPU).
    Rendering,
    /// Running final colorization pipeline.
    Colorizing,
    /// Render complete.
    Complete,
}

impl RenderPhase {
    /// Returns true if this phase uses indeterminate progress (no step count).
    pub fn is_indeterminate(&self) -> bool {
        matches!(
            self,
            RenderPhase::ComputingOrbit | RenderPhase::BuildingBla | RenderPhase::Colorizing
        )
    }

    /// Returns the display label for this phase.
    pub fn label(&self) -> &'static str {
        match self {
            RenderPhase::Idle => "",
            RenderPhase::ComputingOrbit => "Computing orbit...",
            RenderPhase::BuildingBla => "Building BLA table...",
            RenderPhase::Rendering => "Rendering",
            RenderPhase::Colorizing => "Colorizing...",
            RenderPhase::Complete => "Rendered",
        }
    }
}

/// Progress information for ongoing renders.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderProgress {
    /// Current render phase.
    pub phase: RenderPhase,
    /// Completed steps (only meaningful during Rendering phase).
    pub completed_steps: u32,
    /// Total steps (only meaningful during Rendering phase).
    pub total_steps: u32,
    /// Elapsed time in milliseconds (accumulates from render start).
    pub elapsed_ms: f64,
    /// Timestamp when render started (for computing elapsed_ms).
    render_start_time: Option<f64>,
}

impl RenderProgress {
    /// Start a new render. Captures start time and sets phase to ComputingOrbit.
    pub fn start() -> Self {
        Self {
            phase: RenderPhase::ComputingOrbit,
            completed_steps: 0,
            total_steps: 0,
            elapsed_ms: 0.0,
            render_start_time: Some(performance_now()),
        }
    }

    /// Transition to a new phase, updating elapsed time.
    pub fn set_phase(&mut self, phase: RenderPhase) {
        self.update_elapsed();
        self.phase = phase;
    }

    /// Set total steps for the Rendering phase.
    pub fn set_total_steps(&mut self, total: u32) {
        self.total_steps = total;
        self.completed_steps = 0;
    }

    /// Increment completed steps and update elapsed time.
    pub fn increment_step(&mut self) {
        self.update_elapsed();
        self.completed_steps += 1;
    }

    /// Update elapsed time from render start.
    pub fn update_elapsed(&mut self) {
        if let Some(start) = self.render_start_time {
            self.elapsed_ms = performance_now() - start;
        }
    }

    /// Check if render is complete.
    pub fn is_complete(&self) -> bool {
        self.phase == RenderPhase::Complete
    }

    /// Check if currently rendering (any active phase).
    pub fn is_active(&self) -> bool {
        !matches!(self.phase, RenderPhase::Idle | RenderPhase::Complete)
    }

    /// Calculate completion percentage (0.0 to 100.0) for Rendering phase.
    pub fn percentage(&self) -> f32 {
        if self.total_steps == 0 {
            0.0
        } else {
            (self.completed_steps as f32 / self.total_steps as f32) * 100.0
        }
    }

    /// Reset to idle state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_begins_at_computing_orbit() {
        let progress = RenderProgress::start();
        assert_eq!(progress.phase, RenderPhase::ComputingOrbit);
        assert!(progress.render_start_time.is_some());
    }

    #[test]
    fn phase_transitions_update_elapsed() {
        let mut progress = RenderProgress::start();
        progress.set_phase(RenderPhase::Rendering);
        assert_eq!(progress.phase, RenderPhase::Rendering);
        // elapsed_ms should be >= 0 (timing depends on test speed)
        assert!(progress.elapsed_ms >= 0.0);
    }

    #[test]
    fn is_complete_checks_phase() {
        let mut progress = RenderProgress::start();
        assert!(!progress.is_complete());
        progress.set_phase(RenderPhase::Complete);
        assert!(progress.is_complete());
    }

    #[test]
    fn is_active_checks_phase() {
        let mut progress = RenderProgress::default();
        assert!(!progress.is_active()); // Idle

        progress.set_phase(RenderPhase::ComputingOrbit);
        assert!(progress.is_active());

        progress.set_phase(RenderPhase::Complete);
        assert!(!progress.is_active());
    }

    #[test]
    fn percentage_calculation() {
        let mut progress = RenderProgress::start();
        progress.set_total_steps(100);
        progress.completed_steps = 50;
        assert!((progress.percentage() - 50.0).abs() < 0.001);
    }

    #[test]
    fn percentage_zero_steps() {
        let progress = RenderProgress::start();
        assert!((progress.percentage() - 0.0).abs() < 0.001);
    }

    #[test]
    fn indeterminate_phases() {
        assert!(RenderPhase::ComputingOrbit.is_indeterminate());
        assert!(RenderPhase::BuildingBla.is_indeterminate());
        assert!(RenderPhase::Colorizing.is_indeterminate());
        assert!(!RenderPhase::Rendering.is_indeterminate());
        assert!(!RenderPhase::Idle.is_indeterminate());
        assert!(!RenderPhase::Complete.is_indeterminate());
    }

    #[test]
    fn phase_labels() {
        assert_eq!(RenderPhase::ComputingOrbit.label(), "Computing orbit...");
        assert_eq!(RenderPhase::BuildingBla.label(), "Building BLA table...");
        assert_eq!(RenderPhase::Rendering.label(), "Rendering");
        assert_eq!(RenderPhase::Colorizing.label(), "Colorizing...");
        assert_eq!(RenderPhase::Complete.label(), "Rendered");
    }
}
```

**Step 2: Run tests to verify**

Run: `cargo test --package fractalwonder-ui render_progress -- --nocapture`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/render_progress.rs
git commit -m "feat(progress): add RenderPhase enum and phase-aware progress tracking"
```

---

## Task 2: Update CircularProgress with Indeterminate Mode

**Files:**
- Modify: `fractalwonder-ui/src/components/circular_progress.rs`

**Step 1: Add rotating arc animation for indeterminate state**

Replace the entire file content:

```rust
use leptos::*;

use crate::rendering::{RenderPhase, RenderProgress};

/// Generate SVG path for pie chart based on percentage.
fn create_pie_path(percent: f64) -> String {
    if percent <= 0.0 {
        return String::new();
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

/// Generate SVG path for a 90-degree arc (used for indeterminate spinner).
fn create_arc_path() -> &'static str {
    // Arc from 12 o'clock (0°) to 3 o'clock (90°)
    // Start at top (12, 2), arc to right (22, 12)
    "M 12 2 A 10 10 0 0 1 22 12"
}

#[component]
pub fn CircularProgress(
    progress: Signal<RwSignal<RenderProgress>>,
    is_ui_visible: ReadSignal<bool>,
) -> impl IntoView {
    // Get current phase
    let current_phase = create_memo(move |_| {
        let progress_signal = progress.get();
        progress_signal.get().phase
    });

    // Calculate progress percentage (for determinate mode)
    let progress_percent = create_memo(move |_| {
        let progress_signal = progress.get();
        let p = progress_signal.get();
        if p.total_steps > 0 {
            (p.completed_steps as f64 / p.total_steps as f64 * 100.0).min(100.0)
        } else {
            0.0
        }
    });

    // Visibility: show when active AND UI is hidden
    let should_show = create_memo(move |_| {
        let progress_signal = progress.get();
        let p = progress_signal.get();
        p.is_active() && !is_ui_visible.get()
    });

    let opacity_class = move || {
        if should_show.get() {
            "opacity-100"
        } else {
            "opacity-0"
        }
    };

    // Check if current phase is indeterminate
    let is_indeterminate = move || current_phase.get().is_indeterminate();

    view! {
        <div
            class=move || format!(
                "fixed left-[28px] bottom-[24px] transition-opacity duration-300 pointer-events-none {}",
                opacity_class()
            )
        >
            <div class="w-6 h-6 bg-black/50 backdrop-blur-sm rounded-full flex items-center justify-center">
                <svg width="24" height="24" viewBox="0 0 24 24" class="transform">
                    // Background circle
                    <circle
                        cx="12"
                        cy="12"
                        r="10"
                        fill="none"
                        stroke="rgb(100,100,100)"
                        stroke-width="1"
                        opacity="0.2"
                    />

                    // Indeterminate: rotating arc
                    {move || is_indeterminate().then(|| view! {
                        <g class="animate-spin" style="transform-origin: center; animation-duration: 1s;">
                            <path
                                d=create_arc_path()
                                fill="none"
                                stroke="rgb(244,244,244)"
                                stroke-width="2"
                                stroke-linecap="round"
                            />
                        </g>
                    })}

                    // Determinate: pie slice
                    {move || (!is_indeterminate()).then(|| view! {
                        <path
                            d={move || create_pie_path(progress_percent.get())}
                            fill="rgb(244,244,244)"
                        />
                    })}
                </svg>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pie_path_zero_percent() {
        let path = create_pie_path(0.0);
        assert_eq!(path, "");
    }

    #[test]
    fn test_create_pie_path_25_percent() {
        let path = create_pie_path(25.0);
        // At 25%, we're at 3 o'clock (90 degrees from 12 o'clock)
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 0 1 22.00 12.00 Z"));
    }

    #[test]
    fn test_create_pie_path_50_percent() {
        let path = create_pie_path(50.0);
        // At 50%, we're at 6 o'clock (180 degrees from 12 o'clock)
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 0 1 12.00 22.00 Z"));
    }

    #[test]
    fn test_create_pie_path_75_percent() {
        let path = create_pie_path(75.0);
        // At 75%, we're at 9 o'clock (270 degrees from 12 o'clock)
        assert!(path.contains("M 12 12 L 12 2 A 10 10 0 1 1 2.00 12.00 Z"));
    }

    #[test]
    fn test_create_pie_path_100_percent() {
        let path = create_pie_path(100.0);
        // At 100%, large arc flag should be 1 (>180 degrees)
        assert!(path.contains("A 10 10 0 1 1"));
    }

    #[test]
    fn test_arc_path_format() {
        let path = create_arc_path();
        assert!(path.starts_with("M 12 2"));
        assert!(path.contains("A 10 10"));
    }
}
```

**Step 2: Run tests to verify**

Run: `cargo test --package fractalwonder-ui circular_progress -- --nocapture`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/circular_progress.rs
git commit -m "feat(ui): add indeterminate rotating-arc animation to circular progress"
```

---

## Task 3: Update UIPanel Phase Display

**Files:**
- Modify: `fractalwonder-ui/src/components/ui_panel.rs:205-226`

**Step 1: Update the render progress display logic**

Find lines 205-226 (the render progress span) and replace with:

```rust
                    <div class="mt-1 flex items-center justify-center gap-2">
                        <span>
                            {move || {
                                let progress_signal = render_progress.get();
                                let progress = progress_signal.get();

                                match progress.phase {
                                    crate::rendering::RenderPhase::Idle => String::new(),
                                    crate::rendering::RenderPhase::ComputingOrbit
                                    | crate::rendering::RenderPhase::BuildingBla
                                    | crate::rendering::RenderPhase::Colorizing => {
                                        // Indeterminate phases: show label + elapsed time
                                        format!(
                                            "{} ({:.1}s)",
                                            progress.phase.label(),
                                            progress.elapsed_ms / 1000.0
                                        )
                                    }
                                    crate::rendering::RenderPhase::Rendering => {
                                        // Determinate phase: show progress + elapsed time
                                        format!(
                                            "{}: {}/{} ({:.1}s)",
                                            progress.phase.label(),
                                            progress.completed_steps,
                                            progress.total_steps,
                                            progress.elapsed_ms / 1000.0
                                        )
                                    }
                                    crate::rendering::RenderPhase::Complete => {
                                        // Complete: show total time
                                        format!(
                                            "{} in {:.2}s",
                                            progress.phase.label(),
                                            progress.elapsed_ms / 1000.0
                                        )
                                    }
                                }
                            }}
                        </span>
                        // Cancel button - only visible during active render
                        {move || {
                            let progress_signal = render_progress.get();
                            let progress = progress_signal.get();

                            if progress.is_active() {
                                view! {
                                    <button
                                        class="text-white/50 hover:text-white/90 transition-colors cursor-pointer text-sm leading-none"
                                        on:click=move |_| on_cancel.call(())
                                        title="Cancel render"
                                    >
                                        "×"
                                    </button>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
```

**Step 2: Verify compilation**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/components/ui_panel.rs
git commit -m "feat(ui): update panel to show phase-aware progress messages"
```

---

## Task 4: Update WorkerPool Phase Transitions (CPU Path)

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 4.1: Update start_perturbation_render to set phase**

Find the `start_perturbation_render` function (around line 483). After the line:
```rust
self.render_start_time = Some(performance_now());
```

Remove these lines:
```rust
self.progress
    .set(RenderProgress::new(self.pending_tiles.len() as u32));
```

Replace with:
```rust
self.progress.set(RenderProgress::start());
```

**Step 4.2: Update handle_orbit_complete to transition phases**

Find the `handle_orbit_complete` function (around line 285). After logging "Reference orbit complete", add phase transition:

After the line:
```rust
self.pending_orbit_data = Some(orbit_data.clone());
```

Add:
```rust
// Transition to BLA phase (or skip to Rendering if BLA disabled)
if self.perturbation.bla_enabled() && !self.gpu_mode {
    self.progress.update(|p| p.set_phase(RenderPhase::BuildingBla));
}
```

**Step 4.3: Update tile dispatch to transition to Rendering phase**

Find the section where tiles start dispatching (around line 404, after "All N workers have orbit"). Before dispatching tiles, add:

```rust
self.progress.update(|p| {
    p.set_phase(RenderPhase::Rendering);
    p.set_total_steps(self.pending_tiles.len() as u32);
});
```

**Step 4.4: Update tile completion to use increment_step**

Find the tile completion handler (around line 244). Replace:
```rust
self.progress.update(|p| {
    p.completed_steps += 1;
    p.elapsed_ms = elapsed;
    p.is_complete = p.completed_steps >= p.total_steps;
    complete = p.is_complete;
});
```

With:
```rust
self.progress.update(|p| {
    p.increment_step();
    complete = p.completed_steps >= p.total_steps;
});
```

**Step 4.5: Add RenderPhase import at top of file**

Add to imports:
```rust
use crate::rendering::RenderPhase;
```

**Step 4.6: Verify compilation**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 4.7: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "feat(progress): add phase transitions in WorkerPool for CPU path"
```

---

## Task 5: Update ParallelRenderer Phase Transitions (GPU Path + Colorization)

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 5.1: Update start_progressive_gpu_render**

Find `start_progressive_gpu_render` (around line 249). Replace:
```rust
self.progress.set(RenderProgress::new(row_set_count));
```

With:
```rust
self.progress.set(RenderProgress::start());
```

**Step 5.2: Add phase transition when orbit callback triggers**

Inside the orbit complete callback (around line 283), after logging "Orbit ready", add:
```rust
// Transition to Rendering phase
progress.update(|p| {
    p.set_phase(RenderPhase::Rendering);
    p.set_total_steps(row_set_count);
});
```

Note: The BLA table is computed in worker_pool.rs for GPU mode too, so that transition happens there.

**Step 5.3: Update row-set completion to use increment_step**

Find the progress update in schedule_row_set (around line 565). Replace:
```rust
progress.update(|p| {
    p.completed_steps += 1;
    p.elapsed_ms = elapsed_ms;
    p.is_complete = is_final;
});
```

With:
```rust
progress.update(|p| {
    p.increment_step();
});
```

**Step 5.4: Add Colorizing phase before colorize_final**

In the `if is_final` block (around line 571), before calling `colorize_final`, add:
```rust
// Transition to Colorizing phase
progress.update(|p| p.set_phase(RenderPhase::Colorizing));
```

**Step 5.5: Set Complete phase after colorize_final**

After `draw_full_frame` is called (around line 593), add:
```rust
// Render complete
progress.update(|p| p.set_phase(RenderPhase::Complete));
```

**Step 5.6: Update CPU render_complete_callback with Colorizing phase**

Find the `set_render_complete_callback` closure (around line 95). Wrap the colorization in phase transitions:

```rust
worker_pool.borrow().set_render_complete_callback(move || {
    // Transition to Colorizing phase
    progress_complete.update(|p| p.set_phase(RenderPhase::Colorizing));

    let ctx_ref = canvas_ctx_complete.borrow();
    let Some(ctx) = ctx_ref.as_ref() else {
        return;
    };

    // ... existing tile assembly and colorization code ...

    // Render complete
    progress_complete.update(|p| p.set_phase(RenderPhase::Complete));
});
```

Note: You'll need to clone `progress` for this callback. Add near line 90:
```rust
let progress_complete = progress;
```

**Step 5.7: Add RenderPhase import**

Add to imports at top:
```rust
use crate::rendering::RenderPhase;
```

**Step 5.8: Verify compilation**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 5.9: Commit**

```bash
git add fractalwonder-ui/src/rendering/parallel_renderer.rs
git commit -m "feat(progress): add phase transitions in ParallelRenderer for GPU path and colorization"
```

---

## Task 6: Update WorkerPool GPU Mode BLA Phase Transition

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 6.1: Add BLA phase transition for GPU mode**

Find the GPU mode BLA computation section in `handle_orbit_complete` (around line 318). Before computing BLA table, add:
```rust
self.progress.update(|p| p.set_phase(RenderPhase::BuildingBla));
```

**Step 6.2: Verify compilation**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 6.3: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "feat(progress): add BLA phase transition for GPU mode"
```

---

## Task 7: Update Cancel Behavior

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`

**Step 7.1: Update cancel to reset progress properly**

Find the `cancel` function (around line 543). Replace:
```rust
self.progress.update(|p| {
    p.is_complete = true;
});
```

With:
```rust
self.progress.update(|p| p.reset());
```

**Step 7.2: Verify compilation**

Run: `cargo check --package fractalwonder-ui`
Expected: No errors

**Step 7.3: Commit**

```bash
git add fractalwonder-ui/src/workers/worker_pool.rs
git commit -m "fix(progress): reset progress to Idle on cancel"
```

---

## Task 8: Run Full Test Suite

**Step 1: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`
Expected: All tests PASS

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`
Expected: No warnings or errors

**Step 3: Check formatting**

Run: `cargo fmt --all -- --check`
Expected: No formatting issues (run `cargo fmt --all` if needed)

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix lints and formatting"
```

---

## Task 9: Browser Testing

**Step 1: Verify trunk serve is running**

Open http://localhost:8080 in Chrome

**Step 2: Test deep zoom render**

1. Navigate to a deep zoom location (10^100+)
2. Observe: "Computing orbit... (X.Xs)" appears immediately
3. Observe: Circular progress shows rotating arc (when UI hidden)
4. Observe: "Building BLA table... (X.Xs)" appears after orbit completes
5. Observe: "Rendering: X/16 (X.Xs)" shows progress
6. Observe: "Colorizing... (X.Xs)" appears briefly
7. Observe: "Rendered in X.XXs" shows total time

**Step 3: Test cancel during orbit**

1. Start a deep zoom render
2. Click cancel during "Computing orbit..."
3. Verify: Progress resets, no error messages

**Step 4: Commit verification notes**

```bash
git add -A
git commit -m "test: verify phase-aware progress in browser"
```

---

## Summary

After completing all tasks:

1. `RenderProgress` tracks phases with accumulating elapsed time
2. `CircularProgress` shows rotating arc for indeterminate phases
3. `UIPanel` displays phase-specific messages with elapsed time
4. All render phases (orbit, BLA, tiles, colorization) are tracked
5. Total render time shown to user includes all phases
6. Cancel properly resets progress state
