# HDRFloat Design: Extended Precision for GPU Rendering

**Date:** 2025-12-01
**Status:** Approved
**Replaces:** FloatExp

## Summary

Replace FloatExp with HDRFloat, a high dynamic range floating-point type that provides ~48-bit mantissa precision using two f32 values plus an extended exponent. This enables deeper GPU zoom levels while maintaining WebGPU compatibility.

## Motivation

1. **GPU precision limit:** Current FloatExp uses f32 mantissa (24-bit precision), causing artifacts at deep zoom
2. **CPU-GPU consistency:** Same representation on both sides simplifies data transfer
3. **WebGPU compatibility:** No f64 dependency; works on all GPU hardware

## Design

### Core Representation

```rust
// fractalwonder-core/src/hdrfloat.rs

/// High Dynamic Range Float: ~48-bit mantissa precision with extended exponent.
/// Value = (head + tail) × 2^exp
#[derive(Clone, Copy, Debug, Default)]
pub struct HDRFloat {
    pub head: f32,  // Primary mantissa, normalized to [0.5, 2.0)
    pub tail: f32,  // Error term, |tail| ≤ 0.5 × ulp(head)
    pub exp: i32,   // Binary exponent
}

pub struct HDRComplex {
    pub re: HDRFloat,
    pub im: HDRFloat,
}
```

**Memory:** 12 bytes per HDRFloat, 24 bytes per HDRComplex.

**Invariants:**
- Zero: `head == 0.0` implies `tail == 0.0` and `exp == 0`
- Normalized: For non-zero, `head` is in [0.5, 2.0)
- Error bound: `|tail| ≤ |head| × 2^-24`

### Arithmetic Operations

**Multiplication:** Uses FMA to capture rounding error.

```rust
pub fn mul(self, other: Self) -> Self {
    if self.head == 0.0 || other.head == 0.0 { return Self::ZERO; }

    let p = self.head * other.head;
    let err = self.head.mul_add(other.head, -p);  // FMA captures error
    let tail = err + self.head * other.tail + self.tail * other.head;

    Self { head: p, tail, exp: self.exp + other.exp }.normalize()
}
```

**Addition:** Uses two-sum algorithm for error-free addition.

```rust
pub fn add(self, other: Self) -> Self {
    // Align exponents, then:
    let sum = a_head + b_head;
    let err = two_sum_err(a_head, b_head, sum);
    let tail = err + a_tail + b_tail;

    Self { head: sum, tail, exp: result_exp }.normalize()
}

fn two_sum_err(a: f32, b: f32, sum: f32) -> f32 {
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    (a - a_virtual) + (b - b_virtual)
}
```

**Normalization:** Bit manipulation to extract/adjust exponent.

```rust
pub fn normalize(self) -> Self {
    if self.head == 0.0 { return Self::ZERO; }

    let bits = self.head.to_bits();
    let biased_exp = ((bits >> 23) & 0xFF) as i32;
    let exp_adjust = biased_exp - 126;

    let new_head = f32::from_bits((bits & 0x807F_FFFF) | (126 << 23));
    let scale = f32::from_bits(((127 - exp_adjust) as u32) << 23);
    let new_tail = self.tail * scale;

    Self { head: new_head, tail: new_tail, exp: self.exp + exp_adjust }
}
```

### BigFloat Conversion

```rust
pub fn from_bigfloat(bf: &BigFloat) -> Self {
    if bf.is_zero() { return Self::ZERO; }

    let log2_approx = bf.log2_approx();
    let exp = log2_approx.round() as i32;

    // Scale to [0.5, 2.0) range and convert to f64
    let mantissa_f64 = if exp.abs() < 1000 {
        bf.to_f64() * libm::exp2(-exp as f64)
    } else {
        let scale = BigFloat::exp2(-exp as i64, bf.precision_bits());
        bf.mul(&scale).to_f64()
    };

    // Split into head + tail
    let head = mantissa_f64 as f32;
    let tail = (mantissa_f64 - head as f64) as f32;

    Self { head, tail, exp }.normalize()
}
```

### WGSL Implementation

```wgsl
struct HDRFloat {
    head: f32,
    tail: f32,
    exp: i32,
}

fn hdr_mul(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    if a.head == 0.0 || b.head == 0.0 { return HDR_ZERO; }

    let p = a.head * b.head;
    let err = fma(a.head, b.head, -p);
    let tail = err + a.head * b.tail + a.tail * b.head;

    return hdr_normalize(HDRFloat(p, tail, a.exp + b.exp));
}

fn hdr_add(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    // Align exponents via scaling
    // Two-sum for error-free head addition
    // Combine tails with error term
}
```

### GPU Buffer Layout

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HDRFloatGpu {
    pub head: f32,
    pub tail: f32,
    pub exp: i32,
    pub _padding: u32,  // Align to 16 bytes
}
```

## Data Flow

```
BigFloat (arbitrary precision, CPU reference orbit)
    ↓
HDRFloat (48-bit mantissa, CPU delta computation)
    ↓
HDRFloatGpu (GPU-aligned layout)
    ↓
WGSL HDRFloat (shader arithmetic)
```

## Migration Plan

### Files to Delete

- `fractalwonder-core/src/floatexp.rs`
- `fractalwonder-gpu/src/shaders/floatexp.wgsl`
- `fractalwonder-gpu/src/shaders/direct_floatexp.wgsl`
- `fractalwonder-gpu/src/shaders/delta_iteration_floatexp.wgsl`

### Files to Create

- `fractalwonder-core/src/hdrfloat.rs`
- `fractalwonder-gpu/src/shaders/hdrfloat.wgsl`
- `fractalwonder-gpu/src/shaders/delta_iteration_hdr.wgsl`

### Files to Update

- `fractalwonder-core/src/lib.rs` - export HDRFloat instead of FloatExp
- `fractalwonder-compute/src/perturbation.rs` - replace FloatExp functions with HDR variants
- `fractalwonder-gpu/src/buffers.rs` - replace FloatExp buffer types with HDRFloatGpu
- `fractalwonder-gpu/src/perturbation_floatexp_pipeline.rs` → `perturbation_hdr_pipeline.rs`
- `fractalwonder-gpu/src/perturbation_floatexp_renderer.rs` → `perturbation_hdr_renderer.rs`
- `fractalwonder-gpu/src/direct_pipeline.rs`
- `fractalwonder-gpu/src/direct_renderer.rs`
- `fractalwonder-gpu/src/lib.rs`
- `fractalwonder-ui/src/rendering/parallel_renderer.rs`

### Test Updates

- Migrate FloatExp tests to HDRFloat
- Add precision comparison tests (HDRFloat vs BigFloat at 10^500+ zoom)
- Add GPU round-trip tests (Rust HDRFloat ↔ WGSL HDRFloat)

## References

- [FractalShark](https://github.com/mattsaccount364/FractalShark) - 2x32 float+exp implementation
- [Kalles Fraktaler 2+](https://mathr.co.uk/kf/kf.html) - Extended double perturbation
- [Heavy computing with GLSL](https://blog.cyclemap.link/2011-06-09-glsl-part2-emu/) - Double-single emulation
