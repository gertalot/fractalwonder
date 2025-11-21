use fractalwonder_core::BigFloat;

// ============================================================================
// with_precision() tests
// ============================================================================
// These test precision METADATA and path selection, NOT extreme values.
// Extreme value testing happens in from_string() tests and arithmetic tests.

#[test]
fn with_precision_f64_path_32_bits() {
    let bf = BigFloat::with_precision(1.5, 32);
    assert_eq!(bf.precision_bits(), 32);
}

#[test]
fn with_precision_f64_path_64_bits() {
    let bf = BigFloat::with_precision(2.5, 64);
    assert_eq!(bf.precision_bits(), 64);
}

#[test]
fn with_precision_fbig_path_128_bits() {
    let bf = BigFloat::with_precision(1.5, 128);
    assert_eq!(bf.precision_bits(), 128);
}

#[test]
fn with_precision_fbig_path_7000_bits() {
    let bf = BigFloat::with_precision(3.14159, 7000);
    assert_eq!(bf.precision_bits(), 7000);
}

#[test]
fn with_precision_boundary_64_to_65_bits() {
    let bf_64 = BigFloat::with_precision(1.5, 64);
    let bf_65 = BigFloat::with_precision(1.5, 65);

    // Both should have correct precision metadata
    assert_eq!(bf_64.precision_bits(), 64);
    assert_eq!(bf_65.precision_bits(), 65);

    // Both should have same mathematical value (cross-path equality)
    assert_eq!(bf_64, bf_65);
}

#[test]
fn with_precision_zero_f64_path() {
    let bf = BigFloat::with_precision(0.0, 32);
    assert_eq!(bf.precision_bits(), 32);
    assert_eq!(bf.to_f64(), 0.0);
}

#[test]
fn with_precision_zero_fbig_path() {
    // This exercises the special zero handling in the code
    let bf = BigFloat::with_precision(0.0, 7000);
    assert_eq!(bf.precision_bits(), 7000);
    assert_eq!(bf.to_f64(), 0.0);
}

// ============================================================================
// zero() and one() constructor tests
// ============================================================================

#[test]
fn zero_f64_path() {
    let z = BigFloat::zero(32);
    assert_eq!(z.precision_bits(), 32);
    assert_eq!(z.to_f64(), 0.0);
}

#[test]
fn zero_fbig_path() {
    let z = BigFloat::zero(128);
    assert_eq!(z.precision_bits(), 128);
    assert_eq!(z.to_f64(), 0.0);
}

#[test]
fn zero_extreme_precision() {
    let z = BigFloat::zero(7000);
    assert_eq!(z.precision_bits(), 7000);
    assert_eq!(z.to_f64(), 0.0);
}

#[test]
fn one_f64_path() {
    let o = BigFloat::one(32);
    assert_eq!(o.precision_bits(), 32);
    assert_eq!(o.to_f64(), 1.0);
}

#[test]
fn one_fbig_path() {
    let o = BigFloat::one(128);
    assert_eq!(o.precision_bits(), 128);
    assert_eq!(o.to_f64(), 1.0);
}

#[test]
fn one_extreme_precision() {
    let o = BigFloat::one(7000);
    assert_eq!(o.precision_bits(), 7000);
    assert_eq!(o.to_f64(), 1.0);
}

// ============================================================================
// Mathematical identity tests with zero() and one()
// ============================================================================
// These prove that zero/one behave correctly in ACTUAL ARITHMETIC at extreme scales

#[test]
fn zero_identity_addition_extreme() {
    // Create a value that NEEDS extreme precision
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let z = BigFloat::zero(7000);
    let result = a.add(&z);

    // Result should equal original (identity property)
    assert_eq!(result, a);
}

#[test]
fn one_identity_multiplication_extreme() {
    // Create a value that NEEDS extreme precision
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let o = BigFloat::one(7000);
    let result = a.mul(&o);

    // Result should equal original (identity property)
    assert_eq!(result, a);
}

// ============================================================================
// from_string() tests - THE REAL EXTREME VALUE TESTS
// ============================================================================

#[test]
fn from_string_scientific_notation_extreme_tiny() {
    let bf = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    // Value is beyond f64 range
    assert_eq!(bf.to_f64(), 0.0);
}

#[test]
fn from_string_scientific_notation_extreme_large() {
    let bf = BigFloat::from_string("3.5e2000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    // Value is beyond f64 range
    assert_eq!(bf.to_f64(), f64::INFINITY);
}

#[test]
fn from_string_with_mantissa_extreme() {
    let bf = BigFloat::from_string("1.23456789e-1000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
}

#[test]
fn from_string_f64_path_normal_value() {
    let bf = BigFloat::from_string("1.5", 64).unwrap();
    assert_eq!(bf.precision_bits(), 64);
    assert_eq!(bf.to_f64(), 1.5);
}

#[test]
fn from_string_fbig_path_high_precision() {
    let bf = BigFloat::from_string("1.5", 128).unwrap();
    assert_eq!(bf.precision_bits(), 128);
    assert_eq!(bf.to_f64(), 1.5);
}

#[test]
fn from_string_extreme_tiny_5000() {
    let bf = BigFloat::from_string("1e-5000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    assert_eq!(bf.to_f64(), 0.0);
}

#[test]
fn from_string_extreme_large_5000() {
    let bf = BigFloat::from_string("1e5000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    assert_eq!(bf.to_f64(), f64::INFINITY);
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
// CRITICAL TEST: Prove arithmetic correctness at extreme scales
// ============================================================================
// This is what we've been talking about - exact string-based comparison

#[test]
fn from_string_arithmetic_correctness_extreme() {
    // Parse two extreme values
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("3.5e-2000", 7000).unwrap();

    // Add them
    let result = a.add(&b);

    // Parse expected result
    let expected = BigFloat::from_string("4.5e-2000", 7000).unwrap();

    // EXACT comparison - no tolerance
    assert_eq!(result, expected);
}
