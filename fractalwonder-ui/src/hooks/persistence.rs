// fractalwonder-ui/src/hooks/persistence.rs
//!
//! Browser localStorage persistence for viewport and config state.
//! Enables users to continue exploring from their last position after page reload.

use fractalwonder_core::Viewport;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "fractalwonder_state";

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

/// Load persisted state from localStorage.
/// Returns None if no state exists, parsing fails, or localStorage is unavailable.
pub fn load_state() -> Option<PersistedState> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let json = storage.get_item(STORAGE_KEY).ok()??;

    match serde_json::from_str::<PersistedState>(&json) {
        Ok(state) => {
            // Only accept current version (future: add migration logic)
            if state.version == PersistedState::CURRENT_VERSION {
                log::info!("Loaded persisted state: config={}", state.config_id);
                Some(state)
            } else {
                log::warn!(
                    "Ignoring persisted state with version {} (current: {})",
                    state.version,
                    PersistedState::CURRENT_VERSION
                );
                None
            }
        }
        Err(e) => {
            log::warn!("Failed to parse persisted state: {}", e);
            None
        }
    }
}

/// Save state to localStorage.
/// Logs a warning if saving fails (localStorage unavailable or quota exceeded).
pub fn save_state(state: &PersistedState) {
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

/// Clear persisted state from localStorage.
#[allow(dead_code)]
pub fn clear_state() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    let _ = storage.remove_item(STORAGE_KEY);
    log::info!("Cleared persisted state");
}
