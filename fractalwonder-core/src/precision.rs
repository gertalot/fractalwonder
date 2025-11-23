//! Precision calculation for fractal rendering.
//!
//! Determines how many mantissa bits are needed to accurately compute
//! fractal values at a given viewport and resolution.

use crate::Viewport;

/// Safety margin for rounding errors in arithmetic operations.
const SAFETY_BITS: u64 = 64;

/// Default maximum iterations for Mandelbrot computation.
const DEFAULT_MAX_ITERATIONS: u64 = 10_000;

/// Calculate required precision bits for fractal computation.
///
/// Determines how many mantissa bits BigFloat values need to:
/// 1. Represent coordinates at the viewport's zoom level
/// 2. Distinguish adjacent pixels in the computation
/// 3. Survive error amplification over many iterations
///
/// # Arguments
/// * `viewport` - The fractal-space region to render
/// * `canvas_size` - The pixel resolution (width, height)
///
/// # Returns
/// Required precision bits, rounded up to a power of 2 for efficiency.
pub fn calculate_precision_bits(viewport: &Viewport, canvas_size: (u32, u32)) -> usize {
    calculate_precision_bits_with_iterations(viewport, canvas_size, DEFAULT_MAX_ITERATIONS)
}

/// Calculate precision bits with custom iteration count.
pub fn calculate_precision_bits_with_iterations(
    viewport: &Viewport,
    canvas_size: (u32, u32),
    max_iterations: u64,
) -> usize {
    let (cx, cy) = &viewport.center;
    let width = &viewport.width;
    let height = &viewport.height;

    let px = canvas_size.0 as f64;
    let py = canvas_size.1 as f64;

    // log2(min_delta) where delta = dimension / pixels
    let log2_delta_x = width.log2_approx() - px.log2();
    let log2_delta_y = height.log2_approx() - py.log2();
    let log2_min_delta = log2_delta_x.min(log2_delta_y);

    // M = max(|cx| + width/2, |cy| + height/2)
    // Approximate log2(M) conservatively
    let log2_half_width = width.log2_approx() - 1.0;
    let log2_half_height = height.log2_approx() - 1.0;
    let log2_cx = cx.abs().log2_approx();
    let log2_cy = cy.abs().log2_approx();

    // For sums like |cx| + width/2, use max and add 1 bit for safety
    let log2_mx = log2_cx.max(log2_half_width) + 1.0;
    let log2_my = log2_cy.max(log2_half_height) + 1.0;
    let log2_m = log2_mx.max(log2_my);

    // bits_from_ratio = ceil(log2(M / min_delta))
    let log2_ratio = log2_m - log2_min_delta;
    let bits_from_ratio = log2_ratio.ceil().max(0.0) as u64;

    // Bits for iteration error amplification: log2(iterations)
    let iter_bits = if max_iterations > 1 {
        (max_iterations as f64).log2().ceil() as u64
    } else {
        0
    };

    let total_bits = bits_from_ratio + iter_bits + SAFETY_BITS;

    // Round to power of 2, minimum 64 bits
    (total_bits as usize).next_power_of_two().max(64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_at_1x_zoom_is_reasonable() {
        // At 1x zoom with 4K canvas, should need ~128 bits
        let viewport = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);
        let bits = calculate_precision_bits(&viewport, (3840, 2160));
        assert!(bits >= 64);
        assert!(bits <= 256);
    }

    #[test]
    fn precision_increases_with_zoom() {
        // 1x zoom: width = 4.0
        let viewport_1x = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);
        // 10^20 zoom: width = 4e-20 (beyond f64 precision but within BigFloat)
        let viewport_deep = Viewport::from_strings("-0.5", "0.0", "4e-20", "4e-20", 256).unwrap();

        let bits_1x = calculate_precision_bits(&viewport_1x, (1920, 1080));
        let bits_deep = calculate_precision_bits(&viewport_deep, (1920, 1080));

        // At 10^20 zoom, we need approximately 66 more bits (20 * 3.322)
        // This should clearly push us into a higher power-of-2 bucket
        assert!(bits_deep > bits_1x, "Expected {} > {}", bits_deep, bits_1x);
    }

    #[test]
    fn precision_at_extreme_zoom() {
        // At 10^500 zoom, width is ~10^-500
        let viewport = Viewport::from_strings("-0.5", "0.0", "1e-500", "1e-500", 7000).unwrap();

        let bits = calculate_precision_bits(&viewport, (1920, 1080));

        // Should need ~1700+ bits (500 * 3.322 + safety)
        assert!(bits >= 1024);
        assert!(bits <= 4096);
    }

    #[test]
    fn precision_is_power_of_two() {
        let viewport = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);
        let bits = calculate_precision_bits(&viewport, (1920, 1080));
        assert!(bits.is_power_of_two());
    }

    #[test]
    fn precision_minimum_is_64() {
        let viewport = Viewport::from_f64(0.0, 0.0, 1000.0, 1000.0, 64);
        let bits = calculate_precision_bits(&viewport, (100, 100));
        assert!(bits >= 64);
    }
}
