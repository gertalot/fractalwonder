// HDRFloat: High Dynamic Range Float for GPU.
// Value = (head + tail) × 2^exp
// Provides ~48-bit mantissa precision using two f32 values.

struct HDRFloat {
    head: f32,  // Primary mantissa [0.5, 1.0)
    tail: f32,  // Error term
    exp: i32,   // Binary exponent
}

struct HDRComplex {
    re: HDRFloat,
    im: HDRFloat,
}

const HDR_ZERO: HDRFloat = HDRFloat(0.0, 0.0, 0);
const HDR_COMPLEX_ZERO: HDRComplex = HDRComplex(HDRFloat(0.0, 0.0, 0), HDRFloat(0.0, 0.0, 0));

// Compute 2^n for integer n
fn hdr_exp2(n: i32) -> f32 {
    if n < -149 { return 0.0; }
    if n > 127 { return bitcast<f32>(0x7F800000u); } // +inf
    if n >= -126 {
        return bitcast<f32>(u32(n + 127) << 23u);
    }
    return bitcast<f32>(1u << u32(n + 149));
}

// Two-sum error computation (Knuth)
fn hdr_two_sum_err(a: f32, b: f32, sum: f32) -> f32 {
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    return (a - a_virtual) + (b - b_virtual);
}

// Normalize head to [0.5, 1.0) - iterative version (WGSL doesn't support recursion)
fn hdr_normalize(x: HDRFloat) -> HDRFloat {
    var head = x.head;
    var tail = x.tail;
    var exp = x.exp;

    // If head is zero, promote tail to head
    if head == 0.0 {
        if tail == 0.0 {
            return HDR_ZERO;
        }
        head = tail;
        tail = 0.0;
    }

    let abs_head = abs(head);
    if abs_head >= 0.5 && abs_head < 1.0 {
        return HDRFloat(head, tail, exp);
    }

    let bits = bitcast<u32>(head);
    let sign = bits & 0x80000000u;
    let biased_exp = i32((bits >> 23u) & 0xFFu);

    let exp_adjust = biased_exp - 126;
    let new_mantissa_bits = (bits & 0x807FFFFFu) | 0x3F000000u;
    let new_head = bitcast<f32>(new_mantissa_bits | sign);
    let scale = hdr_exp2(-exp_adjust);
    let new_tail = tail * scale;

    return HDRFloat(new_head, new_tail, exp + exp_adjust);
}

// Negate
fn hdr_neg(a: HDRFloat) -> HDRFloat {
    return HDRFloat(-a.head, -a.tail, a.exp);
}

// Multiply with FMA error tracking
fn hdr_mul(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    if a.head == 0.0 || b.head == 0.0 { return HDR_ZERO; }

    let p = a.head * b.head;
    let err = fma(a.head, b.head, -p);
    let tail = err + a.head * b.tail + a.tail * b.head;

    return hdr_normalize(HDRFloat(p, tail, a.exp + b.exp));
}

// Square (optimized)
fn hdr_square(a: HDRFloat) -> HDRFloat {
    if a.head == 0.0 { return HDR_ZERO; }

    let p = a.head * a.head;
    let err = fma(a.head, a.head, -p);
    let tail = err + 2.0 * a.head * a.tail;

    return hdr_normalize(HDRFloat(p, tail, a.exp * 2));
}

// Add with exponent alignment
fn hdr_add(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    if a.head == 0.0 { return b; }
    if b.head == 0.0 { return a; }

    let exp_diff = a.exp - b.exp;
    if exp_diff > 48 { return a; }
    if exp_diff < -48 { return b; }

    var ah: f32; var at: f32; var bh: f32; var bt: f32; var result_exp: i32;

    if exp_diff >= 0 {
        let scale = hdr_exp2(-exp_diff);
        ah = a.head; at = a.tail;
        bh = b.head * scale; bt = b.tail * scale;
        result_exp = a.exp;
    } else {
        let scale = hdr_exp2(exp_diff);
        ah = a.head * scale; at = a.tail * scale;
        bh = b.head; bt = b.tail;
        result_exp = b.exp;
    }

    let sum = ah + bh;
    let err = hdr_two_sum_err(ah, bh, sum);
    let tail = err + at + bt;

    return hdr_normalize(HDRFloat(sum, tail, result_exp));
}

// Subtract
fn hdr_sub(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    return hdr_add(a, hdr_neg(b));
}

// Multiply HDRFloat by f32 (for reference orbit values)
fn hdr_mul_f32(a: HDRFloat, b: f32) -> HDRFloat {
    if a.head == 0.0 || b == 0.0 { return HDR_ZERO; }

    let p = a.head * b;
    let err = fma(a.head, b, -p);
    let tail = err + a.tail * b;

    return hdr_normalize(HDRFloat(p, tail, a.exp));
}

// Convert HDRFloat to f32 (for escape check)
fn hdr_to_f32(x: HDRFloat) -> f32 {
    if x.head == 0.0 { return 0.0; }
    let mantissa = x.head + x.tail;
    let clamped_exp = clamp(x.exp, -126, 127);
    return mantissa * hdr_exp2(clamped_exp);
}

// Complex multiplication
fn hdr_complex_mul(a: HDRComplex, b: HDRComplex) -> HDRComplex {
    return HDRComplex(
        hdr_sub(hdr_mul(a.re, b.re), hdr_mul(a.im, b.im)),
        hdr_add(hdr_mul(a.re, b.im), hdr_mul(a.im, b.re))
    );
}

// Complex addition
fn hdr_complex_add(a: HDRComplex, b: HDRComplex) -> HDRComplex {
    return HDRComplex(hdr_add(a.re, b.re), hdr_add(a.im, b.im));
}

// Complex subtraction
fn hdr_complex_sub(a: HDRComplex, b: HDRComplex) -> HDRComplex {
    return HDRComplex(hdr_sub(a.re, b.re), hdr_sub(a.im, b.im));
}

// Complex square: (a + bi)² = (a² - b²) + 2abi
fn hdr_complex_square(a: HDRComplex) -> HDRComplex {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    let re_im = hdr_mul(a.re, a.im);
    // Multiply by 2 exactly via exponent increment (no rounding error)
    let two_re_im = HDRFloat(re_im.head, re_im.tail, re_im.exp + 1);
    return HDRComplex(
        hdr_sub(re_sq, im_sq),
        two_re_im
    );
}

// Complex squared magnitude |a|² = re² + im²
fn hdr_complex_norm_sq(a: HDRComplex) -> f32 {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    let sum = hdr_add(re_sq, im_sq);
    return hdr_to_f32(sum);
}

// Create HDRFloat from parts (for uniforms)
fn hdr_from_parts(head: f32, tail: f32, exp: i32) -> HDRFloat {
    return HDRFloat(head, tail, exp);
}
