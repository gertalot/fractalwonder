//! Perturbation theory computation for deep Mandelbrot zoom.
//!
//! Computes reference orbits at high precision, then uses fast f64
//! delta iterations for individual pixels.

mod pixel;
mod pixel_hdr_bla;
mod reference_orbit;
mod tile;

#[allow(unused_imports)] // Will be used by consumers in upcoming tasks
pub use tile::{TileRenderResult, TileStats};

pub use pixel::compute_pixel_perturbation;
pub use pixel_hdr_bla::{compute_pixel_perturbation_hdr_bla, BlaStats};
pub use reference_orbit::ReferenceOrbit;

/// Compute normalized z/ρ direction for 3D lighting.
/// Returns (re, im) of the unit vector, or (0, 0) if degenerate.
/// This works at any zoom level since we normalize to a unit vector.
#[inline]
pub(crate) fn compute_surface_normal_direction(
    z_re: f64,
    z_im: f64,
    rho_re: f64,
    rho_im: f64,
) -> (f32, f32) {
    // u = z / ρ (complex division)
    // u = z * conj(ρ) / |ρ|²
    let rho_norm_sq = rho_re * rho_re + rho_im * rho_im;
    if !rho_norm_sq.is_finite() || rho_norm_sq == 0.0 {
        return (0.0, 0.0);
    }

    let u_re = (z_re * rho_re + z_im * rho_im) / rho_norm_sq;
    let u_im = (z_im * rho_re - z_re * rho_im) / rho_norm_sq;

    // Normalize to unit vector
    let u_norm = (u_re * u_re + u_im * u_im).sqrt();
    if !u_norm.is_finite() || u_norm == 0.0 {
        return (0.0, 0.0);
    }

    ((u_re / u_norm) as f32, (u_im / u_norm) as f32)
}

#[cfg(test)]
mod tests;
