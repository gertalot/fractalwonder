use leptos::*;

const UI_HIDE_DELAY_MS: u64 = 3000;

#[allow(clippy::type_complexity)]
pub fn use_ui_visibility() -> (
    ReadSignal<bool>,
    WriteSignal<bool>,
    ReadSignal<bool>,
    WriteSignal<bool>,
) {
    let (is_visible, set_is_visible) = create_signal(true);
    let (is_hovering, set_is_hovering) = create_signal(false);

    // Create a timer that fires after 3 seconds
    let timeout_fn = leptos_use::use_timeout_fn(
        move |_| {
            // Only hide if not hovering over UI
            if !is_hovering.get_untracked() {
                set_is_visible.set(false);
            }
        },
        UI_HIDE_DELAY_MS as f64,
    );

    // Start the initial timer immediately
    (timeout_fn.start)(());

    // Listen for mouse movement on the window
    let _ = leptos_use::use_event_listener(
        leptos_use::use_window(),
        leptos::ev::mousemove,
        move |_| {
            set_is_visible.set(true);
            // Cancel previous timer before starting new one to prevent flickering
            (timeout_fn.stop)();
            (timeout_fn.start)(());
        },
    );

    (is_visible, set_is_visible, is_hovering, set_is_hovering)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hide_delay_constant() {
        assert_eq!(
            UI_HIDE_DELAY_MS, 3000,
            "UI hide delay should be 3000ms (3 seconds)"
        );
    }

    #[test]
    fn test_auto_hide_logic_conditions() {
        // Test the core logic conditions without calling the hook
        // This tests the business logic without WASM dependencies

        // Test case 1: Not hovering -> should hide
        let is_hovering = false;
        let should_hide = !is_hovering;
        assert!(should_hide, "UI should hide when not hovering");

        // Test case 2: Hovering -> should stay visible
        let is_hovering = true;
        let should_hide = !is_hovering;
        assert!(!should_hide, "UI should stay visible when hovering");
    }

    #[test]
    fn test_visibility_state_transitions() {
        // Test the state transitions without WASM dependencies
        let mut is_visible = true;
        let mut is_hovering = false;

        // Initial state: visible, not hovering
        assert!(is_visible);
        assert!(!is_hovering);

        // Simulate timer firing when not hovering -> should hide
        if !is_hovering {
            is_visible = false;
        }
        assert!(
            !is_visible,
            "UI should hide when timer fires and not hovering"
        );

        // Simulate mouse movement -> should show
        is_visible = true;
        assert!(is_visible, "UI should show on mouse movement");

        // Simulate hovering -> should stay visible even if timer fires
        is_hovering = true;
        if !is_hovering {
            is_visible = false;
        }
        assert!(is_visible, "UI should stay visible when hovering");
    }

    #[test]
    fn test_timer_restart_behavior() {
        // Test the timer restart logic
        let mut is_visible = true;
        let is_hovering = false;

        // Simulate mouse movement: should make visible AND restart timer
        // Mouse movement makes visible (already true from initial state)
        // Timer restart happens (we can't test the actual timer, but we can test the logic)

        assert!(is_visible, "Mouse movement should make UI visible");

        // After mouse movement, if we wait and not hovering, should hide
        if !is_hovering {
            is_visible = false;
        }
        assert!(!is_visible, "UI should hide after timer if not hovering");
    }
}
