use leptos::*;

const UI_HIDE_DELAY_MS: u64 = 2000;

/// UI visibility state returned by use_ui_visibility hook
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct UiVisibility {
    pub is_visible: ReadSignal<bool>,
    pub set_is_visible: WriteSignal<bool>,
    pub is_hovering: ReadSignal<bool>,
    pub set_is_hovering: WriteSignal<bool>,
}

pub fn use_ui_visibility() -> UiVisibility {
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

    UiVisibility {
        is_visible,
        set_is_visible,
        is_hovering,
        set_is_hovering,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_ui_hides_after_delay_when_not_hovering() {
        use wasm_bindgen_futures::JsFuture;
        use web_sys::window;

        let _runtime = leptos::create_runtime();
        let ui_vis = use_ui_visibility();

        // Initially visible
        assert!(
            ui_vis.is_visible.get_untracked(),
            "UI should be visible initially"
        );

        // Wait for the hide delay + buffer
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let win = window().expect("should have window");
            win.set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                (UI_HIDE_DELAY_MS + 500) as i32,
            )
            .expect("should set timeout");
        });
        JsFuture::from(promise).await.expect("timeout should resolve");

        // Should now be hidden (not hovering)
        assert!(
            !ui_vis.is_visible.get_untracked(),
            "UI should be hidden after delay when not hovering"
        );
    }

    #[wasm_bindgen_test]
    async fn test_ui_stays_visible_when_hovering() {
        use wasm_bindgen_futures::JsFuture;
        use web_sys::window;

        let _runtime = leptos::create_runtime();
        let ui_vis = use_ui_visibility();

        // Set hovering to true
        ui_vis.set_is_hovering.set(true);

        // Initially visible
        assert!(
            ui_vis.is_visible.get_untracked(),
            "UI should be visible initially"
        );

        // Wait for the hide delay + buffer
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let win = window().expect("should have window");
            win.set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                (UI_HIDE_DELAY_MS + 500) as i32,
            )
            .expect("should set timeout");
        });
        JsFuture::from(promise).await.expect("timeout should resolve");

        // Should still be visible (hovering)
        assert!(
            ui_vis.is_visible.get_untracked(),
            "UI should stay visible when hovering, even after delay"
        );
    }

    #[wasm_bindgen_test]
    async fn test_mouse_movement_makes_ui_visible_and_resets_timer() {
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{window, MouseEvent};

        let _runtime = leptos::create_runtime();
        let ui_vis = use_ui_visibility();

        // Wait for initial hide
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let win = window().expect("should have window");
            win.set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                (UI_HIDE_DELAY_MS + 500) as i32,
            )
            .expect("should set timeout");
        });
        JsFuture::from(promise).await.expect("timeout should resolve");

        // Should be hidden
        assert!(
            !ui_vis.is_visible.get_untracked(),
            "UI should be hidden after initial delay"
        );

        // Simulate mouse movement
        let win = window().expect("should have window");
        let mouse_event = MouseEvent::new("mousemove").expect("should create mouse event");
        win.dispatch_event(&mouse_event)
            .expect("should dispatch event");

        // Small delay to let event handler run
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let win = window().expect("should have window");
            win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 100)
                .expect("should set timeout");
        });
        JsFuture::from(promise).await.expect("timeout should resolve");

        // Should now be visible again
        assert!(
            ui_vis.is_visible.get_untracked(),
            "UI should be visible after mouse movement"
        );
    }
}
