//! Extended arithmetic tests for HDRFloat: sqrt, div (HDRFloat / HDRFloat)
//!
//! These operations are used for BLA (Bivariate Linear Approximation) calculations
//! where we need to compute validity radii without f64 overflow.

use fractalwonder_core::{BigFloat, HDRComplex, HDRFloat};

// ============================================================================
// HDRFloat::is_negative(), min(), max() tests
// ============================================================================

#[test]
fn is_negative_positive_value() {
    let h = HDRFloat::from_f64(5.0);
    assert!(!h.is_negative(), "5.0 should not be negative");
}

#[test]
fn is_negative_negative_value() {
    let h = HDRFloat::from_f64(-5.0);
    assert!(h.is_negative(), "-5.0 should be negative");
}

#[test]
fn is_negative_zero() {
    let h = HDRFloat::ZERO;
    assert!(!h.is_negative(), "zero should not be negative");
}

#[test]
fn min_returns_smaller() {
    let a = HDRFloat::from_f64(3.0);
    let b = HDRFloat::from_f64(5.0);
    let result = a.min(&b);
    assert!(
        (result.to_f64() - 3.0).abs() < 1e-14,
        "min(3, 5) = {}, expected 3",
        result.to_f64()
    );

    // Also test reverse order
    let result2 = b.min(&a);
    assert!(
        (result2.to_f64() - 3.0).abs() < 1e-14,
        "min(5, 3) = {}, expected 3",
        result2.to_f64()
    );
}

#[test]
fn max_returns_larger() {
    let a = HDRFloat::from_f64(3.0);
    let b = HDRFloat::from_f64(5.0);
    let result = a.max(&b);
    assert!(
        (result.to_f64() - 5.0).abs() < 1e-14,
        "max(3, 5) = {}, expected 5",
        result.to_f64()
    );
}

#[test]
fn min_max_with_negatives() {
    let a = HDRFloat::from_f64(-3.0);
    let b = HDRFloat::from_f64(5.0);

    let min_result = a.min(&b);
    assert!(
        (min_result.to_f64() - (-3.0)).abs() < 1e-14,
        "min(-3, 5) = {}, expected -3",
        min_result.to_f64()
    );

    let max_result = a.max(&b);
    assert!(
        (max_result.to_f64() - 5.0).abs() < 1e-14,
        "max(-3, 5) = {}, expected 5",
        max_result.to_f64()
    );
}

#[test]
fn min_max_extreme_exponents() {
    // Compare 1e100 and 1e-100
    let bf_big = BigFloat::from_string("1e100", 256).unwrap();
    let bf_small = BigFloat::from_string("1e-100", 256).unwrap();
    let big = HDRFloat::from_bigfloat(&bf_big);
    let small = HDRFloat::from_bigfloat(&bf_small);

    let min_result = big.min(&small);
    assert!(
        min_result.exp < 0,
        "min(1e100, 1e-100) should have negative exponent, got {}",
        min_result.exp
    );

    let max_result = big.max(&small);
    assert!(
        max_result.exp > 300,
        "max(1e100, 1e-100) should have large exponent, got {}",
        max_result.exp
    );
}

#[test]
fn max_with_zero() {
    let a = HDRFloat::from_f64(-5.0);
    let zero = HDRFloat::ZERO;

    // max(-5, 0) = 0
    let result = a.max(&zero);
    assert!(
        result.is_zero() || result.to_f64() >= 0.0,
        "max(-5, 0) should be 0, got {}",
        result.to_f64()
    );
}

// ============================================================================
// HDRComplex::norm_sq_hdr() and norm_hdr() tests
// ============================================================================

#[test]
fn complex_norm_sq_hdr_basic() {
    // |3 + 4i|² = 9 + 16 = 25
    let c = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let result = c.norm_sq_hdr();
    assert!(
        (result.to_f64() - 25.0).abs() < 1e-14,
        "|3+4i|² = {}, expected 25",
        result.to_f64()
    );
}

#[test]
fn complex_norm_hdr_basic() {
    // |3 + 4i| = 5
    let c = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let result = c.norm_hdr();
    assert!(
        (result.to_f64() - 5.0).abs() < 1e-14,
        "|3+4i| = {}, expected 5",
        result.to_f64()
    );
}

