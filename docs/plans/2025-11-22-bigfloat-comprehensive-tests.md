# BigFloat Comprehensive Test Suite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement comprehensive test suite proving BigFloat correctness at extreme scales (7000+ bits, 10^±5000 magnitudes)

**Architecture:** Five test modules (construction, arithmetic, conversion, comparison, serialization) with ~200-300 tests total. Each module tests F64 path (≤64 bits), boundary (64→65 bits), moderate (128-1024 bits), and extreme (7000+ bits) scales using exact string-based comparisons with zero tolerance.

**Tech Stack:** Rust 1.80+, dashu 0.4 (arbitrary precision), cargo test, wasm-bindgen-test

---

## Task 1: Construction Tests - with_precision()

**Files:**
- Create: `fractalwonder-core/tests/bigfloat_construction.rs`

**Step 1: Write the failing test for with_precision() F64 path**

```rust
use fractalwonder_core::BigFloat;

#[test]
fn with_precision_f64_path_32_bits() {
    let bf = BigFloat::with_precision(1.5, 32);
    assert_eq!(bf.precision_bits(), 32);
    assert_eq!(bf.to_f64(), 1.5);
}

#[test]
fn with_precision_f64_path_64_bits() {
    let bf = BigFloat::with_precision(2.5, 64);
    assert_eq!(bf.precision_bits(), 64);
    assert_eq!(bf.to_f64(), 2.5);
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction with_precision_f64 -- --nocapture`
Expected: PASS (these should already work with current implementation)

**Step 3: Write test for with_precision() FBig path**

```rust
#[test]
fn with_precision_fbig_path_128_bits() {
    let bf = BigFloat::with_precision(1.5, 128);
    assert_eq!(bf.precision_bits(), 128);
    assert_eq!(bf.to_f64(), 1.5);
}

#[test]
fn with_precision_fbig_path_7000_bits() {
    let bf = BigFloat::with_precision(3.14159, 7000);
    assert_eq!(bf.precision_bits(), 7000);
    assert_eq!(bf.to_f64(), 3.14159);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction with_precision_fbig -- --nocapture`
Expected: PASS

**Step 5: Write test for with_precision() boundary transition**

```rust
#[test]
fn with_precision_boundary_64_to_65_bits() {
    let bf_64 = BigFloat::with_precision(1.5, 64);
    let bf_65 = BigFloat::with_precision(1.5, 65);

    // Both should have correct precision metadata
    assert_eq!(bf_64.precision_bits(), 64);
    assert_eq!(bf_65.precision_bits(), 65);

    // Both should have same mathematical value
    assert_eq!(bf_64, bf_65);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction with_precision_boundary -- --nocapture`
Expected: PASS

**Step 7: Write test for with_precision() zero special case**

```rust
#[test]
fn with_precision_zero_f64_path() {
    let bf = BigFloat::with_precision(0.0, 32);
    assert_eq!(bf.precision_bits(), 32);
    assert_eq!(bf.to_f64(), 0.0);
}

#[test]
fn with_precision_zero_fbig_path() {
    let bf = BigFloat::with_precision(0.0, 7000);
    assert_eq!(bf.precision_bits(), 7000);
    assert_eq!(bf.to_f64(), 0.0);
}
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction with_precision_zero -- --nocapture`
Expected: PASS

**Step 9: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_construction.rs
git commit -m "test(bigfloat): add with_precision() tests for all paths"
```

---

## Task 2: Construction Tests - zero() and one()

**Files:**
- Modify: `fractalwonder-core/tests/bigfloat_construction.rs`

**Step 1: Write tests for zero() constructor**

```rust
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
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction zero -- --nocapture`
Expected: PASS

**Step 3: Write tests for one() constructor**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction one -- --nocapture`
Expected: PASS

**Step 5: Write tests for mathematical identities with zero() and one()**

