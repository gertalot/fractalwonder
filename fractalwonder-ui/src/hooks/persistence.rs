// fractalwonder-ui/src/hooks/persistence.rs
//!
//! Browser persistence for viewport and config state.
//! Supports both localStorage and URL hash parameters.
//! Priority on load: URL hash > localStorage > defaults.
//! Enables users to continue exploring from their last position and share fractals via URL.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use fractalwonder_core::Viewport;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

const STORAGE_KEY: &str = "fractalwonder_state";
const URL_HASH_PREFIX: &str = "v1:";

/// State persisted to localStorage between sessions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedState {
    /// Current viewport (center, width, height with arbitrary precision)
    pub viewport: Viewport,
    /// Selected fractal configuration ID
    pub config_id: String,
    /// Schema version for future migrations
    version: u32,
}

impl PersistedState {
    const CURRENT_VERSION: u32 = 1;

    pub fn new(viewport: Viewport, config_id: String) -> Self {
        Self {
            viewport,
            config_id,
            version: Self::CURRENT_VERSION,
        }
    }
}

/// Load persisted state with priority: URL hash > localStorage > None.
/// Returns None if no state exists, parsing fails, or storage is unavailable.
pub fn load_state() -> Option<PersistedState> {
    // Priority 1: URL hash (allows sharing fractals via URL)
    if let Some(state) = load_from_url_hash() {
        return Some(state);
    }

    // Priority 2: localStorage (session persistence)
    load_from_local_storage()
}

/// Load state from localStorage only.
fn load_from_local_storage() -> Option<PersistedState> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let json = storage.get_item(STORAGE_KEY).ok()??;

    match serde_json::from_str::<PersistedState>(&json) {
        Ok(state) => {
            // Only accept current version (future: add migration logic)
            if state.version == PersistedState::CURRENT_VERSION {
                log::info!(
                    "Loaded persisted state from localStorage: config={}",
                    state.config_id
                );
                Some(state)
            } else {
                log::warn!(
                    "Ignoring localStorage state with version {} (current: {})",
                    state.version,
                    PersistedState::CURRENT_VERSION
                );
                None
            }
        }
        Err(e) => {
            log::warn!("Failed to parse localStorage state: {}", e);
            None
        }
    }
}

/// Save state to both localStorage and URL hash.
/// Logs a warning if saving fails (storage unavailable or quota exceeded).
pub fn save_state(state: &PersistedState) {
    // Save to localStorage (for session persistence)
    save_to_local_storage(state);

    // Save to URL hash (for bookmarking/sharing)
    save_to_url_hash(state);
}

/// Save state to localStorage only.
fn save_to_local_storage(state: &PersistedState) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };

    match serde_json::to_string(state) {
        Ok(json) => {
            if let Err(e) = storage.set_item(STORAGE_KEY, &json) {
                log::warn!("Failed to save state to localStorage: {:?}", e);
            }
        }
        Err(e) => {
            log::warn!("Failed to serialize state: {}", e);
        }
    }
}

/// Clear persisted state from localStorage and URL hash.
#[allow(dead_code)]
pub fn clear_state() {
    let Some(window) = web_sys::window() else {
        return;
    };

    // Clear localStorage
    if let Ok(Some(storage)) = window.local_storage() {
        let _ = storage.remove_item(STORAGE_KEY);
    }

    // Clear URL hash
    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(""));
    }

    log::info!("Cleared persisted state");
}

// =============================================================================
// URL Hash Encoding/Decoding
// =============================================================================

/// Encode state to a compressed, URL-safe string.
fn encode_state(state: &PersistedState) -> Option<String> {
    // Serialize to JSON
    let json = serde_json::to_string(state).ok()?;

    // Compress with deflate
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(json.as_bytes()).ok()?;
    let compressed = encoder.finish().ok()?;

    // Encode to URL-safe base64
    let encoded = URL_SAFE_NO_PAD.encode(&compressed);

    // Add version prefix
    Some(format!("{URL_HASH_PREFIX}{encoded}"))
}

