// FloatExp: Extended-range floating point for GPU.
// Value = m × 2^e where m is normalized to [0.5, 1.0) or 0.

struct FloatExp {
    m: f32,  // mantissa
    e: i32,  // exponent (base 2)
}

struct ComplexFE {
    re: FloatExp,
    im: FloatExp,
}

// Zero constant
const FE_ZERO: FloatExp = FloatExp(0.0, 0);
const CFE_ZERO: ComplexFE = ComplexFE(FloatExp(0.0, 0), FloatExp(0.0, 0));

// Create FloatExp from f32
fn fe_from_f32(x: f32) -> FloatExp {
    if x == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(x, 0));
}

// Normalize mantissa to [0.5, 1.0)
fn fe_normalize(x: FloatExp) -> FloatExp {
    if x.m == 0.0 { return FE_ZERO; }

    let abs_m = abs(x.m);
    let e_adjust = i32(floor(log2(abs_m))) + 1;
    let new_m = x.m * exp2(f32(-e_adjust));

    return FloatExp(new_m, x.e + e_adjust);
}

// Negate
fn fe_neg(a: FloatExp) -> FloatExp {
    return FloatExp(-a.m, a.e);
}

// Multiply two FloatExp values
fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 || b.m == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(a.m * b.m, a.e + b.e));
}

// Add two FloatExp values
fn fe_add(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 { return b; }
    if b.m == 0.0 { return a; }

    let exp_diff = a.e - b.e;

    // If difference > 24 bits, smaller is negligible
    if exp_diff > 24 { return a; }
    if exp_diff < -24 { return b; }

    if exp_diff >= 0 {
        let scaled_b = b.m * exp2(f32(-exp_diff));
        return fe_normalize(FloatExp(a.m + scaled_b, a.e));
    } else {
        let scaled_a = a.m * exp2(f32(exp_diff));
        return fe_normalize(FloatExp(scaled_a + b.m, b.e));
    }
}

// Subtract: a - b
fn fe_sub(a: FloatExp, b: FloatExp) -> FloatExp {
    return fe_add(a, fe_neg(b));
}

// Convert FloatExp to f32 (may overflow/underflow)
fn fe_to_f32(x: FloatExp) -> f32 {
    if x.m == 0.0 { return 0.0; }
    // Clamp exponent to avoid inf/0
    let clamped_e = clamp(x.e, -126, 127);
    return x.m * exp2(f32(clamped_e));
}

// Complex multiplication: (a.re + a.im*i) * (b.re + b.im*i)
fn cfe_mul(a: ComplexFE, b: ComplexFE) -> ComplexFE {
    return ComplexFE(
        fe_sub(fe_mul(a.re, b.re), fe_mul(a.im, b.im)),
        fe_add(fe_mul(a.re, b.im), fe_mul(a.im, b.re))
    );
}

// Complex addition
fn cfe_add(a: ComplexFE, b: ComplexFE) -> ComplexFE {
    return ComplexFE(fe_add(a.re, b.re), fe_add(a.im, b.im));
}

// Complex squared magnitude |a|² = re² + im²
// Returns f32 since result is bounded for escape check
fn cfe_norm_sq(a: ComplexFE) -> f32 {
    let re_sq = fe_mul(a.re, a.re);
    let im_sq = fe_mul(a.im, a.im);
    let sum = fe_add(re_sq, im_sq);
    return fe_to_f32(sum);
}

// Convert vec2<f32> to ComplexFE
fn vec2_to_cfe(v: vec2<f32>) -> ComplexFE {
    return ComplexFE(fe_from_f32(v.x), fe_from_f32(v.y));
}
