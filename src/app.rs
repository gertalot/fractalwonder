use leptos::*;

use crate::components::{test_image::TestImageView, ui::UI, ui_visibility::use_ui_visibility};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererType {
    TestImage,
    // Future: Mandelbrot, Julia, BurningShip, etc.
}

#[component]
pub fn App() -> impl IntoView {
    // UI visibility state
    let (is_visible, _set_is_visible, _is_hovering, set_is_hovering) = use_ui_visibility();

    // UI controls this signal (currently fixed to TestImage)
    let (current_renderer, _set_current_renderer) = create_signal(RendererType::TestImage);

    view! {
      <div class="relative w-screen h-screen overflow-hidden bg-black">
        // Dynamic renderer switching
        {move || match current_renderer.get() {
          RendererType::TestImage => {
            view! { <TestImageView /> }.into_view()
          }
        }}

        <UI is_visible=is_visible set_is_hovering=set_is_hovering />
      </div>
    }
}