/// Decode state from a compressed, URL-safe string.
fn decode_state(encoded: &str) -> Option<PersistedState> {
    // Check and strip version prefix
    let data = encoded.strip_prefix(URL_HASH_PREFIX)?;

    // Decode from base64
    let compressed = URL_SAFE_NO_PAD.decode(data).ok()?;

    // Decompress
    let mut decoder = DeflateDecoder::new(&compressed[..]);
    let mut json = String::new();
    decoder.read_to_string(&mut json).ok()?;

    // Deserialize
    let state: PersistedState = serde_json::from_str(&json).ok()?;

    // Validate version
    if state.version == PersistedState::CURRENT_VERSION {
        Some(state)
    } else {
        log::warn!(
            "Ignoring URL hash state with version {} (current: {})",
            state.version,
            PersistedState::CURRENT_VERSION
        );
        None
    }
}

/// Load state from URL hash fragment.
fn load_from_url_hash() -> Option<PersistedState> {
    let window = web_sys::window()?;
    let location = window.location();
    let hash = location.hash().ok()?;

    // Strip leading '#' if present
    let hash = hash.strip_prefix('#').unwrap_or(&hash);

    if hash.is_empty() {
        return None;
    }

    match decode_state(hash) {
        Some(state) => {
            log::info!("Loaded state from URL hash: config={}", state.config_id);
            Some(state)
        }
        None => {
            log::warn!("Failed to decode URL hash state");
            None
        }
    }
}

/// Save state to URL hash fragment (updates browser URL without navigation).
fn save_to_url_hash(state: &PersistedState) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let Some(encoded) = encode_state(state) else {
        log::warn!("Failed to encode state for URL hash");
        return;
    };

    // Use replaceState to update URL without adding to history
    if let Ok(history) = window.history() {
        let new_url = format!("#{encoded}");
        if let Err(e) =
            history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&new_url))
        {
            log::warn!("Failed to update URL hash: {:?}", e);
        }
    }
}

// =============================================================================
// Hashchange Listener Hook
// =============================================================================

use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Listen for hashchange events and call the callback with the new state.
/// The callback is wrapped in an Rc so it can be shared across closures.
pub fn use_hashchange_listener<F>(on_change: F)
where
    F: Fn(PersistedState) + 'static,
{
    use std::rc::Rc;

    // Store the closure so it lives for the component lifetime
    let handler_storage = store_value::<Option<Closure<dyn FnMut(web_sys::HashChangeEvent)>>>(None);

    // Wrap callback in Rc for sharing across closures
    let on_change = Rc::new(on_change);

    create_effect(move |_| {
        let on_change = Rc::clone(&on_change);

        let handler = Closure::wrap(Box::new(move |_e: web_sys::HashChangeEvent| {
            if let Some(state) = load_from_url_hash() {
                on_change(state);
            }
        }) as Box<dyn FnMut(web_sys::HashChangeEvent)>);

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("hashchange", handler.as_ref().unchecked_ref());
        }

        handler_storage.set_value(Some(handler));

        on_cleanup(move || {
            handler_storage.with_value(|handler_opt| {
                if let Some(handler) = handler_opt {
                    if let Some(window) = web_sys::window() {
                        let _ = window.remove_event_listener_with_callback(
                            "hashchange",
                            handler.as_ref().unchecked_ref(),
                        );
                    }
                }
            });
            handler_storage.set_value(None);
        });
    });
}

#[cfg(test)]
mod browser_tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_hashchange_triggers_callback() {
        use gloo_timers::future::TimeoutFuture;
        use leptos::*;

        // Create a runtime for signals
        let runtime = create_runtime();

        // Track if callback was called using a Cell (no reactive context needed)
        let callback_called = Rc::new(Cell::new(false));
        let callback_called_clone = Rc::clone(&callback_called);

        // Set up the listener
        use_hashchange_listener(move |_state| {
            callback_called_clone.set(true);
        });

        // Give the effect time to run and register the listener
        TimeoutFuture::new(10).await;

        // Create a valid encoded state to use as hash
        let test_viewport = fractalwonder_core::Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 64);
        let test_state = PersistedState::new(test_viewport, "mandelbrot".to_string());

        // Encode and set the hash, then dispatch event
        if let Some(encoded) = encode_state(&test_state) {
            let window = web_sys::window().unwrap();
            let _ = window.location().set_hash(&encoded);

            // Manually dispatch hashchange event
            let event = web_sys::HashChangeEvent::new("hashchange").unwrap();
            let _ = window.dispatch_event(&event);
        }

        // Give time for the event handler to run
        TimeoutFuture::new(10).await;

        // Check that callback was called
        assert!(
            callback_called.get(),
            "Hashchange callback should have been triggered"
        );

        runtime.dispose();
    }
}
