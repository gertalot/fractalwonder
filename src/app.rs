use leptos::*;
use crate::components::test_image::TestImageView;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererType {
    TestImage,
}

#[component]
pub fn App() -> impl IntoView {
    // Currently fixed to TestImage
    let current_renderer = RendererType::TestImage;

    view! {
      <div class="relative w-screen h-screen overflow-hidden bg-black">
        {match current_renderer {
          RendererType::TestImage => {
            view! { <TestImageView /> }.into_view()
          }
        }}
      </div>
    }
}
