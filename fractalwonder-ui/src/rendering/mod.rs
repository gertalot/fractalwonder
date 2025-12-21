mod canvas_utils;
pub mod colorizers;
mod parallel_renderer;
mod render_progress;
mod test_pattern;
mod tiles;

pub use canvas_utils::{draw_pixels_to_canvas, get_2d_context, performance_now, yield_to_browser};
pub use colorizers::Colorizer;
pub use parallel_renderer::ParallelRenderer;
pub use render_progress::RenderProgress;
// Only export what's still needed for tests
pub use test_pattern::{calculate_tick_params, calculate_tick_params_from_log2, TickParams};
pub use tiles::{calculate_tile_size, generate_tiles, tile_to_viewport};
