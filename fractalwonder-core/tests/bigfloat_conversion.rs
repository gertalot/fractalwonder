use fractalwonder_core::BigFloat;

// ============================================================================
// Task 9: Conversion Tests - precision_bits() and to_f64()
// ============================================================================

// ============================================================================
// precision_bits() getter tests
// ============================================================================

#[test]
fn precision_bits_with_precision_constructor() {
    let bf = BigFloat::with_precision(1.5, 128);
    assert_eq!(bf.precision_bits(), 128);
}

#[test]
fn precision_bits_from_string_constructor() {
    let bf = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
}

#[test]
fn precision_bits_zero_constructor() {
    let z = BigFloat::zero(512);
    assert_eq!(z.precision_bits(), 512);
}

#[test]
fn precision_bits_one_constructor() {
    let o = BigFloat::one(256);
    assert_eq!(o.precision_bits(), 256);
}

// ============================================================================
// precision_bits() after arithmetic tests
// ============================================================================

#[test]
fn precision_bits_after_add_same_precision() {
    let a = BigFloat::with_precision(1.5, 128);
    let b = BigFloat::with_precision(2.5, 128);
    let result = a.add(&b);

    assert_eq!(result.precision_bits(), 128);
}

#[test]
fn precision_bits_after_add_cross_precision() {
    let a = BigFloat::with_precision(1.5, 64);
    let b = BigFloat::with_precision(2.5, 256);
    let result = a.add(&b);

    assert_eq!(result.precision_bits(), 256); // max
}

#[test]
fn precision_bits_after_mul_cross_precision_extreme() {
    let a = BigFloat::with_precision(2.0, 128);
    let b = BigFloat::from_string("3e-2000", 7000).unwrap();
    let result = a.mul(&b);

    assert_eq!(result.precision_bits(), 7000); // max
}

// ============================================================================
// Value verification tests (using string comparison, NOT to_f64())
// ============================================================================
// to_f64() destroys precision - use to_string() for value verification

#[test]
fn value_verification_f64_path() {
    let bf = BigFloat::with_precision(1.5, 64);
    assert_eq!(bf.to_string(), "1.5");
}

#[test]
fn value_verification_fbig_path() {
    // FBig Display uses binary representation, so use cross-path equality
    let bf_fbig = BigFloat::with_precision(2.5, 128);
    let bf_f64 = BigFloat::with_precision(2.5, 64);
    assert_eq!(bf_fbig, bf_f64);
}

#[test]
fn value_verification_cross_path_equality() {
    // Verify same value across paths via direct comparison
    let bf_f64 = BigFloat::with_precision(7.5, 64);
    let bf_fbig = BigFloat::with_precision(7.5, 128);
    assert_eq!(bf_f64, bf_fbig);
}

// ============================================================================
// to_f64() boundary tests - LEGITIMATE uses of to_f64()
// ============================================================================
// These tests verify the BEHAVIOR of to_f64() at boundaries (overflow/underflow)
// NOT used for verifying BigFloat precision/correctness

#[test]
fn to_f64_extreme_large_becomes_infinity() {
    let bf = BigFloat::from_string("1e2000", 7000).unwrap();
    assert_eq!(bf.to_f64(), f64::INFINITY);
}

#[test]
fn to_f64_extreme_tiny_becomes_zero() {
    let bf = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert_eq!(bf.to_f64(), 0.0);
}

// ============================================================================
// to_f64() at f64 range boundaries - LEGITIMATE uses
// ============================================================================
// These test that values at f64 limits convert correctly (not precision tests)

#[test]
fn to_f64_at_f64_max() {
    let bf = BigFloat::from_string("1.7976931348623157e308", 128).unwrap();
    assert_eq!(bf.to_f64(), f64::MAX);
}

#[test]
fn to_f64_at_f64_min_positive() {
    let bf = BigFloat::from_string("2.2250738585072014e-308", 128).unwrap();
    assert_eq!(bf.to_f64(), f64::MIN_POSITIVE);
}
