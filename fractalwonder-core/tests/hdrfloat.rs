use fractalwonder_core::{BigFloat, HDRComplex, HDRFloat};

#[test]
fn from_f32_zero_gives_zero() {
    let h = HDRFloat::from_f32(0.0);
    assert!(h.is_zero());
    assert_eq!(h.head, 0.0);
    assert_eq!(h.tail, 0.0);
    assert_eq!(h.exp, 0);
}

#[test]
fn from_f32_one_normalized() {
    let h = HDRFloat::from_f32(1.0);
    assert!(!h.is_zero());
    // 1.0 = 0.5 × 2^1, so head should be 0.5, exp should be 1
    assert!((h.head - 0.5).abs() < 1e-7);
    assert_eq!(h.tail, 0.0);
    assert_eq!(h.exp, 1);
}

#[test]
fn from_f32_preserves_value() {
    let values = [1.0f32, -1.0, 0.5, 2.0, 1e10, 1e-10, -std::f32::consts::PI];
    for v in values {
        let h = HDRFloat::from_f32(v);
        let back = h.to_f32();
        assert!(
            (back - v).abs() < v.abs() * 1e-6 + 1e-38,
            "from_f32({}) -> to_f32() = {}, expected {}",
            v,
            back,
            v
        );
    }
}

#[test]
fn normalize_handles_range_one_to_two() {
    // Values in [1.0, 2.0) should be normalized to [0.5, 1.0)
    let h = HDRFloat {
        head: 1.5,
        tail: 0.0,
        exp: 0,
    };
    let normalized = h.normalize();
    assert!((normalized.head - 0.75).abs() < 1e-7);
    assert_eq!(normalized.exp, 1);
}

#[test]
fn from_f64_captures_more_precision_than_f32() {
    // Value with more precision than f32 can represent
    let val: f64 = 1.0 + 1e-10;
    let h = HDRFloat::from_f64(val);

    // Converting back should preserve more precision than direct f32 cast
    let back = h.to_f64();
    let direct = val as f32 as f64;

    let error_hdr = (back - val).abs();
    let error_direct = (direct - val).abs();

    assert!(
        error_hdr < error_direct,
        "HDRFloat error {} should be less than direct f32 error {}",
        error_hdr,
        error_direct
    );
}

#[test]
fn from_f64_preserves_value() {
    let values = [1.0f64, -1.0, 0.5, 2.0, 1e10, 1e-10, std::f64::consts::PI];
    for v in values {
        let h = HDRFloat::from_f64(v);
        let back = h.to_f64();
        // Should preserve ~48 bits of precision
        assert!(
            (back - v).abs() < v.abs() * 1e-14 + 1e-300,
            "from_f64({}) -> to_f64() = {}, diff = {}",
            v,
            back,
            (back - v).abs()
        );
    }
}

#[test]
fn mul_basic() {
    let a = HDRFloat::from_f64(2.0);
    let b = HDRFloat::from_f64(3.0);
    let c = a.mul(&b);
    assert!((c.to_f64() - 6.0).abs() < 1e-14);
}

#[test]
fn mul_by_zero() {
    let a = HDRFloat::from_f64(5.0);
    let z = HDRFloat::ZERO;
    assert!(a.mul(&z).is_zero());
    assert!(z.mul(&a).is_zero());
}

#[test]
fn mul_small_values() {
    let a = HDRFloat::from_f64(1e-20);
    let b = HDRFloat::from_f64(1e-20);
    let c = a.mul(&b);
    // Result is 1e-40, within HDRFloat range
    assert!((c.to_f64() - 1e-40).abs() < 1e-54);
}

#[test]
fn mul_preserves_precision() {
    // Two values that require full precision
    let a = HDRFloat::from_f64(1.0 + 1e-10);
    let b = HDRFloat::from_f64(1.0 + 2e-10);
    let c = a.mul(&b);
    let expected = (1.0 + 1e-10) * (1.0 + 2e-10);
    assert!(
        (c.to_f64() - expected).abs() < expected * 1e-14,
        "mul precision: got {}, expected {}",
        c.to_f64(),
        expected
    );
}

