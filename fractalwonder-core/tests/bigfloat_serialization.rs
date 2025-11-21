use fractalwonder_core::BigFloat;

// ============================================================================
// Task 12: Serialization Tests
// ============================================================================

// ============================================================================
// Basic round-trip serialization tests
// ============================================================================

#[test]
fn serialize_deserialize_f64_path_basic() {
    let original = BigFloat::with_precision(1.5, 64);

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: BigFloat = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, original);
    assert_eq!(deserialized.precision_bits(), 64);
}

#[test]
fn serialize_deserialize_fbig_path_basic() {
    let original = BigFloat::with_precision(2.5, 128);

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: BigFloat = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, original);
    assert_eq!(deserialized.precision_bits(), 128);
}

// ============================================================================
// Extreme value serialization tests
// ============================================================================

#[test]
fn serialize_deserialize_extreme_tiny() {
    let original = BigFloat::from_string("1e-5000", 7000).unwrap();

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: BigFloat = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, original);
    assert_eq!(deserialized.precision_bits(), 7000);
}

#[test]
fn serialize_deserialize_extreme_large() {
    let original = BigFloat::from_string("1e5000", 7000).unwrap();

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: BigFloat = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, original);
    assert_eq!(deserialized.precision_bits(), 7000);
}

// ============================================================================
// Serialization format verification tests
// ============================================================================

#[test]
fn serialize_format_contains_required_fields() {
    let bf = BigFloat::with_precision(1.5, 128);
    let serialized = serde_json::to_string(&bf).unwrap();

    // Verify JSON contains required fields
    assert!(serialized.contains("value"));
    assert!(serialized.contains("precision_bits"));
    assert!(serialized.contains("128"));
}

#[test]
fn serialize_format_extreme_readable() {
    let bf = BigFloat::from_string("1e-2000", 7000).unwrap();
    let serialized = serde_json::to_string(&bf).unwrap();

    // Verify format is human-readable
    assert!(serialized.contains("value"));
    assert!(serialized.contains("precision_bits"));
    assert!(serialized.contains("7000"));
}

// ============================================================================
// Zero serialization tests
// ============================================================================

#[test]
fn serialize_deserialize_zero_f64() {
    let original = BigFloat::zero(64);

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: BigFloat = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, original);
    assert_eq!(deserialized.precision_bits(), 64);
}

#[test]
fn serialize_deserialize_zero_extreme() {
    let original = BigFloat::zero(7000);

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: BigFloat = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, original);
    assert_eq!(deserialized.precision_bits(), 7000);
}

// ============================================================================
// Serialization of arithmetic results tests
// ============================================================================

#[test]
fn serialize_deserialize_after_arithmetic() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-2000", 7000).unwrap();
    let original = a.add(&b);

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: BigFloat = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, original);
    assert_eq!(deserialized.precision_bits(), 7000);

    // Can still use in arithmetic
    let c = BigFloat::from_string("1e-2000", 7000).unwrap();
    let result = deserialized.mul(&c);
    assert_eq!(result.precision_bits(), 7000);
}
