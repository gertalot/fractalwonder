// fractalwonder-ui/src/app.rs
use leptos::*;

use crate::components::{InteractiveCanvas, UIPanel};

#[component]
pub fn App() -> impl IntoView {
    let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

    let on_resize = Callback::new(move |size: (u32, u32)| {
        set_canvas_size.set(size);
    });

    view! {
        <InteractiveCanvas on_resize=on_resize />
        <UIPanel canvas_size=canvas_size.into() />
    }
}
