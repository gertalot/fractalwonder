use crate::rendering::{Viewport, RENDER_CONFIGS};
use leptos::window;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const STORAGE_KEY: &str = "fractal_wonder_state";

#[derive(Clone, Serialize, Deserialize)]
pub struct RendererState {
    pub viewport: Viewport<f64>,
    pub color_scheme_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub selected_renderer_id: String,
    pub renderer_states: HashMap<String, RendererState>,
}

impl AppState {
    pub fn load() -> Self {
        window()
            .local_storage()
            .ok()
            .flatten()
            .and_then(|storage| storage.get_item(STORAGE_KEY).ok().flatten())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(storage) = window().local_storage().ok().flatten() {
            if let Ok(json) = serde_json::to_string(self) {
                let _ = storage.set_item(STORAGE_KEY, &json);
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        let mut renderer_states = HashMap::new();

        for config in RENDER_CONFIGS.iter() {
            let renderer = (config.create_renderer)();
            let natural_bounds = renderer.natural_bounds();

            renderer_states.insert(
                config.id.to_string(),
                RendererState {
                    viewport: Viewport::new(natural_bounds.center(), 1.0),
                    color_scheme_id: config.default_color_scheme_id.to_string(),
                },
            );
        }

        AppState {
            selected_renderer_id: "test_image".to_string(),
            renderer_states,
        }
    }
}