#[test]
fn complex_norm_sq_hdr_extreme_values() {
    // Test with values beyond f64 range
    // |1e200 + 1e200i|² = 2e400 - would overflow f64 but not HDRFloat
    let bf = BigFloat::from_string("1e200", 256).unwrap();
    let val = HDRFloat::from_bigfloat(&bf);
    let c = HDRComplex { re: val, im: val };

    let result = c.norm_sq_hdr();

    // Exponent should be approximately 400 * log2(10) ≈ 1329
    assert!(
        result.exp > 1300 && result.exp < 1400,
        "|1e200 + 1e200i|² exponent = {}, expected ~1329",
        result.exp
    );
    assert!(!result.is_zero(), "Should not underflow");
}

#[test]
fn complex_norm_hdr_extreme_values() {
    // |1e200 + 1e200i| = sqrt(2) * 1e200
    let bf = BigFloat::from_string("1e200", 256).unwrap();
    let val = HDRFloat::from_bigfloat(&bf);
    let c = HDRComplex { re: val, im: val };

    let result = c.norm_hdr();

    // Exponent should be approximately 200 * log2(10) ≈ 664
    assert!(
        result.exp > 650 && result.exp < 700,
        "|1e200 + 1e200i| exponent = {}, expected ~664",
        result.exp
    );
}

#[test]
fn complex_norm_sq_hdr_zero() {
    let c = HDRComplex::ZERO;
    let result = c.norm_sq_hdr();
    assert!(result.is_zero(), "|0|² should be 0");
}

#[test]
fn complex_norm_hdr_zero() {
    let c = HDRComplex::ZERO;
    let result = c.norm_hdr();
    assert!(result.is_zero(), "|0| should be 0");
}

// ============================================================================
// HDRFloat::sqrt() tests
// ============================================================================

#[test]
fn sqrt_zero() {
    let h = HDRFloat::ZERO;
    let result = h.sqrt();
    assert!(result.is_zero(), "sqrt(0) should be 0");
}

#[test]
fn sqrt_one() {
    let h = HDRFloat::from_f64(1.0);
    let result = h.sqrt();
    assert!(
        (result.to_f64() - 1.0).abs() < 1e-14,
        "sqrt(1) = {}, expected 1.0",
        result.to_f64()
    );
}

#[test]
fn sqrt_four() {
    let h = HDRFloat::from_f64(4.0);
    let result = h.sqrt();
    assert!(
        (result.to_f64() - 2.0).abs() < 1e-14,
        "sqrt(4) = {}, expected 2.0",
        result.to_f64()
    );
}

#[test]
fn sqrt_preserves_precision() {
    // Test various values within f64 range
    let values = [0.25, 0.5, 2.0, 9.0, 100.0, 1e10, 1e-10];
    for v in values {
        let h = HDRFloat::from_f64(v);
        let result = h.sqrt();
        let expected = v.sqrt();
        let rel_error = (result.to_f64() - expected).abs() / expected;
        assert!(
            rel_error < 1e-10,
            "sqrt({}) = {}, expected {}, rel_error = {}",
            v,
            result.to_f64(),
            expected,
            rel_error
        );
    }
}

#[test]
fn sqrt_extreme_large_value() {
    // sqrt(1e200) = 1e100 - within HDRFloat but tests large exponent handling
    let bf = BigFloat::from_string("1e200", 256).unwrap();
    let h = HDRFloat::from_bigfloat(&bf);

    let result = h.sqrt();

    // Exponent should be halved: ~200*log2(10)/2 ≈ 332
    assert!(
        result.exp > 300 && result.exp < 400,
        "sqrt(1e200) exponent = {}, expected ~332",
        result.exp
    );

    // Value should be approximately 1e100
    // Can't convert to f64 directly, but mantissa should be ~0.5-1.0
    assert!(
        result.head.abs() >= 0.5 && result.head.abs() < 1.0,
        "sqrt result should be normalized, head = {}",
        result.head
    );
}

#[test]
fn sqrt_extreme_small_value() {
    // sqrt(1e-200) = 1e-100 - tests negative exponent halving
    let bf = BigFloat::from_string("1e-200", 256).unwrap();
    let h = HDRFloat::from_bigfloat(&bf);

    let result = h.sqrt();

    // Exponent should be halved: ~-200*log2(10)/2 ≈ -332
    assert!(
        result.exp < -300 && result.exp > -400,
        "sqrt(1e-200) exponent = {}, expected ~-332",
        result.exp
    );

    assert!(
        result.head.abs() >= 0.5 && result.head.abs() < 1.0,
        "sqrt result should be normalized, head = {}",
        result.head
    );
}

