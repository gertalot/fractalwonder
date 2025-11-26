mod fullscreen;
mod persistence;
mod ui_visibility;
mod use_canvas_interaction;

pub use fullscreen::{toggle_fullscreen, use_fullscreen};
pub use persistence::{load_state, save_state, PersistedState};
pub use ui_visibility::{use_ui_visibility, UiVisibility};
pub use use_canvas_interaction::{use_canvas_interaction, InteractionHandle};

// Re-export PixelTransform for convenience (so users can import from hooks module)
pub use fractalwonder_core::PixelTransform;
