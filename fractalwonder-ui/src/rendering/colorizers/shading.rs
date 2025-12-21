//! Slope shading for 3D lighting effect on iteration height field.

use super::ShadingSettings;
use fractalwonder_core::ComputeData;

/// Check if a compute data point is interior (didn't escape).
fn is_interior(data: &ComputeData) -> bool {
    match data {
        ComputeData::Mandelbrot(m) => !m.escaped,
        ComputeData::TestImage(_) => false,
    }
}

/// Apply slope shading to a pixel buffer in place.
///
/// # Arguments
/// * `pixels` - RGBA pixel buffer to modify
/// * `data` - Original compute data (to check for interior points)
/// * `smooth_iters` - Precomputed smooth iteration values
/// * `settings` - Shading settings
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `zoom_level` - Current zoom level for auto-scaling height factor
pub fn apply_slope_shading(
    pixels: &mut [[u8; 4]],
    data: &[ComputeData],
    smooth_iters: &[f64],
    settings: &ShadingSettings,
    width: usize,
    height: usize,
    zoom_level: f64,
) {
    if !settings.enabled || settings.blend <= 0.0 {
        return;
    }

    // Auto-scale height factor with zoom
    let effective_height = settings.height_factor * (1.0 + zoom_level.log10().max(0.0) / 10.0);

    let light_x = settings.light_angle.cos();
    let light_y = settings.light_angle.sin();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;

            // Skip interior pixels - keep them pure black
            if is_interior(&data[idx]) {
                continue;
            }

            let shade = compute_shade_8neighbor(
                smooth_iters,
                width,
                height,
                x,
                y,
                light_x,
                light_y,
                effective_height,
            );

            pixels[idx] = blend_shade(pixels[idx], shade, settings.blend);
        }
    }
}

/// Compute shade value for a single pixel using 8-neighbor gradient.
/// Uses Sobel operator for robust gradient estimation.
/// Returns value in [0, 1] range where 0.5 is neutral.
#[allow(dead_code, clippy::too_many_arguments)]
fn compute_shade_8neighbor(
    smooth_iters: &[f64],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    light_x: f64,
    light_y: f64,
    height_factor: f64,
) -> f64 {
    let get = |dx: i32, dy: i32| -> f64 {
        let nx = mirror_coord(x as i32 + dx, width);
        let ny = mirror_coord(y as i32 + dy, height);
        smooth_iters[ny * width + nx]
    };

    // Sobel operator for gradient computation
    // Gx kernel: [-1  0  1]     Gy kernel: [-1 -2 -1]
    //            [-2  0  2]                [ 0  0  0]
    //            [-1  0  1]                [ 1  2  1]
    let grad_x =
        -get(-1, -1) + get(1, -1) - 2.0 * get(-1, 0) + 2.0 * get(1, 0) - get(-1, 1) + get(1, 1);

    let grad_y =
        -get(-1, -1) - 2.0 * get(0, -1) - get(1, -1) + get(-1, 1) + 2.0 * get(0, 1) + get(1, 1);

    // Magnitude for normalization
    let grad_mag = (grad_x * grad_x + grad_y * grad_y).sqrt();
    if grad_mag < 1e-10 {
        return 0.5; // Flat region, neutral shade
    }

    // Dot product with light direction, normalized
    let slope = (grad_x * light_x + grad_y * light_y) * height_factor / grad_mag;

    // Map slope to [0, 1] using sigmoid-like function
    (slope / (1.0 + slope.abs()) + 1.0) / 2.0
}

/// Mirror a coordinate at boundaries for seamless edge handling.
#[allow(dead_code)]
fn mirror_coord(coord: i32, max: usize) -> usize {
    if coord < 0 {
        (-coord).min(max as i32 - 1) as usize
    } else if coord >= max as i32 {
        let reflected = 2 * max as i32 - coord - 2;
        reflected.max(0) as usize
    } else {
        coord as usize
    }
}