#[test]
fn add_basic() {
    let a = HDRFloat::from_f64(2.0);
    let b = HDRFloat::from_f64(3.0);
    assert!((a.add(&b).to_f64() - 5.0).abs() < 1e-14);
}

#[test]
fn add_zero() {
    let a = HDRFloat::from_f64(5.0);
    let z = HDRFloat::ZERO;
    assert!((a.add(&z).to_f64() - 5.0).abs() < 1e-14);
    assert!((z.add(&a).to_f64() - 5.0).abs() < 1e-14);
}

#[test]
fn add_different_exponents() {
    // 1e10 + 1e-10 should be approximately 1e10
    let big = HDRFloat::from_f64(1e10);
    let small = HDRFloat::from_f64(1e-10);
    let sum = big.add(&small);
    assert!((sum.to_f64() - 1e10).abs() < 1.0);
}

#[test]
fn add_cancellation() {
    // Test catastrophic cancellation: 1.0 - (1.0 - 1e-10)
    // Note: 1e-15 is beyond f64 precision difference from 1.0, so we use 1e-10
    let a = HDRFloat::from_f64(1.0);
    let b = HDRFloat::from_f64(1.0 - 1e-10);
    let diff = a.sub(&b);
    let expected = 1e-10;
    assert!(
        (diff.to_f64() - expected).abs() < expected * 1e-6,
        "Cancellation: got {}, expected {}",
        diff.to_f64(),
        expected
    );
}

#[test]
fn sub_basic() {
    let a = HDRFloat::from_f64(5.0);
    let b = HDRFloat::from_f64(3.0);
    assert!((a.sub(&b).to_f64() - 2.0).abs() < 1e-14);
}

#[test]
fn from_bigfloat_f64_range() {
    let bf = BigFloat::with_precision(1.234567, 128);
    let h = HDRFloat::from_bigfloat(&bf);
    assert!((h.to_f64() - 1.234567).abs() < 1e-10);
}

#[test]
fn from_bigfloat_zero() {
    let bf = BigFloat::zero(128);
    let h = HDRFloat::from_bigfloat(&bf);
    assert!(h.is_zero());
}

#[test]
fn from_bigfloat_extreme_small() {
    // 10^-100 is beyond f64 range but within HDRFloat
    let bf = BigFloat::from_string("1e-100", 512).unwrap();
    let h = HDRFloat::from_bigfloat(&bf);

    assert!(!h.is_zero(), "Should not underflow to zero");
    // Exponent should be approximately -100 * log2(10) ≈ -332
    assert!(h.exp < -300, "Exponent {} should be < -300", h.exp);
    assert!(h.exp > -400, "Exponent {} should be > -400", h.exp);
}

#[test]
fn complex_add() {
    let a = HDRComplex {
        re: HDRFloat::from_f64(1.0),
        im: HDRFloat::from_f64(2.0),
    };
    let b = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let c = a.add(&b);
    assert!((c.re.to_f64() - 4.0).abs() < 1e-14);
    assert!((c.im.to_f64() - 6.0).abs() < 1e-14);
}

#[test]
fn complex_mul() {
    // (1 + 2i) * (3 + 4i) = (1*3 - 2*4) + (1*4 + 2*3)i = -5 + 10i
    let a = HDRComplex {
        re: HDRFloat::from_f64(1.0),
        im: HDRFloat::from_f64(2.0),
    };
    let b = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let c = a.mul(&b);
    assert!((c.re.to_f64() - (-5.0)).abs() < 1e-14);
    assert!((c.im.to_f64() - 10.0).abs() < 1e-14);
}

#[test]
fn complex_norm_sq() {
    // |3 + 4i|² = 9 + 16 = 25
    let c = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    assert!((c.norm_sq() - 25.0).abs() < 1e-14);
}

