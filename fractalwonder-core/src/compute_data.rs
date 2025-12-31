// fractalwonder-core/src/compute_data.rs

use serde::{Deserialize, Serialize};

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
    /// Pre-computed normalized surface direction for 3D lighting (real component).
    /// This is the normalized z/ρ direction, always in [-1, 1].
    /// Computed at render time to avoid f32 overflow at deep zooms.
    #[serde(default)]
    pub surface_normal_re: f32,
    /// Pre-computed normalized surface direction for 3D lighting (imaginary component).
    /// This is the normalized z/ρ direction, always in [-1, 1].
    #[serde(default)]
    pub surface_normal_im: f32,
}

impl MandelbrotData {
    /// Create a new MandelbrotData, sanitizing any NaN/Infinity float values.
    /// This is critical because serde_json serializes NaN/Infinity as null,
    /// which causes deserialization to fail with "invalid type: null, expected f32".
    pub fn new(
        iterations: u32,
        max_iterations: u32,
        escaped: bool,
        glitched: bool,
        final_z_norm_sq: f32,
        surface_normal_re: f32,
        surface_normal_im: f32,
    ) -> Self {
        Self {
            iterations,
            max_iterations,
            escaped,
            glitched,
            final_z_norm_sq: Self::sanitize_f32(final_z_norm_sq, 0.0),
            surface_normal_re: Self::sanitize_f32(surface_normal_re, 0.0),
            surface_normal_im: Self::sanitize_f32(surface_normal_im, 0.0),
        }
    }

    /// Replace NaN or Infinity with a default value to ensure JSON serialization works.
    #[inline]
    fn sanitize_f32(value: f32, default: f32) -> f32 {
        if value.is_finite() {
            value
        } else {
            default
        }
    }
}

impl Default for MandelbrotData {
    fn default() -> Self {
        Self {
            iterations: 0,
            max_iterations: 0,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 0.0,
            surface_normal_re: 0.0,
            surface_normal_im: 0.0,
        }
    }
}

/// Unified enum for all compute results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ComputeData {
    Mandelbrot(MandelbrotData),
}
