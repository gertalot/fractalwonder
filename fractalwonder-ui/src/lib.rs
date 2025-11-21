use leptos::*;
use wasm_bindgen::prelude::*;

mod components;
mod app;
use app::App;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    leptos::mount_to_body(App);
}
