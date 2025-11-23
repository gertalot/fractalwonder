mod fullscreen;
mod use_canvas_interaction;

pub use fullscreen::{toggle_fullscreen, use_fullscreen};
pub use use_canvas_interaction::{use_canvas_interaction, InteractionHandle};

// Re-export PixelTransform for convenience (so users can import from hooks module)
pub use fractalwonder_core::PixelTransform;
