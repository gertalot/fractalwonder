
use leptos::*;

use crate::components::{canvas::Canvas, ui::UI, ui_visibility::use_ui_visibility};

#[component]
pub fn App() -> impl IntoView {
    let (is_visible, _set_is_visible, _is_hovering, set_is_hovering) = use_ui_visibility();

    view! {
      <div class="h-screen w-screen overflow-hidden">
        <Canvas />
        <UI is_visible=is_visible set_is_hovering=set_is_hovering />
      </div>
    }
}
