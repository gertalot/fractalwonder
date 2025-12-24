//! Derivative-based 3D lighting using Blinn-Phong shading model.

use super::Palette;
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Check if a compute data point is interior (didn't escape).
fn is_interior(data: &ComputeData) -> bool {
    match data {
        ComputeData::Mandelbrot(m) => !m.escaped,
        ComputeData::TestImage(_) => false,
    }
}

/// Compute light direction vector from azimuth and elevation angles.
fn light_direction(azimuth: f64, elevation: f64) -> (f64, f64, f64) {
    let cos_elev = elevation.cos();
    (
        azimuth.cos() * cos_elev,
        azimuth.sin() * cos_elev,
        elevation.sin(),
    )
}

/// Compute surface normal from z and derivative at escape.
/// Returns (nx, ny, nz) normalized vector.
fn compute_normal(m: &MandelbrotData) -> Option<(f64, f64, f64)> {
    let z_re = m.final_z_re as f64;
    let z_im = m.final_z_im as f64;
    let rho_re = m.final_derivative_re as f64;
    let rho_im = m.final_derivative_im as f64;

    // u = z / ρ (complex division)
    let rho_norm_sq = rho_re * rho_re + rho_im * rho_im;
    if rho_norm_sq < 1e-20 {
        return None; // Degenerate case
    }

    // z / ρ = (z_re + i*z_im) / (rho_re + i*rho_im)
    //       = (z_re*rho_re + z_im*rho_im + i*(z_im*rho_re - z_re*rho_im)) / |ρ|²
    let u_re = (z_re * rho_re + z_im * rho_im) / rho_norm_sq;
    let u_im = (z_im * rho_re - z_re * rho_im) / rho_norm_sq;

    // Normalize u to unit vector in 2D
    let u_norm = (u_re * u_re + u_im * u_im).sqrt();
    if u_norm < 1e-20 {
        return None;
    }
    let u_re = u_re / u_norm;
    let u_im = u_im / u_norm;

    // 3D normal: (u_re, u_im, 1) normalized
    let n_len = (u_re * u_re + u_im * u_im + 1.0).sqrt();
    Some((u_re / n_len, u_im / n_len, 1.0 / n_len))
}

/// Apply Blinn-Phong shading to compute final shade value.
fn blinn_phong(normal: (f64, f64, f64), light: (f64, f64, f64), palette: &Palette) -> f64 {
    let (nx, ny, nz) = normal;
    let (lx, ly, lz) = light;

    let n_dot_l = (nx * lx + ny * ly + nz * lz).max(0.0);

    let vz = 1.0;

    let hx = lx;
    let hy = ly;
    let hz = lz + vz;
    let h_len = (hx * hx + hy * hy + hz * hz).sqrt();
    let (hx, hy, hz) = (hx / h_len, hy / h_len, hz / h_len);

    let n_dot_h = (nx * hx + ny * hy + nz * hz).max(0.0);
    let specular = n_dot_h.powf(palette.lighting.shininess);

    palette.lighting.ambient
        + palette.lighting.diffuse * n_dot_l
        + palette.lighting.specular * specular
}

/// Apply derivative-based Blinn-Phong shading to a pixel buffer.
pub fn apply_slope_shading(
    pixels: &mut [[u8; 4]],
    data: &[ComputeData],
    palette: &Palette,
    width: usize,
    height: usize,
) {
    if !palette.shading_enabled {
        return;
    }

    let light = light_direction(palette.lighting.azimuth, palette.lighting.elevation);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;

            if is_interior(&data[idx]) {
                continue;
            }

            let m = match &data[idx] {
                ComputeData::Mandelbrot(m) => m,
                _ => continue,
            };

            let normal = match compute_normal(m) {
                Some(n) => n,
                None => continue,
            };

            let raw_shade = blinn_phong(normal, light, palette);

            // Distance factor using falloff curve
            // Logarithmic normalization spreads iteration values evenly across the range,
            // since iteration counts grow logarithmically as you approach the boundary
            let log_normalized_iter = if m.max_iterations > 1 && m.iterations > 0 {
                (m.iterations as f64 + 1.0).ln() / (m.max_iterations as f64 + 1.0).ln()
            } else {
                0.0
            };
            // x=0 is near set boundary, x=1 is far from set
            let distance_from_set = 1.0 - log_normalized_iter;
            let distance_factor = palette.apply_falloff(distance_from_set);

            let shade = 1.0 + (raw_shade - 1.0) * palette.lighting.strength * distance_factor;

            pixels[idx] = apply_shade(pixels[idx], shade);
        }
    }
}

/// Apply shade value to a pixel.
/// shade: 1.0 = full brightness, 0.0 = black
fn apply_shade(base: [u8; 4], shade: f64) -> [u8; 4] {
    let shade = shade.clamp(0.0, 2.0); // Allow some overbright for specular
    let apply = |c: u8| -> u8 { (c as f64 * shade).clamp(0.0, 255.0) as u8 };
    [apply(base[0]), apply(base[1]), apply(base[2]), base[3]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::colorizers::{LightingParams, Palette};

    fn test_palette() -> Palette {
        Palette {
            shading_enabled: true,
            lighting: LightingParams {
                ambient: 0.15,
                diffuse: 0.7,
                specular: 0.3,
                shininess: 32.0,
                strength: 1.0,
                azimuth: 0.0,
                elevation: std::f64::consts::FRAC_PI_4,
            },
            ..Palette::default()
        }
    }

    #[test]
    fn light_direction_horizontal() {
        let (x, y, z) = light_direction(0.0, 0.0);
        assert!((x - 1.0).abs() < 0.01);
        assert!(y.abs() < 0.01);
        assert!(z.abs() < 0.01);
    }

    #[test]
    fn light_direction_overhead() {
        let (x, y, z) = light_direction(0.0, std::f64::consts::FRAC_PI_2);
        assert!(x.abs() < 0.01);
        assert!(y.abs() < 0.01);
        assert!((z - 1.0).abs() < 0.01);
    }

    #[test]
    fn compute_normal_valid() {
        let m = MandelbrotData {
            iterations: 10,
            max_iterations: 100,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
            final_z_re: 100.0,
            final_z_im: 50.0,
            final_derivative_re: 10.0,
            final_derivative_im: 5.0,
        };
        let normal = compute_normal(&m);
        assert!(normal.is_some());
        let (nx, ny, nz) = normal.unwrap();
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!((len - 1.0).abs() < 0.01);
    }

    #[test]
    fn blinn_phong_facing_light() {
        let normal = (0.0, 0.0, 1.0);
        let light = (0.0, 0.0, 1.0);
        let palette = test_palette();
        let shade = blinn_phong(normal, light, &palette);
        assert!(shade > 0.8, "shade = {}", shade);
    }

    #[test]
    fn blinn_phong_away_from_light() {
        let normal = (0.0, 0.0, 1.0);
        let light = (0.0, 0.0, -1.0);
        let palette = test_palette();
        let shade = blinn_phong(normal, light, &palette);
        assert!(shade < 0.3, "shade = {}", shade);
    }
}
