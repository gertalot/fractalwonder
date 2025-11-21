# BigFloat Comprehensive Test Suite Design

## Overview

This document specifies a comprehensive test suite for the BigFloat arbitrary-precision floating-point implementation. The suite proves correctness of all BigFloat operations at extreme scales (up to 7000+ bits precision, handling values like 10^±5000) using exact comparisons with zero tolerance.

## Goals

1. **Prove arithmetic correctness**: Verify add, sub, mul, div, sqrt produce mathematically correct results at extreme scales
2. **Verify precision handling**: Confirm precision metadata and computational behavior match specifications
3. **Test all code paths**: Cover F64 path (≤64 bits), FBig path (>64 bits), and cross-path interactions
4. **Validate all functions**: Test every public function including constructors, conversions, comparisons, and serialization

## Scale Strategy

### Precision Bit Progression
- **F64 path**: 32, 64 bits (using f64 internally)
- **Boundary**: 64→65 bit transition (F64→FBig upgrade)
- **Moderate**: 128, 256, 512, 1024 bits
- **Extreme**: 2048, 4096, 7000 bits

### Magnitude Progression
- **Normal**: 10^-10 to 10^10
- **Large**: 10^100, 10^1000, 10^2000
- **Tiny**: 10^-100, 10^-1000, 10^-2000, 10^-5000

## Test Organization

Five test modules, each containing ~40-60 test cases:

```
fractalwonder-core/tests/
├── bigfloat_construction.rs    # with_precision, zero, one, from_string
├── bigfloat_arithmetic.rs      # add, sub, mul, div, sqrt
├── bigfloat_conversion.rs      # to_f64, precision_bits
├── bigfloat_comparison.rs      # eq, partial_cmp
└── bigfloat_serialization.rs   # serialize, deserialize
```

## Verification Strategy

### Exact Comparisons (Primary)
```rust
let a = BigFloat::from_string("1e-2000", 7000).unwrap();
let b = BigFloat::from_string("3.5e-2000", 7000).unwrap();
let result = a.add(&b);
let expected = BigFloat::from_string("4.5e-2000", 7000).unwrap();
assert_eq!(result, expected);  // Zero tolerance
```

### Dashu Ground Truth (Complex Calculations)
```rust
use dashu::fbig;
let a = BigFloat::from_string("1e-2000", 7000).unwrap();
let b = BigFloat::from_string("7e-2001", 7000).unwrap();
let result = a.add(&b);
let expected_fbig = fbig!(1e-2000) + fbig!(7e-2001);
assert_eq!(result.to_fbig(), expected_fbig);
```

**No tolerance-based comparisons.** All assertions use exact equality.

## Test Specifications by Module

### 1. Construction Tests (bigfloat_construction.rs)

#### with_precision()
- **Path selection**: 32 bits → F64, 65 bits → FBig, 7000 bits → FBig
- **Precision metadata**: `precision_bits()` returns what was specified
- **Value preservation**: `with_precision(1.5, 64) == with_precision(1.5, 128)` mathematically
- **Zero special case**: `with_precision(0.0, N)` works for all N including 7000

#### zero() and one()
- **Both paths**: Test at 32, 128, 7000 bits
- **Precision metadata**: Correct for all scales
- **Mathematical identity**: `a + zero(N) == a`, `a * one(N) == a` at all scales

#### from_string()

*Parsing correctness:*
- Scientific notation: "1e-2000", "3.5e2000", "1.23456789e-1000"
- Path selection: parseable as f64 with ≤64 bits → F64, else → FBig
- Error handling: Invalid strings return meaningful Err

*Precision enforcement:*
- "1e-2000" parsed at 7000 bits creates 7000-bit value
- Verify via arithmetic that operations preserve requested precision

*Extreme values:*
- 10^-5000, 10^5000 (beyond f64 range)
- Verify these parse and are usable in arithmetic

### 2. Arithmetic Tests (bigfloat_arithmetic.rs)

Organized by scale (F64/boundary/moderate/extreme), covering same-scale, cross-scale, and edge cases.

#### Addition
- **Same-scale**: `1e-2000 + 3.5e-2000 = 4.5e-2000` at 7000 bits
- **Cross-scale operands**: `BigFloat(2.0, 64) + BigFloat(3.0, 256)` → 256 bits, value = 5.0
- **Cross-magnitude**: `1e-2000 + 1e-100` preserves both terms
- **Precision progression**: Test at 64, 128, 512, 2048, 7000 bits
- **Commutativity**: `a + b == b + a`

#### Subtraction
- **Catastrophic cancellation**: `1.0000000001e-2000 - 1.0e-2000` produces correct tiny difference
- **Identical values**: `a - a == zero(N)` at all precisions
- **Sign changes**: Negative results (e.g., `1e-2000 - 2e-2000`)
- **Cross-magnitude**: `1e-2000 - 1e-100` preserves dominant term
- **All scales**: 64, 128, 512, 2048, 7000 bits

#### Multiplication
- **Magnitude doubling**: `1e2000 * 1e2000 = 1e4000`
- **Going smaller**: `1e-2000 * 1e-2000 = 1e-4000`
- **Cross-scale**: `1e2000 * 1e-3000 = 1e-1000`
- **Identity**: `a * one(N) == a` at all scales
- **Zero**: `a * zero(N) == zero(max(a.prec, N))`

