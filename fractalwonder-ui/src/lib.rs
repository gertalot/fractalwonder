use leptos::*;
use wasm_bindgen::prelude::*;

mod app;
mod components;
pub mod config;
pub mod hooks;
pub mod rendering;

pub use config::{default_config, get_config, FractalConfig, FRACTAL_CONFIGS};

use app::App;

#[wasm_bindgen(start)]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    _ = console_log::init_with_level(log::Level::Debug);
    leptos::mount_to_body(App);
}
