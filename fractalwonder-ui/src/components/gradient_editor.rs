//! Interactive gradient editor with color stops, midpoints, and zoom.

use crate::rendering::colorizers::Gradient;
use leptos::*;

/// Interactive gradient editor component.
#[component]
pub fn GradientEditor(
    /// The gradient to edit (None when editor closed)
    gradient: Signal<Option<Gradient>>,
    /// Called when gradient changes (on mouse release)
    _on_change: Callback<Gradient>,
) -> impl IntoView {
    // Internal state
    let selected_stop = create_rw_signal(None::<usize>);
    let zoom = create_rw_signal(1.0_f64);
    let _is_dragging = create_rw_signal(false);

    view! {
        <Show when=move || gradient.get().is_some()>
            <div class="space-y-2">
                // Zoom controls (hidden at 1x)
                <Show when=move || { zoom.get() > 1.0 }>
                    <div class="flex items-center justify-between px-1">
                        <span class="text-white/50 text-xs">
                            {move || format!("Zoom: {:.1}x", zoom.get())}
                        </span>
                    </div>
                </Show>

                // Placeholder for gradient bar
                <div class="h-8 bg-white/10 rounded border border-white/20">
                    <span class="text-white/50 text-xs p-2">"Gradient bar placeholder"</span>
                </div>

                // Placeholder for color picker
                <Show when=move || selected_stop.get().is_some()>
                    <div class="bg-white/5 border border-white/10 rounded p-2">
                        <span class="text-white/50 text-xs">"Color picker placeholder"</span>
                    </div>
                </Show>
            </div>
        </Show>
    }
}
