use leptos::*;

use crate::components::{test_image::TestImageView, ui::UI, ui_visibility::use_ui_visibility};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererType {
    TestImage,
}

#[component]
pub fn App() -> impl IntoView {
    // UI visibility state
    let (is_visible, _set_is_visible, _is_hovering, set_is_hovering) = use_ui_visibility();

    // Currently fixed to TestImage
    let current_renderer = RendererType::TestImage;

    view! {
      <div class="relative w-screen h-screen overflow-hidden bg-black">
        // Renderer
        {match current_renderer {
          RendererType::TestImage => {
            view! { <TestImageView /> }.into_view()
          }
        }}

        <UI is_visible=is_visible set_is_hovering=set_is_hovering />
      </div>
    }
}
