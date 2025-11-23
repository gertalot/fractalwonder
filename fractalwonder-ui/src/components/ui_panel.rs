// fractalwonder-ui/src/components/ui_panel.rs
use crate::components::{FullscreenButton, InfoButton};
use crate::hooks::{use_ui_visibility, UiVisibility};
use leptos::*;

#[component]
pub fn UIPanel(
    /// Canvas dimensions to display, as (width, height)
    canvas_size: Signal<(u32, u32)>,
) -> impl IntoView {
    let UiVisibility {
        is_visible,
        is_hovering: _,
        set_is_visible: _,
        set_is_hovering,
    } = use_ui_visibility();

    let opacity_class = move || {
        if is_visible.get() {
            "opacity-100"
        } else {
            "opacity-0 pointer-events-none"
        }
    };

    view! {
        <div
            class=move || format!(
                "fixed inset-x-0 bottom-0 transition-opacity duration-300 {}",
                opacity_class()
            )
            on:mouseenter=move |_| set_is_hovering.set(true)
            on:mouseleave=move |_| set_is_hovering.set(false)
        >
            <div class="flex items-center justify-between px-4 py-3 bg-black/50 backdrop-blur-sm">
                // Left section: info button
                <div class="flex items-center space-x-2">
                    <InfoButton />
                </div>

                // Center section: canvas dimensions
                <div class="flex-1 text-center text-white text-sm">
                    {move || {
                        let (w, h) = canvas_size.get();
                        format!("Canvas: {} × {}", w, h)
                    }}
                </div>

                // Right section: fullscreen
                <div>
                    <FullscreenButton />
                </div>
            </div>
        </div>
    }
}
