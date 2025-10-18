use leptos::*;

#[component]
pub fn UI(is_visible: ReadSignal<bool>, set_is_hovering: WriteSignal<bool>) -> impl IntoView {
    let opacity_class = move || {
        if is_visible.get() {
            "opacity-100"
        } else {
            "opacity-0"
        }
    };

    view! {
      <div
        class=move || format!(
          "fixed inset-x-0 bottom-0 bg-black/50 backdrop-blur-sm px-4 py-3 transition-opacity duration-300 {}",
          opacity_class()
        )
        on:mouseenter=move |_| set_is_hovering.set(true)
        on:mouseleave=move |_| set_is_hovering.set(false)
      >
        <div class="text-center text-white text-sm">
          "Fractal Wonder - In Development"
        </div>
      </div>
    }
}