#### Division
- **Magnitude swings**: `1.0 / 1e-2000 = 1e2000`
- **Going extreme**: `1e-2000 / 1e2000 = 1e-4000`
- **Exact results**: `6e-2000 / 3e-1000 = 2e-1000` (no spurious loss)
- **Near-zero denominator**: `1.0 / 1e-100` at increasing precisions
- **All scales**: 64, 128, 512, 2048, 7000 bits

#### sqrt()
- **Perfect squares**: `sqrt(4e-2000) == 2e-1000` exactly
- **Precision metadata**: `sqrt(a).precision_bits() == a.precision_bits()`
- **Self-consistency**: `sqrt(x) * sqrt(x) == x` for perfect squares
- **All scales**: 64, 128, 512, 2048, 7000 bits

### 3. Conversion Tests (bigfloat_conversion.rs)

#### precision_bits()
- **Returns correct value** for all construction methods:
  - `with_precision(1.5, N).precision_bits() == N`
  - `from_string("1e-2000", N).precision_bits() == N`
  - `zero(N).precision_bits() == N`
- **After arithmetic**:
  - `(a + b).precision_bits() == max(a.precision_bits(), b.precision_bits())`
  - Cross-precision: 64+256 → 256, 128+7000 → 7000

#### to_f64()

*Within f64 range (F64 path):*
- `with_precision(1.5, 64).to_f64() == 1.5` (exact)
- Round-trip: `BigFloat::with_precision(x, 64).to_f64() == x` for any f64 x

*Beyond f64 range (FBig path):*
- Extreme large: `from_string("1e2000", 7000).to_f64() == f64::INFINITY`
- Extreme small: `from_string("1e-2000", 7000).to_f64() == 0.0`
- Document precision loss for high-precision values

*Boundary behavior:*
- `from_string("1.7976931348623157e308", 128).to_f64()` (at f64::MAX)
- `from_string("2.2250738585072014e-308", 128).to_f64()` (at f64::MIN_POSITIVE)

### 4. Comparison Tests (bigfloat_comparison.rs)

#### PartialEq (eq)

*Reflexivity:*
- `a == a` for all test values at all scales

*Separately constructed identical values:*
- `from_string("1e-2000", 7000) == from_string("1e-2000", 7000)`
- `with_precision(1.5, 128) == with_precision(1.5, 128)`

*ULP-level inequality detection:*
- At 7000 bits, values differing by smallest representable amount detected as unequal
- E.g., `1.0e-2000` vs `1.0000000000000001e-2000` at sufficient precision

*Cross-path comparisons:*
- F64 vs FBig with same value: `with_precision(1.5, 64) == with_precision(1.5, 128)`

#### PartialOrd (partial_cmp)

*Basic ordering:*
- `1e-2000 < 2e-2000` and `2e-2000 > 1e-2000`
- `1e-2000 <= 1e-2000` (equal case)

*Transitivity:*
- If `a < b` and `b < c`, then `a < c` at extreme scales
- Test with 10^-3000, 10^-2000, 10^-1000

*Cross-magnitude ordering:*
- `1e-5000 < 1e-2000 < 1e-100 < 1.0`

*Cross-path ordering:*
- F64 vs FBig: `with_precision(1.5, 64) < with_precision(2.5, 128)`

### 5. Serialization Tests (bigfloat_serialization.rs)

#### Round-trip identity
- **Basic**: `deserialize(serialize(x)) == x` for all test values
- **F64 path**: 32, 64 bits with various values (0.0, 1.5, 1e10, 1e-10)
- **FBig path**: 128, 512, 2048, 7000 bits
- **Extreme values**: 10^-5000, 10^5000 at 7000 bits

#### Precision preservation
- Serialize `from_string("1e-2000", 7000)`
- Deserialize and verify:
  - `precision_bits() == 7000`
  - Value mathematically identical
  - Usable in arithmetic operations

#### Format verification
- JSON structure contains:
  - `"value": "..."` (string representation)
  - `"precision_bits": N` (integer)
- Verify F64 vs FBig serialization format
- Human-readable for debugging

#### Cross-precision round-trips
- Serialize at 7000 bits, deserialize, perform arithmetic
- Verify precision metadata survives serialization boundary

## Implementation Notes

### Test Naming Convention
```rust
#[test] fn add_f64_path_same_scale() { ... }
#[test] fn add_boundary_64_to_65_bits() { ... }
#[test] fn add_extreme_7000_bits_tiny_values() { ... }
#[test] fn mul_cross_scale_magnitude_swing() { ... }
```

### Helper Functions
Start with direct string literals. Add helpers only when repetitive patterns emerge:
```rust
// Only if needed
fn extreme_small(mantissa: &str, exponent: i32, precision: usize) -> BigFloat
fn extreme_large(mantissa: &str, exponent: i32, precision: usize) -> BigFloat
```

### Coverage Estimate
- ~15 functions × ~4 scale levels × ~3-5 scenarios each
- **Total: 200-300 test cases**
- Organized across 5 modules

## Success Criteria

All tests must:
1. Use exact equality assertions (no tolerance)
2. Pass at all precision scales (64 to 7000 bits)
3. Pass at all magnitude scales (10^-5000 to 10^5000)
4. Cover all code paths (F64, FBig, cross-path)
5. Execute without panics or undefined behavior

This test suite proves BigFloat correctness for Fractal Wonder's extreme zoom requirements (10^2000+ zoom levels requiring 7000+ bit precision).