#[test]
fn complex_square() {
    // (3 + 4i)² = 9 - 16 + 24i = -7 + 24i
    let c = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let sq = c.square();
    assert!((sq.re.to_f64() - (-7.0)).abs() < 1e-14);
    assert!((sq.im.to_f64() - 24.0).abs() < 1e-14);
}

#[test]
fn complex_square_zero() {
    let c = HDRComplex::ZERO;
    let sq = c.square();
    assert!(sq.is_zero());
}

#[test]
fn complex_mul_zero() {
    let a = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(4.0),
    };
    let z = HDRComplex::ZERO;
    assert!(a.mul(&z).is_zero());
    assert!(z.mul(&a).is_zero());
}

#[test]
fn complex_sub() {
    let a = HDRComplex {
        re: HDRFloat::from_f64(5.0),
        im: HDRFloat::from_f64(7.0),
    };
    let b = HDRComplex {
        re: HDRFloat::from_f64(3.0),
        im: HDRFloat::from_f64(2.0),
    };
    let c = a.sub(&b);
    assert!((c.re.to_f64() - 2.0).abs() < 1e-14);
    assert!((c.im.to_f64() - 5.0).abs() < 1e-14);
}

#[test]
fn complex_is_zero() {
    assert!(HDRComplex::ZERO.is_zero());

    let non_zero = HDRComplex {
        re: HDRFloat::from_f64(1.0),
        im: HDRFloat::ZERO,
    };
    assert!(!non_zero.is_zero());
}

#[test]
fn mul_f64_basic() {
    let h = HDRFloat::from_f64(2.0);
    let result = h.mul_f64(3.0);
    assert!((result.to_f64() - 6.0).abs() < 1e-14);
}

#[test]
fn mul_f64_preserves_precision() {
    // Multiply HDRFloat by f64 reference orbit value
    let h = HDRFloat::from_f64(1e-50);
    let z_m: f64 = 0.123456789012345; // Reference orbit value
    let result = h.mul_f64(z_m);
    let expected: f64 = 1e-50 * z_m;
    assert!(
        (result.to_f64() - expected).abs() < expected.abs() * 1e-14,
        "mul_f64: got {}, expected {}",
        result.to_f64(),
        expected
    );
}

#[test]
fn div_f64_basic() {
    let h = HDRFloat::from_f64(6.0);
    let result = h.div_f64(2.0);
    assert!(
        (result.to_f64() - 3.0).abs() < 1e-14,
        "div_f64: got {}, expected 3.0",
        result.to_f64()
    );
}

#[test]
fn div_f64_preserves_extended_exponent() {
    // This is the critical test: dividing a very small value by an image dimension
    // At 10^14 zoom, viewport width might be ~10^-14
    // Dividing by image width (e.g., 1920) should preserve the extended exponent
    let bf = BigFloat::from_string("1e-50", 256).unwrap();
    let h = HDRFloat::from_bigfloat(&bf);

    // Simulate dividing viewport by image width
    let image_width = 1920.0;
    let result = h.div_f64(image_width);

    // Expected: 1e-50 / 1920 ≈ 5.2e-54
    let expected = 1e-50 / 1920.0;
    let rel_error = (result.to_f64() - expected).abs() / expected.abs();
    assert!(
        rel_error < 1e-6,
        "div_f64 with extended exponent: got {}, expected {}, rel_error={}",
        result.to_f64(),
        expected,
        rel_error
    );

    // Most importantly: the exponent should be preserved (not underflowed)
    assert!(
        result.exp < -150,
        "Exponent {} should be < -150 for 1e-50 / 1920",
        result.exp
    );
}

