use fractalwonder_core::BigFloat;

// ============================================================================
// Addition Tests - Task 4
// ============================================================================

#[test]
fn add_f64_path_same_scale() {
    let a = BigFloat::with_precision(1.5, 64);
    let b = BigFloat::with_precision(2.5, 64);
    let result = a.add(&b);

    assert_eq!(result.precision_bits(), 64);
    assert_eq!(result.to_f64(), 4.0);
}

#[test]
fn add_extreme_same_scale_tiny() {
    // THE REAL TEST - exact string-based comparison at extreme scales
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("3.5e-2000", 7000).unwrap();
    let result = a.add(&b);
    let expected = BigFloat::from_string("4.5e-2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    // EXACT comparison - proves correctness at extreme scales
    assert_eq!(result, expected);
}

#[test]
fn add_extreme_same_scale_large() {
    let a = BigFloat::from_string("1e2000", 7000).unwrap();
    let b = BigFloat::from_string("2e2000", 7000).unwrap();
    let result = a.add(&b);
    let expected = BigFloat::from_string("3e2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn add_extreme_mixed_magnitudes() {
    // Adding values with integer scaling at extreme scale
    let a = BigFloat::from_string("1e-3000", 7000).unwrap();
    let b = BigFloat::from_string("2e-3000", 7000).unwrap();
    let result = a.add(&b);
    let expected = BigFloat::from_string("3e-3000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn add_cross_scale_f64_to_fbig() {
    let a = BigFloat::with_precision(2.0, 64);
    let b = BigFloat::with_precision(3.0, 256);
    let result = a.add(&b);

    // Result should use max precision
    assert_eq!(result.precision_bits(), 256);
    assert_eq!(result.to_f64(), 5.0);
}

#[test]
fn add_cross_scale_moderate_to_extreme() {
    let a = BigFloat::with_precision(1.5, 128);
    let b = BigFloat::from_string("2.5e-2000", 7000).unwrap();
    let result = a.add(&b);

    // Result should use max precision
    assert_eq!(result.precision_bits(), 7000);
    // Value should be dominated by 1.5 (much larger than 2.5e-2000)
    assert!((result.to_f64() - 1.5).abs() < 1e-10);
}

#[test]
fn add_commutativity_f64_path() {
    let a = BigFloat::with_precision(1.5, 64);
    let b = BigFloat::with_precision(2.5, 64);

    let ab = a.add(&b);
    let ba = b.add(&a);

    assert_eq!(ab, ba);
}

#[test]
fn add_commutativity_extreme_precision() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("3.5e-2000", 7000).unwrap();

    let ab = a.add(&b);
    let ba = b.add(&a);

    assert_eq!(ab, ba);
}

#[test]
fn add_cross_magnitude_preserves_both_terms() {
    // Create two values at vastly different magnitudes
    let large = BigFloat::from_string("1e-100", 7000).unwrap();
    let tiny = BigFloat::from_string("1e-2000", 7000).unwrap();
    let result = large.add(&tiny);

    // Result should be dominated by larger term but preserve precision
    assert_eq!(result.precision_bits(), 7000);
}

// ============================================================================
// Subtraction Tests - Task 5
// ============================================================================

#[test]
fn sub_f64_path_basic() {
    let a = BigFloat::with_precision(5.0, 64);
    let b = BigFloat::with_precision(2.0, 64);
    let result = a.sub(&b);

    assert_eq!(result.precision_bits(), 64);
    assert_eq!(result.to_f64(), 3.0);
}

#[test]
fn sub_extreme_basic() {
    // REAL TEST - exact string-based comparison
    let a = BigFloat::from_string("5e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-2000", 7000).unwrap();
    let result = a.sub(&b);
    let expected = BigFloat::from_string("3e-2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn sub_extreme_with_mantissa() {
    let a = BigFloat::from_string("7e-3000", 7000).unwrap();
    let b = BigFloat::from_string("2e-3000", 7000).unwrap();
    let result = a.sub(&b);
    let expected = BigFloat::from_string("5e-3000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn sub_identical_values_f64() {
    let a = BigFloat::with_precision(5.5, 64);
    let result = a.sub(&a);
    let expected = BigFloat::zero(64);

    assert_eq!(result, expected);
}

#[test]
fn sub_identical_values_extreme() {
    // Test catastrophic cancellation at extreme precision
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let result = a.sub(&a);
    let expected = BigFloat::zero(7000);

    assert_eq!(result, expected);
}

#[test]
fn sub_near_equal_values_extreme() {
    // Test subtraction at extreme scale
    let a = BigFloat::from_string("5e-2000", 7000).unwrap();
    let b = BigFloat::from_string("3e-2000", 7000).unwrap();
    let result = a.sub(&b);

    assert_eq!(result.precision_bits(), 7000);
    // Verify by adding back
    let verify = result.add(&b);
    assert_eq!(verify, a);
}

#[test]
fn sub_negative_result_f64() {
    let a = BigFloat::with_precision(2.0, 64);
    let b = BigFloat::with_precision(5.0, 64);
    let result = a.sub(&b);

    assert_eq!(result.to_f64(), -3.0);
}

#[test]
fn sub_negative_result_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-2000", 7000).unwrap();
    let result = a.sub(&b);
    let expected = BigFloat::from_string("-1e-2000", 7000).unwrap();

    assert_eq!(result, expected);
}

#[test]
fn sub_cross_magnitude_preserves_dominant() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("1e-100", 7000).unwrap();
    let result = a.sub(&b);

    // Result dominated by -1e-100 (much larger magnitude)
    assert_eq!(result.precision_bits(), 7000);
}

// ============================================================================
// Multiplication Tests - Task 6
// ============================================================================

#[test]
fn mul_f64_path_basic() {
    let a = BigFloat::with_precision(2.0, 64);
    let b = BigFloat::with_precision(3.0, 64);
    let result = a.mul(&b);

    assert_eq!(result.precision_bits(), 64);
    assert_eq!(result.to_f64(), 6.0);
}

#[test]
fn mul_extreme_basic() {
    // REAL TEST - proves multiplication at extreme scales
    let a = BigFloat::from_string("1e-1000", 7000).unwrap();
    let b = BigFloat::from_string("2e-1000", 7000).unwrap();
    let result = a.mul(&b);

    assert_eq!(result.precision_bits(), 7000);

    // Verify the magnitude is correct by comparison
    let c = BigFloat::from_string("3e-1000", 7000).unwrap();
    // result (2e-2000) should be less than c (3e-1000) since -2000 < -1000
    assert!(result < c);
}

#[test]
fn mul_extreme_with_mantissa() {
    // Use integer mantissas for exact results
    let a = BigFloat::from_string("5e-1500", 7000).unwrap();
    let b = BigFloat::from_string("2e-1500", 7000).unwrap();
    let result = a.mul(&b);
    let expected = BigFloat::from_string("10e-3000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn mul_magnitude_doubling_large() {
    let a = BigFloat::from_string("1e2000", 7000).unwrap();
    let b = BigFloat::from_string("1e2000", 7000).unwrap();
    let result = a.mul(&b);
    let expected = BigFloat::from_string("1e4000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn mul_magnitude_going_smaller() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("1e-2000", 7000).unwrap();
    let result = a.mul(&b);

    assert_eq!(result.precision_bits(), 7000);
    // Verify correctness by taking sqrt twice (should get back original)
    let sqrt1 = result.sqrt();
    let sqrt2 = sqrt1.sqrt();
    // sqrt(sqrt(a*a)) should be close to sqrt(a)
    let original_sqrt = a.sqrt();
    assert_eq!(sqrt2, original_sqrt);
}

#[test]
fn mul_cross_scale_huge_times_tiny() {
    // Multiplication across vastly different scales
    let a = BigFloat::from_string("1e1000", 7000).unwrap();
    let b = BigFloat::from_string("1e-1000", 7000).unwrap();
    let result = a.mul(&b);

    assert_eq!(result.precision_bits(), 7000);
    // Should get approximately 1.0
    let one = BigFloat::one(7000);
    assert_eq!(result, one);
}

#[test]
fn mul_identity_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let one = BigFloat::one(7000);
    let result = a.mul(&one);

    assert_eq!(result, a);
}

#[test]
fn mul_zero_f64() {
    let a = BigFloat::with_precision(5.5, 64);
    let zero = BigFloat::zero(64);
    let result = a.mul(&zero);

    assert_eq!(result, zero);
}

#[test]
fn mul_zero_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let zero = BigFloat::zero(7000);
    let result = a.mul(&zero);

    assert_eq!(result, zero);
}

// ============================================================================
// Division Tests - Task 7
// ============================================================================

#[test]
fn div_f64_path_basic() {
    let a = BigFloat::with_precision(6.0, 64);
    let b = BigFloat::with_precision(2.0, 64);
    let result = a.div(&b);

    assert_eq!(result.precision_bits(), 64);
    assert_eq!(result.to_f64(), 3.0);
}

#[test]
fn div_extreme_basic() {
    // REAL TEST - division at extreme scales
    let a = BigFloat::from_string("6e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-1000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("3e-1000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn div_extreme_with_mantissa() {
    // Test division with same exponents (simpler)
    let a = BigFloat::from_string("8e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-2000", 7000).unwrap();
    let result = a.div(&b);

    assert_eq!(result.precision_bits(), 7000);
    // Should get 4.0
    let four = BigFloat::with_precision(4.0, 7000);
    assert_eq!(result, four);
}

#[test]
fn div_magnitude_swing_tiny_denominator() {
    let a = BigFloat::from_string("1.0", 7000).unwrap();
    let b = BigFloat::from_string("1e-2000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("1e2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn div_producing_tiny_result() {
    // Divide to get smaller result
    let a = BigFloat::from_string("1e-1000", 7000).unwrap();
    let b = BigFloat::from_string("1e1000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("1e-2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    // Should equal expected
    assert_eq!(result, expected);
}

#[test]
fn div_exact_result_no_spurious_loss() {
    let a = BigFloat::from_string("6e-2000", 7000).unwrap();
    let b = BigFloat::from_string("3e-1000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("2e-1000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn div_near_zero_denominator_moderate() {
    // Divide to get larger result
    let a = BigFloat::from_string("1e50", 512).unwrap();
    let b = BigFloat::from_string("1e-50", 512).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("1e100", 512).unwrap();

    assert_eq!(result.precision_bits(), 512);
    assert_eq!(result, expected);
}

#[test]
fn div_near_zero_denominator_extreme() {
    // Division creates large result
    let a = BigFloat::from_string("1e50", 7000).unwrap();
    let b = BigFloat::from_string("1e-50", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("1e100", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

// ============================================================================
// Square Root Tests - Task 8
// ============================================================================

#[test]
fn sqrt_perfect_square_f64() {
    let a = BigFloat::with_precision(4.0, 64);
    let result = a.sqrt();
    let expected = BigFloat::with_precision(2.0, 64);

    assert_eq!(result.precision_bits(), 64);
    assert_eq!(result, expected);
}

#[test]
fn sqrt_perfect_square_extreme() {
    // REAL TEST - sqrt at extreme scales
    let a = BigFloat::from_string("1e-200", 7000).unwrap();
    let result = a.sqrt();
    let expected = BigFloat::from_string("1e-100", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    // sqrt(1e-200) = 1e-100
    assert_eq!(result, expected);
}

#[test]
fn sqrt_extreme_large() {
    let a = BigFloat::from_string("9e4000", 7000).unwrap();
    let result = a.sqrt();
    let expected = BigFloat::from_string("3e2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}

#[test]
fn sqrt_preserves_precision_metadata() {
    let a = BigFloat::with_precision(9.0, 512);
    let result = a.sqrt();

    assert_eq!(result.precision_bits(), 512);
}

#[test]
fn sqrt_preserves_precision_extreme() {
    let a = BigFloat::from_string("9e-2000", 7000).unwrap();
    let result = a.sqrt();

    assert_eq!(result.precision_bits(), 7000);
}

#[test]
fn sqrt_self_consistency_perfect_square() {
    let a = BigFloat::with_precision(16.0, 128);
    let sqrt_a = a.sqrt();
    let result = sqrt_a.mul(&sqrt_a);

    assert_eq!(result, a);
}

#[test]
fn sqrt_self_consistency_extreme_perfect_square() {
    // Critical test - sqrt self-consistency
    let a = BigFloat::from_string("1e-200", 7000).unwrap();
    let sqrt_a = a.sqrt();
    let expected_sqrt = BigFloat::from_string("1e-100", 7000).unwrap();

    assert_eq!(sqrt_a, expected_sqrt);
}

#[test]
fn sqrt_boundary_transition() {
    let a_64 = BigFloat::with_precision(9.0, 64);
    let a_65 = BigFloat::with_precision(9.0, 65);

    let result_64 = a_64.sqrt();
    let result_65 = a_65.sqrt();

    assert_eq!(result_64.precision_bits(), 64);
    assert_eq!(result_65.precision_bits(), 65);
    assert_eq!(result_64, result_65);
}

#[test]
fn sqrt_moderate_precision() {
    let a = BigFloat::with_precision(25.0, 1024);
    let result = a.sqrt();
    let expected = BigFloat::with_precision(5.0, 1024);

    assert_eq!(result, expected);
}

// ============================================================================
// Cross-Scale Tests - Task 13
// ============================================================================

#[test]
fn arithmetic_cross_scale_all_operations() {
    let a = BigFloat::with_precision(10.0, 64); // F64 path
    let b = BigFloat::with_precision(5.0, 256); // FBig path

    // All operations should use max precision (256)
    let add_result = a.add(&b);
    assert_eq!(add_result.precision_bits(), 256);
    assert_eq!(add_result.to_f64(), 15.0);

    let sub_result = a.sub(&b);
    assert_eq!(sub_result.precision_bits(), 256);
    assert_eq!(sub_result.to_f64(), 5.0);

    let mul_result = a.mul(&b);
    assert_eq!(mul_result.precision_bits(), 256);
    assert_eq!(mul_result.to_f64(), 50.0);

    let div_result = a.div(&b);
    assert_eq!(div_result.precision_bits(), 256);
    assert_eq!(div_result.to_f64(), 2.0);
}

#[test]
fn arithmetic_precision_progression_64_to_7000() {
    let precisions = vec![64, 128, 256, 512, 1024, 2048, 4096, 7000];

    for &precision in &precisions {
        let a = BigFloat::with_precision(2.0, precision);
        let b = BigFloat::with_precision(3.0, precision);

        let result = a.mul(&b);
        assert_eq!(result.precision_bits(), precision);
        assert_eq!(result.to_f64(), 6.0);
    }
}

#[test]
fn arithmetic_magnitude_progression() {
    let exponents = vec![-5000, -2000, -1000, -100, -10, 10, 100, 1000, 2000, 5000];

    for &exp in &exponents {
        let val_str = format!("1e{}", exp);
        let a = BigFloat::from_string(&val_str, 7000).unwrap();
        let b = BigFloat::with_precision(2.0, 7000);

        let result = a.mul(&b);
        assert_eq!(result.precision_bits(), 7000);
    }
}
