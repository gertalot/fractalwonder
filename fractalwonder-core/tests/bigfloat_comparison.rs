use fractalwonder_core::BigFloat;

// ============================================================================
// Task 10: Comparison Tests - PartialEq
// ============================================================================

// ============================================================================
// Equality reflexivity tests
// ============================================================================

#[test]
fn eq_reflexivity_f64_path() {
    let a = BigFloat::with_precision(1.5, 64);
    assert_eq!(a, a);
}

#[test]
fn eq_reflexivity_fbig_path() {
    let a = BigFloat::with_precision(1.5, 128);
    assert_eq!(a, a);
}

#[test]
fn eq_reflexivity_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert_eq!(a, a);
}

// ============================================================================
// Equality of separately constructed identical values
// ============================================================================

#[test]
fn eq_separately_constructed_f64_path() {
    let a = BigFloat::with_precision(1.5, 64);
    let b = BigFloat::with_precision(1.5, 64);
    assert_eq!(a, b);
}

#[test]
fn eq_separately_constructed_from_string() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert_eq!(a, b);
}

// ============================================================================
// Equality cross-path comparisons
// ============================================================================

#[test]
fn eq_cross_path_same_value() {
    let a = BigFloat::with_precision(1.5, 64); // F64 path
    let b = BigFloat::with_precision(1.5, 128); // FBig path
    assert_eq!(a, b);
}

#[test]
fn eq_cross_path_boundary() {
    let a = BigFloat::with_precision(2.5, 64);
    let b = BigFloat::with_precision(2.5, 65);
    assert_eq!(a, b);
}

// ============================================================================
// Inequality detection tests
// ============================================================================

#[test]
fn neq_different_values_f64() {
    let a = BigFloat::with_precision(1.5, 64);
    let b = BigFloat::with_precision(2.5, 64);
    assert_ne!(a, b);
}

#[test]
fn neq_different_values_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-2000", 7000).unwrap();
    assert_ne!(a, b);
}

#[test]
fn neq_ulp_level_difference_extreme() {
    // At extreme precision, detect ULP-level differences
    let a = BigFloat::from_string("1.0e-2000", 7000).unwrap();
    let b = BigFloat::from_string("1.0000000000000001e-2000", 7000).unwrap();
    assert_ne!(a, b);
}

// ============================================================================
// Task 11: Comparison Tests - PartialOrd
// ============================================================================

// ============================================================================
// Basic ordering tests
// ============================================================================

#[test]
fn ord_basic_less_than_f64() {
    let a = BigFloat::with_precision(1.0, 64);
    let b = BigFloat::with_precision(2.0, 64);
    assert!(a < b);
}

#[test]
fn ord_basic_greater_than_f64() {
    let a = BigFloat::with_precision(2.0, 64);
    let b = BigFloat::with_precision(1.0, 64);
    assert!(a > b);
}

#[test]
fn ord_basic_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-2000", 7000).unwrap();
    assert!(a < b);
    assert!(b > a);
}

// ============================================================================
// Ordering with equality tests
// ============================================================================

#[test]
fn ord_less_than_or_equal() {
    let a = BigFloat::with_precision(1.0, 64);
    let b = BigFloat::with_precision(1.0, 64);
    assert!(a <= b);
}

#[test]
fn ord_greater_than_or_equal() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert!(a >= b);
}

// ============================================================================
// Ordering transitivity tests
// ============================================================================

#[test]
fn ord_transitivity_extreme() {
    let a = BigFloat::from_string("1e-3000", 7000).unwrap();
    let b = BigFloat::from_string("1e-2000", 7000).unwrap();
    let c = BigFloat::from_string("1e-1000", 7000).unwrap();

    assert!(a < b);
    assert!(b < c);
    assert!(a < c); // transitivity
}

// ============================================================================
// Cross-magnitude ordering tests
// ============================================================================

#[test]
fn ord_cross_magnitude_vast_differences() {
    let tiny = BigFloat::from_string("1e-5000", 7000).unwrap();
    let small = BigFloat::from_string("1e-2000", 7000).unwrap();
    let medium = BigFloat::from_string("1e-100", 7000).unwrap();
    let one = BigFloat::with_precision(1.0, 7000);

    assert!(tiny < small);
    assert!(small < medium);
    assert!(medium < one);
}

// ============================================================================
// Cross-path ordering tests
// ============================================================================

#[test]
fn ord_cross_path_f64_vs_fbig() {
    let a = BigFloat::with_precision(1.5, 64); // F64 path
    let b = BigFloat::with_precision(2.5, 128); // FBig path
    assert!(a < b);
}