/// Blend shade value with a pixel color.
/// shade: 0.5 = neutral, <0.5 = darken, >0.5 = lighten
/// blend: 0.0 = no effect, 1.0 = full shading effect
#[allow(dead_code)]
fn blend_shade(base: [u8; 4], shade: f64, blend: f64) -> [u8; 4] {
    if blend <= 0.0 {
        return base;
    }

    // shade 0 = factor 0.85 (subtle shadow), shade 0.5 = factor 1.0, shade 1 = factor 1.15 (subtle highlight)
    let factor = 0.85 + shade * 0.3;

    let apply = |c: u8| -> u8 {
        let shaded = (c as f64 * factor).clamp(0.0, 255.0);
        let blended = c as f64 + blend * (shaded - c as f64);
        blended.clamp(0.0, 255.0) as u8
    };

    [apply(base[0]), apply(base[1]), apply(base[2]), base[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_coord_in_bounds() {
        assert_eq!(mirror_coord(5, 10), 5);
        assert_eq!(mirror_coord(0, 10), 0);
        assert_eq!(mirror_coord(9, 10), 9);
    }

    #[test]
    fn mirror_coord_negative() {
        assert_eq!(mirror_coord(-1, 10), 1);
        assert_eq!(mirror_coord(-2, 10), 2);
    }

    #[test]
    fn mirror_coord_beyond_max() {
        assert_eq!(mirror_coord(10, 10), 8);
        assert_eq!(mirror_coord(11, 10), 7);
    }

    #[test]
    fn shade_flat_region_is_neutral() {
        // All same values = no slope = neutral shade (0.5)
        let iters = vec![10.0; 9];
        let shade = compute_shade_8neighbor(&iters, 3, 3, 1, 1, 1.0, 1.0, 1.0);
        assert!((shade - 0.5).abs() < 0.01, "shade = {}", shade);
    }

    #[test]
    fn shade_slope_facing_light_is_bright() {
        // Higher values to the right and top = slope facing top-right light
        #[rustfmt::skip]
        let iters = vec![
            1.0, 2.0, 3.0,
            2.0, 3.0, 4.0,
            3.0, 4.0, 5.0,
        ];
        let shade = compute_shade_8neighbor(&iters, 3, 3, 1, 1, 1.0, 1.0, 1.0);
        assert!(
            shade > 0.5,
            "shade facing light should be > 0.5, got {}",
            shade
        );
    }

    #[test]
    fn shade_slope_away_from_light_is_dark() {
        // Higher values to the left and bottom = slope away from top-right light
        #[rustfmt::skip]
        let iters = vec![
            5.0, 4.0, 3.0,
            4.0, 3.0, 2.0,
            3.0, 2.0, 1.0,
        ];
        let shade = compute_shade_8neighbor(&iters, 3, 3, 1, 1, 1.0, 1.0, 1.0);
        assert!(
            shade < 0.5,
            "shade away from light should be < 0.5, got {}",
            shade
        );
    }

    #[test]
    fn blend_neutral_unchanged() {
        let base = [128, 128, 128, 255];
        let result = blend_shade(base, 0.5, 1.0);
        // Factor = 0.85 + 0.5 * 0.3 = 1.0, so unchanged
        assert_eq!(result, base);
    }

    #[test]
    fn blend_dark_darkens() {
        let base = [128, 128, 128, 255];
        let result = blend_shade(base, 0.0, 1.0);
        // Factor = 0.3, so darkened
        assert!(result[0] < base[0], "expected darker, got {:?}", result);
    }

    #[test]
    fn blend_bright_brightens() {
        let base = [128, 128, 128, 255];
        let result = blend_shade(base, 1.0, 1.0);
        // Factor = 1.7, so brightened
        assert!(result[0] > base[0], "expected brighter, got {:?}", result);
    }

    #[test]
    fn blend_zero_unchanged() {
        let base = [128, 128, 128, 255];
        let result = blend_shade(base, 0.0, 0.0);
        assert_eq!(result, base);
    }

    use fractalwonder_core::MandelbrotData;

    fn make_exterior_data(iterations: u32) -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations: 100,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
        })
    }

    fn make_interior_data() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 100,
            max_iterations: 100,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 4.0,
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
        })
    }

    #[test]
    fn apply_shading_disabled_no_change() {
        let mut pixels = vec![[128, 128, 128, 255]; 9];
        let original = pixels.clone();
        let data: Vec<_> = (0..9).map(|i| make_exterior_data(i as u32 * 10)).collect();
        let smooth: Vec<_> = (0..9).map(|i| i as f64 * 10.0).collect();
        let settings = ShadingSettings::disabled();

        apply_slope_shading(&mut pixels, &data, &smooth, &settings, 3, 3, 1.0);

        assert_eq!(pixels, original);
    }

    #[test]
    fn apply_shading_interior_unchanged() {
        let mut pixels = vec![[0, 0, 0, 255]; 9];
        let original = pixels.clone();
        let data = vec![make_interior_data(); 9];
        let smooth = vec![100.0; 9];
        let settings = ShadingSettings::enabled();

        apply_slope_shading(&mut pixels, &data, &smooth, &settings, 3, 3, 1.0);

        assert_eq!(pixels, original);
    }

    #[test]
    fn apply_shading_modifies_exterior() {
        let mut pixels = vec![[128, 128, 128, 255]; 9];
        let original = pixels.clone();
        // Create gradient in iterations
        let data: Vec<_> = (0..9).map(|i| make_exterior_data(i as u32 * 10)).collect();
        let smooth: Vec<_> = (0..9).map(|i| i as f64 * 10.0).collect();
        let settings = ShadingSettings::enabled();

        apply_slope_shading(&mut pixels, &data, &smooth, &settings, 3, 3, 1.0);

        // At least some pixels should be modified (the center has neighbors)
        assert_ne!(pixels, original, "shading should modify some pixels");
    }
}
