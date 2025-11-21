use leptos::*;
use wasm_bindgen::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <div style="width: 100vw; height: 100vh; display: flex; align-items: center; justify-content: center; background: #1a1a1a; color: white;">
            <h1>"Fractal Wonder - Stage 0"</h1>
        </div>
    }
}

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    leptos::mount_to_body(App);
}
