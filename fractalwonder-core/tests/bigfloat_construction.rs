use fractalwonder_core::BigFloat;

// ============================================================================
// with_precision() tests
// ============================================================================
// Test precision metadata and path selection using cross-path equality
// and string comparison - NOT to_f64()

#[test]
fn with_precision_f64_path_32_bits() {
    let bf = BigFloat::with_precision(1.5, 32);
    assert_eq!(bf.precision_bits(), 32);
    assert_eq!(bf.to_string(), "1.5");
}

#[test]
fn with_precision_f64_path_64_bits() {
    let bf = BigFloat::with_precision(2.5, 64);
    assert_eq!(bf.precision_bits(), 64);
    assert_eq!(bf.to_string(), "2.5");
}

#[test]
fn with_precision_fbig_path_128_bits() {
    let bf = BigFloat::with_precision(1.5, 128);
    assert_eq!(bf.precision_bits(), 128);
    // Verify value via cross-path equality
    let bf_f64 = BigFloat::with_precision(1.5, 64);
    assert_eq!(bf, bf_f64);
}

#[test]
fn with_precision_fbig_path_7000_bits() {
    let bf = BigFloat::with_precision(3.0, 7000);
    assert_eq!(bf.precision_bits(), 7000);
    // Verify value via cross-path equality
    let bf_f64 = BigFloat::with_precision(3.0, 64);
    assert_eq!(bf, bf_f64);
}

#[test]
fn with_precision_boundary_64_to_65_bits() {
    let bf_64 = BigFloat::with_precision(1.5, 64);
    let bf_65 = BigFloat::with_precision(1.5, 65);

    assert_eq!(bf_64.precision_bits(), 64);
    assert_eq!(bf_65.precision_bits(), 65);
    // Cross-path equality
    assert_eq!(bf_64, bf_65);
}

#[test]
fn with_precision_zero_f64_path() {
    let bf = BigFloat::with_precision(0.0, 32);
    assert_eq!(bf.precision_bits(), 32);
    assert_eq!(bf, BigFloat::zero(32));
}

#[test]
fn with_precision_zero_fbig_path() {
    let bf = BigFloat::with_precision(0.0, 7000);
    assert_eq!(bf.precision_bits(), 7000);
    assert_eq!(bf, BigFloat::zero(7000));
}

// ============================================================================
// zero() and one() constructor tests
// ============================================================================

#[test]
fn zero_f64_path() {
    let z = BigFloat::zero(32);
    assert_eq!(z.precision_bits(), 32);
    assert_eq!(z.to_string(), "0");
}

#[test]
fn zero_fbig_path() {
    let z = BigFloat::zero(128);
    assert_eq!(z.precision_bits(), 128);
    // Cross-path equality - zero is zero
    assert_eq!(z, BigFloat::zero(64));
}

#[test]
fn zero_extreme_precision() {
    let z = BigFloat::zero(7000);
    assert_eq!(z.precision_bits(), 7000);
    assert_eq!(z, BigFloat::zero(64));
}

#[test]
fn one_f64_path() {
    let o = BigFloat::one(32);
    assert_eq!(o.precision_bits(), 32);
    assert_eq!(o.to_string(), "1");
}

#[test]
fn one_fbig_path() {
    let o = BigFloat::one(128);
    assert_eq!(o.precision_bits(), 128);
    assert_eq!(o, BigFloat::one(64));
}

#[test]
fn one_extreme_precision() {
    let o = BigFloat::one(7000);
    assert_eq!(o.precision_bits(), 7000);
    assert_eq!(o, BigFloat::one(64));
}

// ============================================================================
// Mathematical identity tests at EXTREME scales
// ============================================================================

#[test]
fn zero_identity_addition_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let z = BigFloat::zero(7000);
    let result = a.add(&z);
    assert_eq!(result, a);
}

#[test]
fn one_identity_multiplication_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let o = BigFloat::one(7000);
    let result = a.mul(&o);
    assert_eq!(result, a);
}

