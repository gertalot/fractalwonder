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

// Normalize head to [0.5, 1.0)
fn hdr_normalize(x: HDRFloat) -> HDRFloat {
    if x.head == 0.0 {
        if x.tail != 0.0 {
            // Promote tail to head
            return hdr_normalize(HDRFloat(x.tail, 0.0, x.exp));
        }
        return HDR_ZERO;
    }

    let abs_head = abs(x.head);
    if abs_head >= 0.5 && abs_head < 1.0 {
        return x;
    }

    let bits = bitcast<u32>(x.head);
    let sign = bits & 0x80000000u;
    let biased_exp = i32((bits >> 23u) & 0xFFu);

    let exp_adjust = biased_exp - 126;
    let new_mantissa_bits = (bits & 0x807FFFFFu) | 0x3F000000u;
    let new_head = bitcast<f32>(new_mantissa_bits | sign);
    let scale = hdr_exp2(-exp_adjust);
    let new_tail = x.tail * scale;

    return HDRFloat(new_head, new_tail, x.exp + exp_adjust);
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

// ============================================================
// Delta Iteration Shader for HDRFloat Perturbation Rendering
// ============================================================

struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,
    _pad0: u32,

    dc_origin_re_head: f32,
    dc_origin_re_tail: f32,
    dc_origin_re_exp: i32,
    _pad1: u32,
    dc_origin_im_head: f32,
    dc_origin_im_tail: f32,
    dc_origin_im_exp: i32,
    _pad2: u32,

    dc_step_re_head: f32,
    dc_step_re_tail: f32,
    dc_step_re_exp: i32,
    _pad3: u32,
    dc_step_im_head: f32,
    dc_step_im_tail: f32,
    dc_step_im_exp: i32,

    adam7_step: u32,
    reference_escaped: u32,
    orbit_len: u32,
    _pad4: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(4) var<storage, read_write> z_norm_sq: array<f32>;

const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFFu;

// Adam7 interlacing pattern
fn adam7_coords(pass: u32) -> vec2<u32> {
    switch pass {
        case 1u: { return vec2<u32>(0u, 0u); }
        case 2u: { return vec2<u32>(4u, 0u); }
        case 3u: { return vec2<u32>(0u, 4u); }
        case 4u: { return vec2<u32>(2u, 0u); }
        case 5u: { return vec2<u32>(0u, 2u); }
        case 6u: { return vec2<u32>(1u, 0u); }
        case 7u: { return vec2<u32>(0u, 1u); }
        default: { return vec2<u32>(0u, 0u); }
    }
}

fn adam7_step_size(pass: u32) -> vec2<u32> {
    switch pass {
        case 1u: { return vec2<u32>(8u, 8u); }
        case 2u: { return vec2<u32>(8u, 8u); }
        case 3u: { return vec2<u32>(4u, 8u); }
        case 4u: { return vec2<u32>(4u, 4u); }
        case 5u: { return vec2<u32>(2u, 4u); }
        case 6u: { return vec2<u32>(2u, 2u); }
        case 7u: { return vec2<u32>(1u, 2u); }
        default: { return vec2<u32>(1u, 1u); }
    }
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if x >= uniforms.width || y >= uniforms.height {
        return;
    }

    let pixel_idx = y * uniforms.width + x;

    // Adam7 filtering
    if uniforms.adam7_step > 0u {
        let offset = adam7_coords(uniforms.adam7_step);
        let step = adam7_step_size(uniforms.adam7_step);
        if (x % step.x) != offset.x || (y % step.y) != offset.y {
            results[pixel_idx] = SENTINEL_NOT_COMPUTED;
            glitch_flags[pixel_idx] = 0u;
            z_norm_sq[pixel_idx] = 0.0;
            return;
        }
    }

    // Construct δc for this pixel
    let dc_origin_re = hdr_from_parts(uniforms.dc_origin_re_head, uniforms.dc_origin_re_tail, uniforms.dc_origin_re_exp);
    let dc_origin_im = hdr_from_parts(uniforms.dc_origin_im_head, uniforms.dc_origin_im_tail, uniforms.dc_origin_im_exp);
    let dc_step_re = hdr_from_parts(uniforms.dc_step_re_head, uniforms.dc_step_re_tail, uniforms.dc_step_re_exp);
    let dc_step_im = hdr_from_parts(uniforms.dc_step_im_head, uniforms.dc_step_im_tail, uniforms.dc_step_im_exp);

    // δc = dc_origin + pixel_pos * dc_step
    let x_hdr = HDRFloat(f32(x), 0.0, 0);
    let y_hdr = HDRFloat(f32(y), 0.0, 0);
    let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
    let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
    let dc = HDRComplex(dc_re, dc_im);

    // δz starts at origin
    var dz = HDR_COMPLEX_ZERO;
    var m: u32 = 0u;
    var glitched: bool = false;

    let orbit_len = uniforms.orbit_len;
    let reference_escaped = uniforms.reference_escaped != 0u;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n = n + 1u) {
        // Reference exhaustion detection
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        // Get Z_m from reference orbit
        let z_m = reference_orbit[m % orbit_len];
        let z_m_re = z_m.x;
        let z_m_im = z_m.y;

        // Full z = Z_m + δz (convert Z_m to HDRFloat for addition)
        let z_m_hdr_re = HDRFloat(z_m_re, 0.0, 0);
        let z_m_hdr_im = HDRFloat(z_m_im, 0.0, 0);
        let z_re = hdr_add(z_m_hdr_re, dz.re);
        let z_im = hdr_add(z_m_hdr_im, dz.im);
        let z = HDRComplex(z_re, z_im);

        // Magnitudes
        let z_mag_sq = hdr_complex_norm_sq(z);
        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;
        let dz_mag_sq = hdr_complex_norm_sq(dz);

        // 1. Escape check
        if z_mag_sq > uniforms.escape_radius_sq {
            results[pixel_idx] = n;
            glitch_flags[pixel_idx] = select(0u, 1u, glitched);
            z_norm_sq[pixel_idx] = z_mag_sq;
            return;
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < uniforms.tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check
        if z_mag_sq < dz_mag_sq {
            dz = z;
            m = 0u;
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        // 2·Z_m·δz (complex multiplication then scale by 2)
        let two_z_dz_re = hdr_mul_f32(hdr_sub(hdr_mul_f32(dz.re, z_m_re), hdr_mul_f32(dz.im, z_m_im)), 2.0);
        let two_z_dz_im = hdr_mul_f32(hdr_add(hdr_mul_f32(dz.re, z_m_im), hdr_mul_f32(dz.im, z_m_re)), 2.0);

        // δz²
        let dz_sq = hdr_complex_square(dz);

        // δz' = 2·Z·δz + δz² + δc
        dz = HDRComplex(
            hdr_add(hdr_add(two_z_dz_re, dz_sq.re), dc.re),
            hdr_add(hdr_add(two_z_dz_im, dz_sq.im), dc.im)
        );

        m = m + 1u;
    }

    // Reached max iterations
    results[pixel_idx] = uniforms.max_iterations;
    glitch_flags[pixel_idx] = select(0u, 1u, glitched);
    z_norm_sq[pixel_idx] = 0.0;
}
