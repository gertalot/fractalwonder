// fractalwonder-ui/src/hooks/fullscreen.rs
use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::{window, Document};

/// Toggle fullscreen mode for the document
pub fn toggle_fullscreen() {
    if let Some(window) = window() {
        if let Some(document) = window.document() {
            if is_fullscreen(&document) {
                document.exit_fullscreen();
            } else if let Some(element) = document.document_element() {
                let _ = element.request_fullscreen();
            }
        }
    }
}

/// Check if currently in fullscreen mode
fn is_fullscreen(document: &Document) -> bool {
    document.fullscreen_element().is_some()
}

/// Leptos hook to track fullscreen state reactively.
/// Returns (is_fullscreen signal, toggle function).
pub fn use_fullscreen() -> (ReadSignal<bool>, impl Fn()) {
    let (is_fullscreen_signal, set_is_fullscreen) = create_signal(false);

    create_effect(move |_| {
        if let Some(win) = window() {
            if let Some(document) = win.document() {
                // Set initial state
                set_is_fullscreen.set(document.fullscreen_element().is_some());

                let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Some(doc) = window().and_then(|w| w.document()) {
                        set_is_fullscreen.set(doc.fullscreen_element().is_some());
                    }
                }) as Box<dyn FnMut(_)>);

                let _ = document.add_event_listener_with_callback(
                    "fullscreenchange",
                    closure.as_ref().unchecked_ref(),
                );

                closure.forget();
            }
        }
    });

    (is_fullscreen_signal, toggle_fullscreen)
}