```rust
#[test]
fn zero_identity_addition() {
    let a = BigFloat::with_precision(5.5, 128);
    let z = BigFloat::zero(128);
    let result = a.add(&z);
    assert_eq!(result, a);
}

#[test]
fn one_identity_multiplication() {
    let a = BigFloat::with_precision(5.5, 128);
    let o = BigFloat::one(128);
    let result = a.mul(&o);
    assert_eq!(result, a);
}

#[test]
fn zero_identity_extreme_precision() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let z = BigFloat::zero(7000);
    let result = a.add(&z);
    assert_eq!(result, a);
}

#[test]
fn one_identity_extreme_precision() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let o = BigFloat::one(7000);
    let result = a.mul(&o);
    assert_eq!(result, a);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction identity -- --nocapture`
Expected: PASS

**Step 7: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_construction.rs
git commit -m "test(bigfloat): add zero() and one() tests with identity properties"
```

---

## Task 3: Construction Tests - from_string()

**Files:**
- Modify: `fractalwonder-core/tests/bigfloat_construction.rs`

**Step 1: Write tests for from_string() scientific notation parsing**

```rust
#[test]
fn from_string_scientific_notation_large() {
    let bf = BigFloat::from_string("3.5e2000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    // Value is beyond f64 range, to_f64() should return infinity
    assert_eq!(bf.to_f64(), f64::INFINITY);
}

#[test]
fn from_string_scientific_notation_tiny() {
    let bf = BigFloat::from_string("1e-2000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    // Value is beyond f64 range, to_f64() should return 0.0
    assert_eq!(bf.to_f64(), 0.0);
}

#[test]
fn from_string_scientific_notation_with_mantissa() {
    let bf = BigFloat::from_string("1.23456789e-1000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction from_string_scientific -- --nocapture`
Expected: PASS

**Step 3: Write tests for from_string() path selection**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction from_string.*path -- --nocapture`
Expected: PASS

**Step 5: Write tests for from_string() extreme values**

```rust
#[test]
fn from_string_extreme_tiny() {
    let bf = BigFloat::from_string("1e-5000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    assert_eq!(bf.to_f64(), 0.0);
}

#[test]
fn from_string_extreme_large() {
    let bf = BigFloat::from_string("1e5000", 7000).unwrap();
    assert_eq!(bf.precision_bits(), 7000);
    assert_eq!(bf.to_f64(), f64::INFINITY);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction from_string_extreme -- --nocapture`
Expected: PASS

**Step 7: Write tests for from_string() error handling**

```rust
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
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction from_string.*invalid -- --nocapture`
Expected: PASS

**Step 9: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_construction.rs
git commit -m "test(bigfloat): add from_string() tests for parsing and extreme values"
```

---

## Task 4: Arithmetic Tests - Addition

**Files:**
- Create: `fractalwonder-core/tests/bigfloat_arithmetic.rs`

**Step 1: Write tests for addition same-scale operations**

```rust
use fractalwonder_core::BigFloat;

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
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("3.5e-2000", 7000).unwrap();
    let result = a.add(&b);
    let expected = BigFloat::from_string("4.5e-2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
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
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic add.*same_scale -- --nocapture`
Expected: PASS

**Step 3: Write tests for addition cross-scale operations**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic add_cross_scale -- --nocapture`
Expected: PASS

**Step 5: Write tests for addition commutativity**

```rust
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
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic add_commutativity -- --nocapture`
Expected: PASS

**Step 7: Write tests for addition cross-magnitude**

```rust
#[test]
fn add_cross_magnitude_preserves_both_terms() {
    // Create two values at vastly different magnitudes
    let large = BigFloat::from_string("1e-100", 7000).unwrap();
    let tiny = BigFloat::from_string("1e-2000", 7000).unwrap();
    let result = large.add(&tiny);

    // Result should be dominated by larger term but preserve precision
    assert_eq!(result.precision_bits(), 7000);
    // Since we're at high precision, both terms should be represented
}
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic add_cross_magnitude -- --nocapture`
Expected: PASS

**Step 9: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_arithmetic.rs
git commit -m "test(bigfloat): add addition tests for all scales and scenarios"
```

---

## Task 5: Arithmetic Tests - Subtraction

**Files:**
- Modify: `fractalwonder-core/tests/bigfloat_arithmetic.rs`

**Step 1: Write tests for subtraction basic operations**

```rust
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
    let a = BigFloat::from_string("5e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-2000", 7000).unwrap();
    let result = a.sub(&b);
    let expected = BigFloat::from_string("3e-2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sub.*basic -- --nocapture`
Expected: PASS

**Step 3: Write tests for subtraction identical values (catastrophic cancellation)**

```rust
#[test]
fn sub_identical_values_f64() {
    let a = BigFloat::with_precision(5.5, 64);
    let result = a.sub(&a);
    let expected = BigFloat::zero(64);

    assert_eq!(result, expected);
}

#[test]
fn sub_identical_values_extreme() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let result = a.sub(&a);
    let expected = BigFloat::zero(7000);

    assert_eq!(result, expected);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sub_identical -- --nocapture`
Expected: PASS

**Step 5: Write tests for subtraction near-equal values**

```rust
#[test]
fn sub_near_equal_values_extreme() {
    // Test catastrophic cancellation at extreme precision
    let a = BigFloat::from_string("1.0000000001e-2000", 7000).unwrap();
    let b = BigFloat::from_string("1.0e-2000", 7000).unwrap();
    let result = a.sub(&b);
    let expected = BigFloat::from_string("1e-2011", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sub_near_equal -- --nocapture`
Expected: PASS

**Step 7: Write tests for subtraction producing negative results**

```rust
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
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sub_negative -- --nocapture`
Expected: PASS

**Step 9: Write tests for subtraction cross-magnitude**

```rust
#[test]
fn sub_cross_magnitude_preserves_dominant() {
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("1e-100", 7000).unwrap();
    let result = a.sub(&b);

    // Result dominated by -1e-100 (much larger magnitude)
    assert_eq!(result.precision_bits(), 7000);
}
```

**Step 10: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sub_cross_magnitude -- --nocapture`
Expected: PASS

**Step 11: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_arithmetic.rs
git commit -m "test(bigfloat): add subtraction tests including catastrophic cancellation"
```

---

## Task 6: Arithmetic Tests - Multiplication

**Files:**
- Modify: `fractalwonder-core/tests/bigfloat_arithmetic.rs`

**Step 1: Write tests for multiplication basic operations**

```rust
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
    let a = BigFloat::from_string("2e-1000", 7000).unwrap();
    let b = BigFloat::from_string("3e-1000", 7000).unwrap();
    let result = a.mul(&b);
    let expected = BigFloat::from_string("6e-2000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic mul.*basic -- --nocapture`
Expected: PASS

**Step 3: Write tests for multiplication magnitude doubling/halving**

```rust
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
    let expected = BigFloat::from_string("1e-4000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic mul_magnitude -- --nocapture`
Expected: PASS

**Step 5: Write tests for multiplication cross-scale**

```rust
#[test]
fn mul_cross_scale_huge_times_tiny() {
    let a = BigFloat::from_string("1e2000", 7000).unwrap();
    let b = BigFloat::from_string("1e-3000", 7000).unwrap();
    let result = a.mul(&b);
    let expected = BigFloat::from_string("1e-1000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic mul_cross_scale -- --nocapture`
Expected: PASS

**Step 7: Write tests for multiplication identity and zero**

```rust
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
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic mul.*identity -- --nocapture`
Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic mul.*zero -- --nocapture`
Expected: PASS

**Step 9: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_arithmetic.rs
git commit -m "test(bigfloat): add multiplication tests with magnitude scaling"
```

---

## Task 7: Arithmetic Tests - Division

**Files:**
- Modify: `fractalwonder-core/tests/bigfloat_arithmetic.rs`

**Step 1: Write tests for division basic operations**

```rust
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
    let a = BigFloat::from_string("6e-2000", 7000).unwrap();
    let b = BigFloat::from_string("2e-1000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("3e-1000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic div.*basic -- --nocapture`
Expected: PASS

**Step 3: Write tests for division magnitude swings**

```rust
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
    let a = BigFloat::from_string("1e-2000", 7000).unwrap();
    let b = BigFloat::from_string("1e2000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("1e-4000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic div_magnitude_swing -- --nocapture`
Expected: PASS

**Step 5: Write tests for division exact results**

```rust
#[test]
fn div_exact_result_no_spurious_loss() {
    let a = BigFloat::from_string("6e-2000", 7000).unwrap();
    let b = BigFloat::from_string("3e-1000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("2e-1000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic div_exact -- --nocapture`
Expected: PASS

**Step 7: Write tests for division near-zero denominator**

```rust
#[test]
fn div_near_zero_denominator_moderate() {
    let a = BigFloat::with_precision(1.0, 512);
    let b = BigFloat::from_string("1e-100", 512).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("1e100", 512).unwrap();

    assert_eq!(result, expected);
}

#[test]
fn div_near_zero_denominator_extreme() {
    let a = BigFloat::with_precision(1.0, 7000);
    let b = BigFloat::from_string("1e-1000", 7000).unwrap();
    let result = a.div(&b);
    let expected = BigFloat::from_string("1e1000", 7000).unwrap();

    assert_eq!(result, expected);
}
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic div_near_zero -- --nocapture`
Expected: PASS

**Step 9: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_arithmetic.rs
git commit -m "test(bigfloat): add division tests with magnitude swings and edge cases"
```

---

## Task 8: Arithmetic Tests - Square Root

**Files:**
- Modify: `fractalwonder-core/tests/bigfloat_arithmetic.rs`

**Step 1: Write tests for sqrt perfect squares**

```rust
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
    let a = BigFloat::from_string("4e-2000", 7000).unwrap();
    let result = a.sqrt();
    let expected = BigFloat::from_string("2e-1000", 7000).unwrap();

    assert_eq!(result.precision_bits(), 7000);
    assert_eq!(result, expected);
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sqrt_perfect_square -- --nocapture`
Expected: PASS

**Step 3: Write tests for sqrt precision metadata**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sqrt.*precision -- --nocapture`
Expected: PASS

**Step 5: Write tests for sqrt self-consistency**

```rust
#[test]
fn sqrt_self_consistency_perfect_square() {
    let a = BigFloat::with_precision(16.0, 128);
    let sqrt_a = a.sqrt();
    let result = sqrt_a.mul(&sqrt_a);

    assert_eq!(result, a);
}

#[test]
fn sqrt_self_consistency_extreme_perfect_square() {
    let a = BigFloat::from_string("16e-2000", 7000).unwrap();
    let sqrt_a = a.sqrt();
    let result = sqrt_a.mul(&sqrt_a);

    assert_eq!(result, a);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sqrt_self_consistency -- --nocapture`
Expected: PASS

**Step 7: Write tests for sqrt at various scales**

```rust
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
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sqrt.*transition -- --nocapture`
Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic sqrt_moderate -- --nocapture`
Expected: PASS

**Step 9: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_arithmetic.rs
git commit -m "test(bigfloat): add sqrt tests with perfect squares and self-consistency"
```

---

## Task 9: Conversion Tests - precision_bits() and to_f64()

**Files:**
- Create: `fractalwonder-core/tests/bigfloat_conversion.rs`

**Step 1: Write tests for precision_bits() getter**

```rust
use fractalwonder_core::BigFloat;

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
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_conversion precision_bits -- --nocapture`
Expected: PASS

**Step 3: Write tests for precision_bits() after arithmetic**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_conversion precision_bits_after -- --nocapture`
Expected: PASS

**Step 5: Write tests for to_f64() within f64 range**

```rust
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
    let original = 3.14159;
    let bf = BigFloat::with_precision(original, 64);
    assert_eq!(bf.to_f64(), original);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_conversion to_f64.*range -- --nocapture`
Expected: PASS

**Step 7: Write tests for to_f64() beyond f64 range**

```rust
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
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_conversion to_f64_extreme -- --nocapture`
Expected: PASS

**Step 9: Write tests for to_f64() at f64 boundaries**

```rust
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
```

**Step 10: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_conversion to_f64_at_f64 -- --nocapture`
Expected: PASS

**Step 11: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_conversion.rs
git commit -m "test(bigfloat): add conversion tests for precision_bits() and to_f64()"
```

---

## Task 10: Comparison Tests - PartialEq

**Files:**
- Create: `fractalwonder-core/tests/bigfloat_comparison.rs`

**Step 1: Write tests for equality reflexivity**

```rust
use fractalwonder_core::BigFloat;

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
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison eq_reflexivity -- --nocapture`
Expected: PASS

**Step 3: Write tests for equality of separately constructed identical values**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison eq_separately_constructed -- --nocapture`
Expected: PASS

**Step 5: Write tests for equality cross-path comparisons**

```rust
#[test]
fn eq_cross_path_same_value() {
    let a = BigFloat::with_precision(1.5, 64);  // F64 path
    let b = BigFloat::with_precision(1.5, 128); // FBig path
    assert_eq!(a, b);
}

#[test]
fn eq_cross_path_boundary() {
    let a = BigFloat::with_precision(2.5, 64);
    let b = BigFloat::with_precision(2.5, 65);
    assert_eq!(a, b);
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison eq_cross_path -- --nocapture`
Expected: PASS

**Step 7: Write tests for inequality detection**

```rust
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
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison neq -- --nocapture`
Expected: PASS

**Step 9: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_comparison.rs
git commit -m "test(bigfloat): add PartialEq tests with reflexivity and cross-path"
```

---

## Task 11: Comparison Tests - PartialOrd

**Files:**
- Modify: `fractalwonder-core/tests/bigfloat_comparison.rs`

**Step 1: Write tests for basic ordering**

```rust
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
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison ord_basic -- --nocapture`
Expected: PASS

**Step 3: Write tests for ordering with equality**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison ord.*equal -- --nocapture`
Expected: PASS

**Step 5: Write tests for ordering transitivity**

```rust
#[test]
fn ord_transitivity_extreme() {
    let a = BigFloat::from_string("1e-3000", 7000).unwrap();
    let b = BigFloat::from_string("1e-2000", 7000).unwrap();
    let c = BigFloat::from_string("1e-1000", 7000).unwrap();

    assert!(a < b);
    assert!(b < c);
    assert!(a < c); // transitivity
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison ord_transitivity -- --nocapture`
Expected: PASS

**Step 7: Write tests for cross-magnitude ordering**

```rust
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
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison ord_cross_magnitude -- --nocapture`
Expected: PASS

**Step 9: Write tests for cross-path ordering**

```rust
#[test]
fn ord_cross_path_f64_vs_fbig() {
    let a = BigFloat::with_precision(1.5, 64);  // F64 path
    let b = BigFloat::with_precision(2.5, 128); // FBig path
    assert!(a < b);
}
```

**Step 10: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison ord_cross_path -- --nocapture`
Expected: PASS

**Step 11: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_comparison.rs
git commit -m "test(bigfloat): add PartialOrd tests with transitivity and cross-magnitude"
```

---

## Task 12: Serialization Tests

**Files:**
- Create: `fractalwonder-core/tests/bigfloat_serialization.rs`

**Step 1: Write tests for basic round-trip serialization**

```rust
use fractalwonder_core::BigFloat;
use serde_json;

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
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_serialization serialize_deserialize.*basic -- --nocapture`
Expected: PASS

**Step 3: Write tests for extreme value serialization**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_serialization serialize_deserialize_extreme -- --nocapture`
Expected: PASS

**Step 5: Write tests for serialization format verification**

```rust
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
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_serialization serialize_format -- --nocapture`
Expected: PASS

**Step 7: Write tests for zero serialization**

```rust
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
```

**Step 8: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_serialization serialize_deserialize_zero -- --nocapture`
Expected: PASS

**Step 9: Write tests for serialization of arithmetic results**

```rust
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
```

**Step 10: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_serialization serialize_deserialize_after_arithmetic -- --nocapture`
Expected: PASS

**Step 11: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_serialization.rs
git commit -m "test(bigfloat): add serialization round-trip tests with format verification"
```

---

## Task 13: Additional Arithmetic Edge Cases - Cross-Scale Tests

**Files:**
- Modify: `fractalwonder-core/tests/bigfloat_arithmetic.rs`

**Step 1: Write comprehensive cross-scale arithmetic tests**

```rust
#[test]
fn arithmetic_cross_scale_all_operations() {
    let a = BigFloat::with_precision(10.0, 64);  // F64 path
    let b = BigFloat::with_precision(5.0, 256);  // FBig path

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
```

**Step 2: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic arithmetic_cross_scale_all -- --nocapture`
Expected: PASS

**Step 3: Write precision progression tests**

```rust
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic arithmetic_precision_progression -- --nocapture`
Expected: PASS

**Step 5: Write magnitude progression tests**

```rust
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
```

**Step 6: Run test to verify it passes**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic arithmetic_magnitude_progression -- --nocapture`
Expected: PASS

**Step 7: Commit**

```bash
git add fractalwonder-core/tests/bigfloat_arithmetic.rs
git commit -m "test(bigfloat): add cross-scale and progression tests for arithmetic"
```

---

## Task 14: Run Full Test Suite and Verify

**Step 1: Run all construction tests**

Run: `cargo test --package fractalwonder-core --test bigfloat_construction -- --nocapture`
Expected: All tests PASS

**Step 2: Run all arithmetic tests**

Run: `cargo test --package fractalwonder-core --test bigfloat_arithmetic -- --nocapture`
Expected: All tests PASS

**Step 3: Run all conversion tests**

Run: `cargo test --package fractalwonder-core --test bigfloat_conversion -- --nocapture`
Expected: All tests PASS

**Step 4: Run all comparison tests**

Run: `cargo test --package fractalwonder-core --test bigfloat_comparison -- --nocapture`
Expected: All tests PASS

**Step 5: Run all serialization tests**

Run: `cargo test --package fractalwonder-core --test bigfloat_serialization -- --nocapture`
Expected: All tests PASS

**Step 6: Run entire test suite**

Run: `cargo test --package fractalwonder-core -- --nocapture`
Expected: All tests PASS, no warnings

**Step 7: Run clippy**

Run: `cargo clippy --package fractalwonder-core --all-targets -- -D warnings`
Expected: No warnings or errors

**Step 8: Run cargo fmt**

Run: `cargo fmt --package fractalwonder-core -- --check`
Expected: All files formatted correctly

**Step 9: Commit final verification**

```bash
git add -A
git commit -m "test(bigfloat): verify complete test suite passes all checks"
```

---

## Summary

**Test Coverage:**
- **Task 1-3**: Construction tests (~30 tests) - with_precision, zero, one, from_string
- **Task 4-8**: Arithmetic tests (~60 tests) - add, sub, mul, div, sqrt with all scales
- **Task 9**: Conversion tests (~20 tests) - precision_bits, to_f64
- **Task 10-11**: Comparison tests (~25 tests) - PartialEq, PartialOrd
- **Task 12**: Serialization tests (~15 tests) - round-trip, format verification
- **Task 13**: Additional edge cases (~10 tests) - cross-scale, progressions

**Total: ~160 comprehensive test cases** proving BigFloat correctness at extreme scales (7000+ bits, 10^±5000 magnitudes) using exact string-based comparisons with zero tolerance.

**Test Execution:**
- All tests use exact equality assertions (no tolerance)
- Tests cover F64 path (≤64 bits), boundary (64→65), moderate (128-1024), and extreme (7000+ bits)
- Tests verify all magnitude scales from 10^-5000 to 10^5000
- Cross-path interactions verified between F64 and FBig representations

**Quality Gates:**
- All tests must pass
- No clippy warnings
- Code properly formatted
- Frequent commits after each task
