// fractalwonder-ui/src/hooks/ui_visibility.rs
use leptos::*;

const UI_HIDE_DELAY_MS: f64 = 2000.0;

/// UI visibility state returned by use_ui_visibility hook
#[derive(Debug, Clone, Copy)]
pub struct UiVisibility {
    pub is_visible: ReadSignal<bool>,
    pub set_is_visible: WriteSignal<bool>,
    pub is_hovering: ReadSignal<bool>,
    pub set_is_hovering: WriteSignal<bool>,
}

/// Hook that manages UI panel visibility with autohide.
/// - Starts visible
/// - Hides after 2s of mouse inactivity (unless hovering over UI)
/// - Shows on any mouse movement
pub fn use_ui_visibility() -> UiVisibility {
    let (is_visible, set_is_visible) = create_signal(true);
    let (is_hovering, set_is_hovering) = create_signal(false);

    // Create a timer that fires after delay
    let timeout_fn = leptos_use::use_timeout_fn(
        move |_: ()| {
            // Only hide if not hovering over UI
            if !is_hovering.get_untracked() {
                set_is_visible.set(false);
            }
        },
        UI_HIDE_DELAY_MS,
    );

    // Start the initial timer immediately
    (timeout_fn.start)(());

    // Listen for mouse movement on the window
    let _ = leptos_use::use_event_listener(
        leptos_use::use_window(),
        leptos::ev::mousemove,
        move |_| {
            set_is_visible.set(true);
            // Cancel previous timer before starting new one
            (timeout_fn.stop)();
            (timeout_fn.start)(());
        },
    );

    UiVisibility {
        is_visible,
        set_is_visible,
        is_hovering,
        set_is_hovering,
    }
}
