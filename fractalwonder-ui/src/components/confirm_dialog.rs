//! Reusable confirmation dialog component.

use leptos::*;

/// A centered modal dialog for confirming destructive actions.
#[component]
pub fn ConfirmDialog(
    /// Whether the dialog is visible
    #[prop(into)]
    visible: Signal<bool>,
    /// Dialog title
    #[prop(into)]
    title: String,
    /// Dialog message
    #[prop(into)]
    message: String,
    /// Cancel button label (e.g., "Cancel")
    #[prop(into)]
    cancel_label: String,
    /// Confirm button label (e.g., "Delete", "Reset", "Continue")
    #[prop(into)]
    confirm_label: String,
    /// Called when cancel is clicked
    on_cancel: Callback<()>,
    /// Called when confirm is clicked
    on_confirm: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || visible.get()>
            // Backdrop
            <div
                class="fixed inset-0 z-[100] bg-black/50 backdrop-blur-sm flex items-center justify-center"
                on:click=move |_| on_cancel.call(())
            >
                // Dialog
                <div
                    class="bg-black/95 border border-white/10 rounded-lg p-4 max-w-sm mx-4 space-y-4"
                    on:click=|e| e.stop_propagation()
                >
                    <h3 class="text-white text-sm font-medium">{title.clone()}</h3>
                    <p class="text-gray-300 text-sm">{message.clone()}</p>
                    <div class="flex gap-2">
                        <button
                            class="flex-1 px-3 py-1.5 rounded-lg border border-white/20 text-white text-sm hover:bg-white/10 transition-colors"
                            on:click=move |_| on_cancel.call(())
                        >
                            {cancel_label.clone()}
                        </button>
                        <button
                            class="flex-1 px-3 py-1.5 rounded-lg bg-white/20 text-white text-sm hover:bg-white/30 transition-colors"
                            on:click=move |_| on_confirm.call(())
                        >
                            {confirm_label.clone()}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
