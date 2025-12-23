//! Perturbation rendering coordination and glitch resolution.

mod glitch_resolution;
mod helpers;

pub use glitch_resolution::GlitchResolver;
pub use helpers::{calculate_dc_max, calculate_render_max_iterations, validate_viewport};