#[test]
fn sqrt_odd_exponent() {
    // When exponent is odd, sqrt needs special handling
    // sqrt(2) = sqrt(0.5 * 2^2) = sqrt(0.5) * 2^1
    let h = HDRFloat::from_f64(2.0);
    let result = h.sqrt();
    let expected = 2.0_f64.sqrt();
    assert!(
        (result.to_f64() - expected).abs() < 1e-10,
        "sqrt(2) = {}, expected {}",
        result.to_f64(),
        expected
    );
}

#[test]
fn sqrt_negative_returns_zero() {
    // sqrt of negative should return zero (or could panic, but zero is safer)
    let h = HDRFloat::from_f64(-4.0);
    let result = h.sqrt();
    assert!(
        result.is_zero(),
        "sqrt(-4) should return zero, got {:?}",
        result
    );
}

// ============================================================================
// HDRFloat::div() tests (dividing two HDRFloat values)
// ============================================================================

#[test]
fn div_basic() {
    let a = HDRFloat::from_f64(6.0);
    let b = HDRFloat::from_f64(2.0);
    let result = a.div(&b);
    assert!(
        (result.to_f64() - 3.0).abs() < 1e-14,
        "6 / 2 = {}, expected 3.0",
        result.to_f64()
    );
}

#[test]
fn div_by_one() {
    let a = HDRFloat::from_f64(42.0);
    let one = HDRFloat::from_f64(1.0);
    let result = a.div(&one);
    assert!(
        (result.to_f64() - 42.0).abs() < 1e-14,
        "42 / 1 = {}, expected 42.0",
        result.to_f64()
    );
}

#[test]
fn div_zero_dividend() {
    let zero = HDRFloat::ZERO;
    let b = HDRFloat::from_f64(5.0);
    let result = zero.div(&b);
    assert!(result.is_zero(), "0 / 5 should be 0");
}

#[test]
fn div_by_zero_returns_infinity() {
    let a = HDRFloat::from_f64(5.0);
    let zero = HDRFloat::ZERO;
    let result = a.div(&zero);
    // Should return infinity (head = inf, or very large exponent)
    assert!(
        result.head.is_infinite() || result.exp > 1000,
        "5 / 0 should be infinity, got {:?}",
        result
    );
}

#[test]
fn div_preserves_precision() {
    let values = [(10.0, 3.0), (1.0, 7.0), (22.0, 7.0), (1e10, 3.0)];
    for (a, b) in values {
        let ha = HDRFloat::from_f64(a);
        let hb = HDRFloat::from_f64(b);
        let result = ha.div(&hb);
        let expected = a / b;
        let rel_error = (result.to_f64() - expected).abs() / expected.abs();
        assert!(
            rel_error < 1e-10,
            "{} / {} = {}, expected {}, rel_error = {}",
            a,
            b,
            result.to_f64(),
            expected,
            rel_error
        );
    }
}

#[test]
fn div_extreme_exponents() {
    // 1e200 / 1e100 = 1e100 - tests exponent subtraction
    let bf_a = BigFloat::from_string("1e200", 256).unwrap();
    let bf_b = BigFloat::from_string("1e100", 256).unwrap();
    let a = HDRFloat::from_bigfloat(&bf_a);
    let b = HDRFloat::from_bigfloat(&bf_b);

    let result = a.div(&b);

    // Result exponent should be approximately 100 * log2(10) ≈ 332
    assert!(
        result.exp > 300 && result.exp < 400,
        "1e200 / 1e100 exponent = {}, expected ~332",
        result.exp
    );
}

#[test]
fn div_small_by_large() {
    // 1e-100 / 1e100 = 1e-200
    let bf_a = BigFloat::from_string("1e-100", 256).unwrap();
    let bf_b = BigFloat::from_string("1e100", 256).unwrap();
    let a = HDRFloat::from_bigfloat(&bf_a);
    let b = HDRFloat::from_bigfloat(&bf_b);

    let result = a.div(&b);

    // Result exponent should be approximately -200 * log2(10) ≈ -664
    assert!(
        result.exp < -600 && result.exp > -700,
        "1e-100 / 1e100 exponent = {}, expected ~-664",
        result.exp
    );
    assert!(!result.is_zero(), "Should not underflow to zero");
}