#[test]
fn div_f64_extreme_small_value() {
    // Test with value beyond f64 range
    let bf = BigFloat::from_string("1e-100", 512).unwrap();
    let h = HDRFloat::from_bigfloat(&bf);

    // Dividing by image dimension should still work
    let result = h.div_f64(1920.0);

    // Result should not be zero (would happen if we used to_f64() / divisor)
    assert!(
        !result.is_zero(),
        "div_f64 should not underflow to zero for 1e-100 / 1920"
    );

    // Exponent should be approximately -100*log2(10) - log2(1920) ≈ -332 - 11 = -343
    assert!(
        result.exp < -300,
        "Exponent {} should be < -300 for 1e-100 / 1920",
        result.exp
    );
}

#[test]
fn div_f64_zero_dividend() {
    let h = HDRFloat::ZERO;
    let result = h.div_f64(5.0);
    assert!(result.is_zero());
}

#[test]
fn div_f64_renderer_scenario() {
    // Exact values from the renderer log at zoom ~10^14:
    // vp_width: (0.59008217, 0.000000019070486, -43)
    // vp_height: (0.5756648, 0.0000000035213406, -44)
    // image dimensions: 1146x559
    //
    // At this zoom level, the pixel steps coincidentally round to the same
    // f32 head value (0.52726364) due to f32's limited precision (~7 decimal digits).
    // This is expected behavior - the perturbation algorithm still works because
    // each pixel gets a unique dc value from (x * step_re, y * step_im).
    let vp_width = HDRFloat {
        head: 0.59008217,
        tail: 0.000000019070486,
        exp: -43,
    };
    let vp_height = HDRFloat {
        head: 0.5756648,
        tail: 0.0000000035213406,
        exp: -44,
    };

    let step_re = vp_width.div_f64(1146.0);
    let step_im = vp_height.div_f64(559.0);

    // Verify the exponents are correctly computed
    // vp_width has exp -43, 1146 ≈ 2^10.16, so result exp should be around -53 or -54
    assert!(
        step_re.exp == -53 || step_re.exp == -54,
        "step_re.exp = {}",
        step_re.exp
    );
    assert!(
        step_im.exp == -53 || step_im.exp == -54,
        "step_im.exp = {}",
        step_im.exp
    );

    // The values should be non-zero and normalized
    assert!(!step_re.is_zero());
    assert!(!step_im.is_zero());
    assert!(step_re.head.abs() >= 0.5 && step_re.head.abs() < 1.0);
    assert!(step_im.head.abs() >= 0.5 && step_im.head.abs() < 1.0);
}

#[test]
fn hdr_complex_delta_zero() {
    use fractalwonder_core::ComplexDelta;
    let c = HDRComplex::from_f64_pair(1.0, 2.0);
    let z = c.zero();
    assert!(z.re.is_zero());
    assert!(z.im.is_zero());
}

#[test]
fn hdr_complex_delta_add() {
    use fractalwonder_core::ComplexDelta;
    let a = HDRComplex::from_f64_pair(1.0, 2.0);
    let b = HDRComplex::from_f64_pair(3.0, 4.0);
    let c = a.add(&b);
    let (re, im) = c.to_f64_pair();
    assert!((re - 4.0).abs() < 1e-10);
    assert!((im - 6.0).abs() < 1e-10);
}

#[test]
fn hdr_complex_delta_mul() {
    use fractalwonder_core::ComplexDelta;
    // (1 + 2i) * (3 + 4i) = -5 + 10i
    let a = HDRComplex::from_f64_pair(1.0, 2.0);
    let b = HDRComplex::from_f64_pair(3.0, 4.0);
    let c = a.mul(&b);
    let (re, im) = c.to_f64_pair();
    assert!((re - (-5.0)).abs() < 1e-10);
    assert!((im - 10.0).abs() < 1e-10);
}

#[test]
fn hdr_complex_delta_norm_sq() {
    use fractalwonder_core::ComplexDelta;
    // |3 + 4i|² = 25
    let a = HDRComplex::from_f64_pair(3.0, 4.0);
    let norm = a.norm_sq();
    assert!((norm - 25.0).abs() < 1e-10);
}

// NOTE: sqrt() and div() tests are in hdrfloat_extended_arithmetic.rs
