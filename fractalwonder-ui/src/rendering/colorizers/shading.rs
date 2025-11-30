//! Slope shading for 3D lighting effect on iteration height field.

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
}
