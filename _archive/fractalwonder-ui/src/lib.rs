mod app;
mod components;
pub mod hooks;
pub mod rendering;
pub mod state;
pub mod workers;

use leptos::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| {
        view! {
          <app::App />
        }
    });
}
