// ABOUTME: Library entry point for the Fractal Wonder WASM application
// ABOUTME: Re-exports the main App component and provides the hydrate function for browser mounting

mod app;
mod components;

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
