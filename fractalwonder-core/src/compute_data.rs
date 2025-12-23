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
    /// |z|² at escape for smooth iteration coloring. Interior points store 0.0.
    #[serde(default)]
    pub final_z_norm_sq: f32,
    /// Real part of z at escape (for derivative-based lighting)
    #[serde(default)]
    pub final_z_re: f32,
    /// Imaginary part of z at escape (for derivative-based lighting)
    #[serde(default)]
    pub final_z_im: f32,
    /// Real part of derivative ρ = dz/dc at escape
    #[serde(default)]
    pub final_derivative_re: f32,
    /// Imaginary part of derivative ρ = dz/dc at escape
    #[serde(default)]
    pub final_derivative_im: f32,
}

impl Default for MandelbrotData {
    fn default() -> Self {
        Self {
            iterations: 0,
            max_iterations: 0,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 0.0,
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
        }
    }
}

/// Unified enum for all compute results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ComputeData {
    TestImage(TestImageData),
    Mandelbrot(MandelbrotData),
}
