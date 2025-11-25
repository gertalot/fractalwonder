pub mod colorizers;
mod render_progress;
mod test_pattern;
mod tiles;

pub use colorizers::{colorize_mandelbrot, colorize_test_image};
pub use render_progress::RenderProgress;
// Only export what's still needed for tests
pub use test_pattern::{calculate_tick_params, calculate_tick_params_from_log2, TickParams};
pub use tiles::{calculate_tile_size, generate_tiles};