// ============================================================================
// from_string() tests - EXTREME VALUE TESTS
// ============================================================================

#[test]
fn from_string_extreme_tiny() {
    let bf = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    // Verify it's positive and non-zero via comparison
    assert!(bf > BigFloat::zero(7000));
}

#[test]
fn from_string_extreme_large() {
    let bf = BigFloat::from_string("3.5e2000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    assert!(bf > BigFloat::one(7000));
}

#[test]
fn from_string_with_mantissa_extreme() {
    let bf = BigFloat::from_string("1.23456789e-1000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    assert!(bf > BigFloat::zero(7000));
}

#[test]
fn from_string_f64_path_normal_value() {
    let bf = BigFloat::from_string("1.5", 64).unwrap();
    assert_eq!(bf.precision_bits(), 64);
    assert_eq!(bf, BigFloat::with_precision(1.5, 64));
}

#[test]
fn from_string_fbig_path_high_precision() {
    let bf = BigFloat::from_string("1.5", 128).unwrap();
    assert_eq!(bf.precision_bits(), 128);
    assert_eq!(bf, BigFloat::with_precision(1.5, 128));
}

#[test]
fn from_string_extreme_tiny_5000() {
    let bf = BigFloat::from_string("1e-5000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    assert!(bf > BigFloat::zero(7000));
    // It should be smaller than 1e-2000
    let larger = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert!(bf < larger);
}

#[test]
fn from_string_extreme_large_5000() {
    let bf = BigFloat::from_string("1e5000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    // It should be larger than 1e2000
    let smaller = BigFloat::from_string("1e2000", 7000).unwrap();
    assert!(bf > smaller);
}

#[test]
fn from_string_invalid_format() {
    let result = BigFloat::from_string("not_a_number", 128);
    assert!(result.is_err());
}

#[test]
fn from_string_empty_string() {
    let result = BigFloat::from_string("", 128);
    assert!(result.is_err());
}

// ============================================================================
// CRITICAL: Prove arithmetic correctness at extreme scales
// ============================================================================

#[test]
fn arithmetic_add_extreme() {
    // 1e-2000 + 3.5e-2000 = 4.5e-2000
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("3.5e-2000", 7000).unwrap();
    let result = a.add(&b);
    let expected = BigFloat::from_string("4.5e-2000", 7000).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn arithmetic_sub_extreme() {
    // 5e-2000 - 2e-2000 = 3e-2000
    let a = BigFloat::from_string("5e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-2000", 7000).unwrap();
    let result = a.sub(&b);
    let expected = BigFloat::from_string("3e-2000", 7000).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn arithmetic_mul_extreme() {
    // Use values that result in exact binary representations
    // 2e-1000 * 2e-1000 = 4e-2000
    let a = BigFloat::from_string("2e-1000", 7000).unwrap();
    let b = BigFloat::from_string("2e-1000", 7000).unwrap();
    let result = a.mul(&b);
    let expected = BigFloat::from_string("4e-2000", 7000).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn arithmetic_div_extreme() {
    // 6e-2000 / 2e-1000 = 3e-1000
    let a = BigFloat::from_string("6e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-1000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("3e-1000", 7000).unwrap();
    assert_eq!(result, expected);
}

// ============================================================================
// to_f64() - ONLY for testing boundary behavior (underflow/overflow)
// These are the ONLY legitimate uses of to_f64()
// ============================================================================

#[test]
fn to_f64_extreme_large_overflows_to_infinity() {
    // This tests that values beyond f64 range overflow correctly
    let bf = BigFloat::from_string("1e2000", 7000).unwrap();
    assert_eq!(bf.to_f64(), f64::INFINITY);
}

#[test]
fn to_f64_extreme_tiny_underflows_to_zero() {
    // This tests that values beyond f64 range underflow correctly
    let bf = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert_eq!(bf.to_f64(), 0.0);
}
