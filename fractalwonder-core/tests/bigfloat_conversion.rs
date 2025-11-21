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
// to_f64() within f64 range tests
// ============================================================================

#[test]
fn to_f64_f64_path_exact() {
    let bf = BigFloat::with_precision(1.5, 64);
    assert_eq!(bf.to_f64(), 1.5);
}

#[test]
fn to_f64_fbig_path_within_range() {
    let bf = BigFloat::with_precision(2.5, 128);
    assert_eq!(bf.to_f64(), 2.5);
}

#[test]
fn to_f64_round_trip() {
    let original = 7.5;
    let bf = BigFloat::with_precision(original, 64);
    assert_eq!(bf.to_f64(), original);
}

// ============================================================================
// to_f64() beyond f64 range tests
// ============================================================================

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
// to_f64() at f64 boundaries tests
// ============================================================================

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
