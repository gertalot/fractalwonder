# Perturbation Trait Refactoring Design

## Problem

`fractalwonder-compute/src/perturbation.rs` contains 5 nearly identical functions for perturbation iteration:

| Function | Numeric Type | BLA | Used in Production |
|----------|--------------|-----|-------------------|
| `compute_pixel_perturbation` | f64 | No | Yes |
| `compute_pixel_perturbation_hdr` | HDRFloat | No | Yes |
| `compute_pixel_perturbation_hdr_bla` | HDRFloat | Yes | Yes |
| `compute_pixel_perturbation_bigfloat` | BigFloat | No | Yes |
| `compute_pixel_perturbation_bla` | f64 | Yes | **No (dead code)** |

The 4 non-BLA functions share ~90% identical logic. Each is ~150-180 lines, totaling ~600+ lines of duplication.

## Solution

Create a `ComplexDelta` trait that abstracts complex number operations, enabling a single generic function that the Rust compiler monomorphizes into type-specific code with zero runtime overhead.

## Trait Definition

```rust
// fractalwonder-core/src/complex_delta.rs

/// Complex number type for perturbation delta arithmetic.
///
/// Abstracts operations needed for perturbation iteration, enabling
/// a single generic implementation for f64, HDRFloat, and BigFloat.
pub trait ComplexDelta: Clone + Sized {
    /// Returns the additive identity (zero) with the same precision as self.
    fn zero(&self) -> Self;

    /// Construct from f64 real/imaginary components.
    fn from_f64_pair(re: f64, im: f64) -> Self;

    /// Extract as f64 pair for output and comparisons.
    fn to_f64_pair(&self) -> (f64, f64);

    /// Complex addition.
    fn add(&self, other: &Self) -> Self;

    /// Complex subtraction.
    fn sub(&self, other: &Self) -> Self;

    /// Complex multiplication.
    fn mul(&self, other: &Self) -> Self;

    /// Multiply by f64 scalar.
    fn scale(&self, factor: f64) -> Self;

    /// Complex square (optimized).
    fn square(&self) -> Self;

    /// Magnitude squared as f64 (for escape/rebase checks).
    fn norm_sq(&self) -> f64;
}
```

## Type Implementations

### F64Complex (new type)

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct F64Complex {
    pub re: f64,
    pub im: f64,
}
```

Standard complex arithmetic with native f64 operations.

### HDRComplex (existing type)

Implement trait by wrapping existing `HDRFloat` methods. Already has `add`, `sub`, `mul`, `square`.

### BigFloatComplex (new type)

```rust
#[derive(Clone, Debug)]
pub struct BigFloatComplex {
    pub re: BigFloat,
    pub im: BigFloat,
}
```

`zero(&self)` extracts precision from `self.re.precision_bits()` to create matching-precision zeros.

## Generic Function

```rust
pub fn compute_pixel_perturbation<D: ComplexDelta>(
    orbit: &ReferenceOrbit,
    delta_c: D,
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData
```

Replaces `compute_pixel_perturbation`, `compute_pixel_perturbation_hdr`, and `compute_pixel_perturbation_bigfloat`.

## What Stays Separate

`compute_pixel_perturbation_hdr_bla` remains a separate function because:
- BLA coefficients are f64, applied to HDRFloat deltas via `mul_f64()`
- BLA only benefits HDRFloat (f64 zoom too shallow, BigFloat not implemented)
- Mixing BLA into the trait would add complexity for a single use case

## Performance Considerations

**Accepted trade-off:** The current HDR code uses `HDRFloat::mul_f64()` to multiply by reference orbit values (f64). The generic function converts f64 to HDRComplex first, using full HDR×HDR multiplication. This may be slightly slower.

**Decision:** Keep the trait clean. Benchmark after implementation. If regression is unacceptable, revisit.

**Why this is likely acceptable:**
- HDR path is already 10-20x slower than f64
- BigFloat path (for extreme zooms) is even slower
- Code clarity outweighs micro-optimization

## File Changes

### fractalwonder-core/src/

| File | Change |
|------|--------|
| `lib.rs` | Add `pub mod complex_delta;` |
| `complex_delta.rs` | NEW: Trait + F64Complex + BigFloatComplex |
| `hdrfloat.rs` | Add `impl ComplexDelta for HDRComplex` |

### fractalwonder-compute/src/

| File | Change |
|------|--------|
| `perturbation.rs` | Replace 4 functions with 1 generic, delete dead f64+BLA |
| `lib.rs` | Update exports |
| `worker.rs` | Update call sites (minimal) |

## Migration Strategy

1. **Add trait and implementations** (non-breaking) - all existing code works
2. **Add generic function alongside existing** - both coexist
3. **Add equivalence tests** - verify generic matches existing for all types
4. **Switch call sites** - update worker.rs
5. **Delete old functions** - remove 4 duplicates + 1 dead code
6. **Benchmark** - verify acceptable performance

## Estimated Impact

- Current: ~1983 lines
- After: ~1100 lines
- **Reduction: ~880 lines (44%)**

## Decisions Made

| Question | Decision | Rationale |
|----------|----------|-----------|
| Operator overloading vs methods? | Methods | Matches existing BigFloat/HDRFloat API, cleaner reference semantics |
| How to handle BigFloat precision? | `zero(&self)` | Extracts precision from input, no magic defaults |
| Optimize HDR mul_f64? | No | Keep trait clean, benchmark first |
| Unify BLA variants? | No | BLA only applies to HDR, keep separate |
