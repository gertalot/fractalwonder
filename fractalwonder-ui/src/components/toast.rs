//! Toast notification component for transient feedback.

use leptos::*;

/// Toast notification that appears briefly then fades out.
/// Only shows when UI panel is hidden.
#[component]
pub fn Toast(
    /// Message to display (None = hidden)
    message: Signal<Option<String>>,
    /// Whether UI panel is visible (toast hidden when true)
    ui_visible: Signal<bool>,
) -> impl IntoView {
    // Track visibility with fade animation
    let (is_visible, set_is_visible) = create_signal(false);
    let (display_message, set_display_message) = create_signal(String::new());

    // Handle message changes
    create_effect(move |_| {
        if let Some(msg) = message.get() {
            // Don't show if UI is visible
            if ui_visible.get_untracked() {
                return;
            }

            set_display_message.set(msg);
            set_is_visible.set(true);

            // Auto-hide after 1.5 seconds
            set_timeout(
                move || {
                    set_is_visible.set(false);
                },
                std::time::Duration::from_millis(1500),
            );
        }
    });

    view! {
        <div
            class=move || format!(
                "fixed bottom-12 left-1/2 -translate-x-1/2 z-50 \
                 px-4 py-2 rounded-lg \
                 bg-black/80 text-white text-sm font-medium \
                 transition-opacity duration-300 \
                 pointer-events-none {}",
                if is_visible.get() { "opacity-100" } else { "opacity-0" }
            )
        >
            {move || display_message.get()}
        </div>
    }
}
