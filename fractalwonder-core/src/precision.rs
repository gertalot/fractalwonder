//! Precision calculation for fractal rendering.
//!
//! Determines how many mantissa bits are needed to accurately compute
//! fractal values at a given viewport and resolution.
//!
//! # Limitations
//!
//! This implementation uses f64 for intermediate log2 calculations. This is acceptable
//! for zooms with decimal exponents up to ~10^6 (which correspond to base-2 logs ~3.3×10^6,
//! representable in f64). For extreme zooms beyond this, consider computing log2 using
//! high-precision arithmetic or extracting mantissa+exponent pairs.

use crate::Viewport;

/// Base safety margin for rounding errors in arithmetic operations.
/// Additional safety is added based on zoom depth.
const BASE_SAFETY_BITS: u64 = 16;

/// Maximum allowed precision bits (1M bits cap to prevent overflow).
const MAX_ALLOWED_BITS: usize = 1 << 20;

/// Default maximum iterations for Mandelbrot computation.
const DEFAULT_MAX_ITERATIONS: u64 = 10_000;

/// Compute ceil(log2(n)) for integer n >= 1 using bit operations.
/// Returns 0 for n <= 1.
fn ceil_log2_u64(n: u64) -> u64 {
    if n <= 1 {
        return 0;
    }
    // Number of bits needed to represent (n-1), which equals ceil(log2(n))
    64 - (n - 1).leading_zeros() as u64
}

