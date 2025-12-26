//! Interactive curve editor for transfer and falloff curves.

#[allow(unused_imports)]
use crate::rendering::colorizers::{Curve, CurvePoint};
#[allow(unused_imports)]
use crate::rendering::get_2d_context;
use leptos::*;
#[allow(unused_imports)]
use web_sys::HtmlCanvasElement;

/// Interactive curve editor component.
#[component]
pub fn CurveEditor(
    /// The curve to edit (None when editor closed)
    curve: Signal<Option<Curve>>,
    /// Called when curve changes
    #[allow(unused_variables)]
    on_change: Callback<Curve>,
    /// Canvas size in logical pixels
    #[prop(default = 320)]
    #[allow(unused_variables)]
    size: u32,
) -> impl IntoView {
    view! {
        <Show when=move || curve.get().is_some()>
            <div class="bg-white/5 border border-white/10 rounded-lg p-4 space-y-2">
                <div class="text-white/50 text-xs">
                    "Transfer Curve (placeholder)"
                </div>
            </div>
        </Show>
    }
}
