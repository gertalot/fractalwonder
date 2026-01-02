//! Perturbation theory computation for deep Mandelbrot zoom.
//!
//! Computes reference orbits at high precision, then uses fast f64
//! delta iterations for individual pixels.

mod pixel;
mod pixel_f64_bla;
mod pixel_hdr_bla;
mod reference_orbit;
mod tile;

pub use tile::{render_tile_f64, render_tile_hdr, TileConfig, TileRenderResult, TileStats};

pub use pixel::compute_pixel_perturbation;
pub use pixel_f64_bla::compute_pixel_perturbation_f64_bla;
pub use pixel_hdr_bla::{compute_pixel_perturbation_hdr_bla, BlaStats};
pub use reference_orbit::ReferenceOrbit;

use fractalwonder_core::HDRFloat;

/// Compute surface normal direction for 3D lighting using HDRFloat arithmetic.
///
/// Computes u = z × conj(ρ) and returns the normalized direction as (f32, f32).
/// Uses HDRFloat throughout to preserve precision at deep zoom where ρ can have
/// magnitude ~10^100+. Only converts to f32 at the final normalization step.
///
/// This mirrors the GPU's hdr_complex_direction() approach: scale both components
/// to a common exponent before normalizing, preserving the ratio at any magnitude.
#[inline]
pub(crate) fn compute_surface_normal_direction(
    z_re: &HDRFloat,
    z_im: &HDRFloat,
    rho_re: &HDRFloat,
    rho_im: &HDRFloat,
) -> (f32, f32) {
    // Compute u = z × conj(ρ) in HDRFloat
    // u_re = z_re × ρ_re + z_im × ρ_im
    // u_im = z_im × ρ_re - z_re × ρ_im
    let u_re = z_re.mul(rho_re).add(&z_im.mul(rho_im));
    let u_im = z_im.mul(rho_re).sub(&z_re.mul(rho_im));

    // Handle zero case
    if u_re.is_zero() && u_im.is_zero() {
        return (0.0, 0.0);
    }

    // Scale both components to common exponent to preserve ratio
    // exp - max_exp is always <= 0, so 2^(exp - max_exp) is in (0, 1]
    let max_exp = u_re.exp.max(u_im.exp);
    let re_mantissa = (u_re.head as f64) + (u_re.tail as f64);
    let im_mantissa = (u_im.head as f64) + (u_im.tail as f64);

    let re_scaled = re_mantissa * 2.0_f64.powi(u_re.exp - max_exp);
    let im_scaled = im_mantissa * 2.0_f64.powi(u_im.exp - max_exp);

    // Normalize to unit vector
    let norm = (re_scaled * re_scaled + im_scaled * im_scaled).sqrt();
    if norm == 0.0 || !norm.is_finite() {
        return (0.0, 0.0);
    }

    ((re_scaled / norm) as f32, (im_scaled / norm) as f32)
}

#[cfg(test)]
mod tests;
