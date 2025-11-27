// fractalwonder-core/src/compute_data.rs

use serde::{Deserialize, Serialize};

/// Data computed for a test image pixel.
/// All fields are bools derived from normalized coordinate comparisons.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestImageData {
    pub is_on_origin: bool,
    pub is_on_x_axis: bool,
    pub is_on_y_axis: bool,
    pub is_on_major_tick_x: bool,
    pub is_on_medium_tick_x: bool,
    pub is_on_minor_tick_x: bool,
    pub is_on_major_tick_y: bool,
    pub is_on_medium_tick_y: bool,
    pub is_on_minor_tick_y: bool,
    pub is_light_cell: bool,
}

impl Default for TestImageData {
    fn default() -> Self {
        Self {
            is_on_origin: false,
            is_on_x_axis: false,
            is_on_y_axis: false,
            is_on_major_tick_x: false,
            is_on_medium_tick_x: false,
            is_on_minor_tick_x: false,
            is_on_major_tick_y: false,
            is_on_medium_tick_y: false,
            is_on_minor_tick_y: false,
            is_light_cell: true,
        }
    }
}

/// Data computed for a Mandelbrot pixel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MandelbrotData {
    /// Number of iterations before escape (or max_iterations if didn't escape)
    pub iterations: u32,
    /// Maximum iterations used for this computation (for colorizer normalization)
    pub max_iterations: u32,
    /// Whether the point escaped the set
    pub escaped: bool,
    /// Whether this pixel was computed with a glitched reference orbit.
    /// When true, the colorizer can render this pixel distinctively (e.g., cyan overlay).
    #[serde(default)]
    pub glitched: bool,
}

/// Unified enum for all compute results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ComputeData {
    TestImage(TestImageData),
    Mandelbrot(MandelbrotData),
}