/// Compute log2(x + y) from log2(x) and log2(y) using the identity:
/// log2(x + y) = max(log2(x), log2(y)) + log2(1 + 2^{-d})
/// where d = |log2(x) - log2(y)|.
///
/// Handles -inf (representing zero) correctly.
fn log2_sum_from_logs(log2_a: f64, log2_b: f64) -> f64 {
    // Handle -inf (zero values)
    if log2_a.is_infinite() && log2_a.is_sign_negative() {
        return log2_b;
    }
    if log2_b.is_infinite() && log2_b.is_sign_negative() {
        return log2_a;
    }

    let (mx, mn) = if log2_a >= log2_b {
        (log2_a, log2_b)
    } else {
        (log2_b, log2_a)
    };

    let d = mx - mn;

    // If one term is > 2^60 times the other, the smaller term is negligible
    if d > 60.0 {
        return mx;
    }

    // log2(1 + 2^{-d}) correction term
    let correction = (1.0f64 + 2f64.powf(-d)).log2();
    mx + correction
}

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
/// Required precision bits (not rounded to power of 2), minimum 64 bits,
/// clamped to MAX_ALLOWED_BITS.
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
    // Compute log2(M) accurately using log2_sum_from_logs
    let log2_half_width = width.log2_approx() - 1.0;
    let log2_half_height = height.log2_approx() - 1.0;
    let log2_cx = cx.abs().log2_approx();
    let log2_cy = cy.abs().log2_approx();

    // Accurate log2(|cx| + width/2) and log2(|cy| + height/2)
    let log2_mx = log2_sum_from_logs(log2_cx, log2_half_width);
    let log2_my = log2_sum_from_logs(log2_cy, log2_half_height);
    let log2_m = log2_mx.max(log2_my);

    // bits_from_ratio = ceil(log2(M / min_delta))
    let log2_ratio = log2_m - log2_min_delta;
    let bits_from_ratio = if log2_ratio.is_finite() && log2_ratio > 0.0 {
        log2_ratio.ceil() as u64
    } else {
        0
    };

    // Zoom depth floor: ensure enough precision to handle panning to any location
    // at this zoom level. If viewport width is tiny (e.g., 10^-1000), and user pans
    // to center (1.0, 0), coordinates become 1 + O(10^-1000), requiring ~3320 bits.
    // zoom_depth_bits = bits to represent pixel_delta relative to unit scale (1.0)
    let zoom_depth_bits = if log2_min_delta.is_finite() && log2_min_delta < 0.0 {
        (-log2_min_delta).ceil() as u64
    } else {
        0
    };
    let bits_from_ratio = bits_from_ratio.max(zoom_depth_bits);

    // Integer-safe iteration bits using bit operations
    let iter_bits = ceil_log2_u64(max_iterations);

    // Scale safety margin based on zoom depth (bits_from_ratio indicates zoom)
    // At shallow zooms (< 64 bits), use base safety
    // At deeper zooms, add ~10% extra for accumulated rounding errors
    let zoom_safety = if bits_from_ratio > 64 {
        bits_from_ratio / 10
    } else {
        0
    };
    let safety_bits = BASE_SAFETY_BITS.saturating_add(zoom_safety);

    // Combine with safety margin using saturating arithmetic
    let total_bits = bits_from_ratio
        .saturating_add(iter_bits)
        .saturating_add(safety_bits);

    // Clamp to MAX_ALLOWED_BITS and ensure minimum of 64 bits
    let total_usize = if (total_bits as usize) > MAX_ALLOWED_BITS {
        MAX_ALLOWED_BITS
    } else {
        total_bits as usize
    };

    let result = total_usize.max(64);

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(
        &format!(
            "[Precision] bits_from_ratio={}, iter_bits={}, safety_bits={}, total={}, result={}",
            bits_from_ratio, iter_bits, safety_bits, total_bits, result
        )
        .into(),
    );

    result
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
    fn precision_minimum_is_64() {
        let viewport = Viewport::from_f64(0.0, 0.0, 1000.0, 1000.0, 64);
        let bits = calculate_precision_bits(&viewport, (100, 100));
        assert!(bits >= 64);
    }

    // --- Tests for fixed precision calculation ---

    #[test]
    fn precision_not_rounded_to_power_of_two() {
        // 6730 bits should NOT become 8192
        // At a zoom where we need ~6730 bits, the result should be close to that
        // not rounded up to 8192 (which wastes ~22% precision)
        let viewport = Viewport::from_strings("-0.5", "0.0", "1e-2000", "1e-2000", 7000).unwrap();
        let bits = calculate_precision_bits(&viewport, (1920, 1080));

        // Should be less than what power-of-two rounding would produce
        // 2000 * log2(10) ≈ 6644, plus safety bits, but NOT rounded to 8192
        assert!(
            !bits.is_power_of_two() || bits <= 4096,
            "Got {} which suggests power-of-two rounding is still active",
            bits
        );
    }

    #[test]
    fn precision_handles_zero_center() {
        // Center at origin - should not panic or produce infinite bits
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let bits = calculate_precision_bits(&viewport, (1920, 1080));

        assert!(bits >= 64);
        assert!(bits < 1000); // Reasonable bound for 1x zoom
    }

    #[test]
    fn precision_accurate_sum_not_crude_max_plus_one() {
        // When |cx| and width/2 are very different magnitudes, the crude max+1
        // formula wastes bits. Test that we get tighter estimates.
        //
        // Example: cx = 0.001, width = 4.0
        // log2(|cx| + width/2) = log2(0.001 + 2.0) ≈ log2(2.001) ≈ 1.0
        // Crude formula: max(log2(0.001), log2(2)) + 1 = max(-10, 1) + 1 = 2
        // Accurate: ~1.0
        //
        // The difference compounds over the calculation.
        let viewport_small_center = Viewport::from_f64(0.001, 0.001, 4.0, 4.0, 128);
        let viewport_large_center = Viewport::from_f64(1.5, 1.5, 4.0, 4.0, 128);

        let bits_small = calculate_precision_bits(&viewport_small_center, (1920, 1080));
        let bits_large = calculate_precision_bits(&viewport_large_center, (1920, 1080));

        // Both should be similar since width dominates, but with accurate
        // log2(x+y) calculation, small center shouldn't add unnecessary bits
        let diff = (bits_small as i64 - bits_large as i64).unsigned_abs();
        assert!(
            diff <= 10,
            "Expected similar precision for width-dominated cases, got {} vs {} (diff={})",
            bits_small,
            bits_large,
            diff
        );
    }

    #[test]
    fn precision_uses_integer_iter_bits_for_large_iterations() {
        // For iterations > 2^53, f64 loses precision
        // Test with a value that would round incorrectly in f64
        let viewport = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);

        // 2^60 iterations - ceil(log2(2^60)) should be exactly 60
        let bits_2_60 =
            calculate_precision_bits_with_iterations(&viewport, (1920, 1080), 1u64 << 60);

        // 2^60 + 1 iterations - ceil(log2(2^60 + 1)) should be 61
        let bits_2_60_plus_1 =
            calculate_precision_bits_with_iterations(&viewport, (1920, 1080), (1u64 << 60) + 1);

        // The +1 iteration should add exactly 1 bit (not be lost to f64 rounding)
        assert_eq!(
            bits_2_60_plus_1 - bits_2_60,
            1,
            "Expected exactly 1 bit difference for 2^60 vs 2^60+1 iterations"
        );
    }

    #[test]
    fn precision_clamped_to_max_allowed() {
        // Test that results are clamped to MAX_ALLOWED_BITS
        // Use a deep zoom that produces a large but testable bit count
        let deep_viewport =
            Viewport::from_strings("-0.5", "0.0", "1e-2000", "1e-2000", 7000).unwrap();

        let bits = calculate_precision_bits(&deep_viewport, (1920, 1080));

        // Should be within MAX_ALLOWED_BITS and not overflow
        assert!(
            bits <= MAX_ALLOWED_BITS,
            "Got {} which exceeds MAX_ALLOWED_BITS",
            bits
        );
        assert!(bits >= 64, "Minimum should still be enforced");
    }

    #[test]
    fn ceil_log2_u64_correctness() {
        // Test the integer bit operation helper
        assert_eq!(ceil_log2_u64(0), 0);
        assert_eq!(ceil_log2_u64(1), 0);
        assert_eq!(ceil_log2_u64(2), 1);
        assert_eq!(ceil_log2_u64(3), 2);
        assert_eq!(ceil_log2_u64(4), 2);
        assert_eq!(ceil_log2_u64(5), 3);
        assert_eq!(ceil_log2_u64(8), 3);
        assert_eq!(ceil_log2_u64(9), 4);
        assert_eq!(ceil_log2_u64(1u64 << 60), 60);
        assert_eq!(ceil_log2_u64((1u64 << 60) + 1), 61);
        assert_eq!(ceil_log2_u64(u64::MAX), 64);
    }

    #[test]
    fn log2_sum_from_logs_correctness() {
        // Test accurate log2(x+y) computation

        // Equal values: log2(x + x) = log2(2x) = log2(x) + 1
        let result = log2_sum_from_logs(3.0, 3.0);
        assert!(
            (result - 4.0).abs() < 0.001,
            "log2(8+8)=log2(16)=4, got {}",
            result
        );

        // Very different magnitudes: log2(1 + 1024) ≈ log2(1024) = 10
        let result = log2_sum_from_logs(0.0, 10.0);
        let expected = (1.0f64 + 1024.0).log2(); // ~10.001
        assert!(
            (result - expected).abs() < 0.001,
            "Expected ~{}, got {}",
            expected,
            result
        );

        // Zero handling: log2(0 + x) = log2(x)
        let result = log2_sum_from_logs(f64::NEG_INFINITY, 5.0);
        assert_eq!(result, 5.0);

        let result = log2_sum_from_logs(5.0, f64::NEG_INFINITY);
        assert_eq!(result, 5.0);

        // Both zero
        let result = log2_sum_from_logs(f64::NEG_INFINITY, f64::NEG_INFINITY);
        assert!(result.is_infinite() && result.is_sign_negative());
    }
}
