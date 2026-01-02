use super::canvas_utils::performance_now;

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

    // === Backward compatibility methods (to be removed when other tasks update callers) ===

    /// Create progress tracker with total steps (starts in Rendering phase).
    /// Deprecated: Use `start()` and `set_total_steps()` for proper phase tracking.
    pub fn new(total_steps: u32) -> Self {
        Self {
            phase: RenderPhase::Rendering,
            completed_steps: 0,
            total_steps,
            elapsed_ms: 0.0,
            render_start_time: Some(performance_now()),
        }
    }

    /// Mark render as complete.
    /// Deprecated: Use `set_phase(RenderPhase::Complete)` instead.
    pub fn set_complete(&mut self) {
        self.set_phase(RenderPhase::Complete);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a progress instance for testing without browser APIs.
    /// Uses default() and manually sets fields to avoid performance_now() calls.
    fn test_progress() -> RenderProgress {
        RenderProgress {
            phase: RenderPhase::ComputingOrbit,
            completed_steps: 0,
            total_steps: 0,
            elapsed_ms: 0.0,
            render_start_time: Some(0.0), // Mock start time
        }
    }

    #[test]
    fn default_is_idle() {
        let progress = RenderProgress::default();
        assert_eq!(progress.phase, RenderPhase::Idle);
        assert!(progress.render_start_time.is_none());
    }

    #[test]
    fn phase_transitions() {
        let mut progress = test_progress();
        assert_eq!(progress.phase, RenderPhase::ComputingOrbit);

        progress.phase = RenderPhase::Rendering;
        assert_eq!(progress.phase, RenderPhase::Rendering);

        progress.phase = RenderPhase::Complete;
        assert_eq!(progress.phase, RenderPhase::Complete);
    }

    #[test]
    fn is_complete_checks_phase() {
        let mut progress = test_progress();
        assert!(!progress.is_complete());

        progress.phase = RenderPhase::Complete;
        assert!(progress.is_complete());
    }

    #[test]
    fn is_active_checks_phase() {
        let mut progress = RenderProgress::default();
        assert!(!progress.is_active()); // Idle

        progress.phase = RenderPhase::ComputingOrbit;
        assert!(progress.is_active());

        progress.phase = RenderPhase::Rendering;
        assert!(progress.is_active());

        progress.phase = RenderPhase::Complete;
        assert!(!progress.is_active());
    }

    #[test]
    fn percentage_calculation() {
        let mut progress = test_progress();
        progress.total_steps = 100;
        progress.completed_steps = 50;
        assert!((progress.percentage() - 50.0).abs() < 0.001);
    }

    #[test]
    fn percentage_zero_steps() {
        let progress = test_progress();
        assert!((progress.percentage() - 0.0).abs() < 0.001);
    }

    #[test]
    fn percentage_complete() {
        let mut progress = test_progress();
        progress.total_steps = 64;
        progress.completed_steps = 64;
        assert!((progress.percentage() - 100.0).abs() < 0.001);
    }

    #[test]
    fn set_total_steps_resets_completed() {
        let mut progress = test_progress();
        progress.completed_steps = 10;
        progress.set_total_steps(100);
        assert_eq!(progress.total_steps, 100);
        assert_eq!(progress.completed_steps, 0);
    }

    #[test]
    fn increment_step() {
        let mut progress = test_progress();
        progress.total_steps = 10;
        assert_eq!(progress.completed_steps, 0);

        progress.completed_steps += 1;
        assert_eq!(progress.completed_steps, 1);
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
        assert_eq!(RenderPhase::Idle.label(), "");
    }

    #[test]
    fn reset_returns_to_default() {
        let mut progress = test_progress();
        progress.phase = RenderPhase::Rendering;
        progress.completed_steps = 50;
        progress.total_steps = 100;

        progress.reset();

        assert_eq!(progress.phase, RenderPhase::Idle);
        assert_eq!(progress.completed_steps, 0);
        assert_eq!(progress.total_steps, 0);
        assert!(progress.render_start_time.is_none());
    }
}
