// Progressive GPU Rendering Shader
// Processes row-sets with iteration chunking and persistent state.

// HDRFloat library (same as delta_iteration_hdr.wgsl)
struct HDRFloat {
    head: f32,
    tail: f32,
    exp: i32,
}

struct HDRComplex {
    re: HDRFloat,
    im: HDRFloat,
}

const HDR_ZERO: HDRFloat = HDRFloat(0.0, 0.0, 0);
const HDR_COMPLEX_ZERO: HDRComplex = HDRComplex(HDRFloat(0.0, 0.0, 0), HDRFloat(0.0, 0.0, 0));

fn hdr_exp2(n: i32) -> f32 {
    if n < -149 { return 0.0; }
    if n > 127 { return 1.0e38; }
    if n >= -126 {
        return bitcast<f32>(u32(n + 127) << 23u);
    }
    return bitcast<f32>(1u << u32(n + 149));
}

fn hdr_two_sum_err(a: f32, b: f32, sum: f32) -> f32 {
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    return (a - a_virtual) + (b - b_virtual);
}

fn hdr_normalize(x: HDRFloat) -> HDRFloat {
    var head = x.head;
    var tail = x.tail;
    var exp = x.exp;

    if head == 0.0 {
        if tail == 0.0 { return HDR_ZERO; }
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

fn hdr_neg(a: HDRFloat) -> HDRFloat {
    return HDRFloat(-a.head, -a.tail, a.exp);
}

fn hdr_mul(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    if a.head == 0.0 || b.head == 0.0 { return HDR_ZERO; }
    let p = a.head * b.head;
    let err = fma(a.head, b.head, -p);
    let tail = err + a.head * b.tail + a.tail * b.head;
    return hdr_normalize(HDRFloat(p, tail, a.exp + b.exp));
}

fn hdr_square(a: HDRFloat) -> HDRFloat {
    if a.head == 0.0 { return HDR_ZERO; }
    let p = a.head * a.head;
    let err = fma(a.head, a.head, -p);
    let tail = err + 2.0 * a.head * a.tail;
    return hdr_normalize(HDRFloat(p, tail, a.exp * 2));
}

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

fn hdr_sub(a: HDRFloat, b: HDRFloat) -> HDRFloat {
    return hdr_add(a, hdr_neg(b));
}

fn hdr_mul_f32(a: HDRFloat, b: f32) -> HDRFloat {
    if a.head == 0.0 || b == 0.0 { return HDR_ZERO; }
    let p = a.head * b;
    let err = fma(a.head, b, -p);
    let tail = err + a.tail * b;
    return hdr_normalize(HDRFloat(p, tail, a.exp));
}

fn hdr_to_f32(x: HDRFloat) -> f32 {
    if x.head == 0.0 { return 0.0; }
    let mantissa = x.head + x.tail;
    let clamped_exp = clamp(x.exp, -126, 127);
    return mantissa * hdr_exp2(clamped_exp);
}

fn hdr_complex_square(a: HDRComplex) -> HDRComplex {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    let re_im = hdr_mul(a.re, a.im);
    let two_re_im = HDRFloat(re_im.head, re_im.tail, re_im.exp + 1);
    return HDRComplex(hdr_sub(re_sq, im_sq), two_re_im);
}

fn hdr_complex_norm_sq(a: HDRComplex) -> f32 {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    let sum = hdr_add(re_sq, im_sq);
    return hdr_to_f32(sum);
}

// Return norm_sq as HDRFloat (preserves extended exponent range)
fn hdr_complex_norm_sq_hdr(a: HDRComplex) -> HDRFloat {
    let re_sq = hdr_square(a.re);
    let im_sq = hdr_square(a.im);
    return hdr_add(re_sq, im_sq);
}

// Compare two HDRFloat values: a < b
// For magnitude comparisons, both values are non-negative
fn hdr_less_than(a: HDRFloat, b: HDRFloat) -> bool {
    // Handle zeros
    let a_zero = a.head == 0.0 && a.tail == 0.0;
    let b_zero = b.head == 0.0 && b.tail == 0.0;
    if a_zero { return !b_zero; }
    if b_zero { return false; }

    // Compare exponents first (both positive for magnitudes)
    if a.exp != b.exp {
        return a.exp < b.exp;
    }

    // Same exponent - compare mantissas
    return (a.head + a.tail) < (b.head + b.tail);
}

// Compare: a > b
fn hdr_greater_than(a: HDRFloat, b: HDRFloat) -> bool {
    return hdr_less_than(b, a);
}

// Create HDRFloat from f32 constant (for escape_radius_sq, tau_sq)
fn hdr_from_f32_const(val: f32) -> HDRFloat {
    if val == 0.0 { return HDR_ZERO; }
    return hdr_normalize(HDRFloat(val, 0.0, 0));
}

fn hdr_from_parts(head: f32, tail: f32, exp: i32) -> HDRFloat {
    return HDRFloat(head, tail, exp);
}

// ============================================================
// Progressive Iteration Shader
// ============================================================

struct Uniforms {
    image_width: u32,
    image_height: u32,
    row_set_index: u32,
    row_set_count: u32,
    row_set_pixel_count: u32,
    _pad0: u32,

    chunk_start_iter: u32,
    chunk_size: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,
    _pad1: u32,

    dc_origin_re_head: f32,
    dc_origin_re_tail: f32,
    dc_origin_re_exp: i32,
    _pad2: u32,
    dc_origin_im_head: f32,
    dc_origin_im_tail: f32,
    dc_origin_im_exp: i32,
    _pad3: u32,

    dc_step_re_head: f32,
    dc_step_re_tail: f32,
    dc_step_re_exp: i32,
    _pad4: u32,
    dc_step_im_head: f32,
    dc_step_im_tail: f32,
    dc_step_im_exp: i32,
    _pad5: u32,

    reference_escaped: u32,
    orbit_len: u32,
    _pad6a: u32,
    _pad6b: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;

// Persistent state buffers - HDRFloat stored as 3 consecutive f32s (head, tail, exp as f32)
@group(0) @binding(2) var<storage, read_write> z_re: array<f32>;
@group(0) @binding(3) var<storage, read_write> z_im: array<f32>;
@group(0) @binding(4) var<storage, read_write> iter_count: array<u32>;
@group(0) @binding(5) var<storage, read_write> escaped_buf: array<u32>;
@group(0) @binding(6) var<storage, read_write> orbit_index: array<u32>;

// Result buffers
@group(0) @binding(7) var<storage, read_write> results: array<u32>;
@group(0) @binding(8) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(9) var<storage, read_write> z_norm_sq: array<f32>;

// Inline helper functions for z_re buffer
fn load_z_re(idx: u32) -> HDRFloat {
    let base = idx * 3u;
    return HDRFloat(z_re[base], z_re[base + 1u], i32(bitcast<u32>(z_re[base + 2u])));
}

fn store_z_re(idx: u32, val: HDRFloat) {
    let base = idx * 3u;
    z_re[base] = val.head;
    z_re[base + 1u] = val.tail;
    z_re[base + 2u] = bitcast<f32>(u32(val.exp));
}

// Inline helper functions for z_im buffer
fn load_z_im(idx: u32) -> HDRFloat {
    let base = idx * 3u;
    return HDRFloat(z_im[base], z_im[base + 1u], i32(bitcast<u32>(z_im[base + 2u])));
}

fn store_z_im(idx: u32, val: HDRFloat) {
    let base = idx * 3u;
    z_im[base] = val.head;
    z_im[base + 1u] = val.tail;
    z_im[base + 2u] = bitcast<f32>(u32(val.exp));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let linear_idx = id.x;
    if linear_idx >= uniforms.row_set_pixel_count {
        return;
    }

    // Check if already escaped
    if escaped_buf[linear_idx] != 0u {
        return;
    }

    // Convert linear index to image coordinates
    let row_within_set = linear_idx / uniforms.image_width;
    let col = linear_idx % uniforms.image_width;
    let global_row = row_within_set * uniforms.row_set_count + uniforms.row_set_index;

    // Construct δc for this pixel
    let dc_origin_re = hdr_from_parts(uniforms.dc_origin_re_head, uniforms.dc_origin_re_tail, uniforms.dc_origin_re_exp);
    let dc_origin_im = hdr_from_parts(uniforms.dc_origin_im_head, uniforms.dc_origin_im_tail, uniforms.dc_origin_im_exp);
    let dc_step_re = hdr_from_parts(uniforms.dc_step_re_head, uniforms.dc_step_re_tail, uniforms.dc_step_re_exp);
    let dc_step_im = hdr_from_parts(uniforms.dc_step_im_head, uniforms.dc_step_im_tail, uniforms.dc_step_im_exp);

    let x_hdr = HDRFloat(f32(col), 0.0, 0);
    let y_hdr = HDRFloat(f32(global_row), 0.0, 0);
    let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
    let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
    let dc = HDRComplex(dc_re, dc_im);

    // Load persistent state
    var dz = HDRComplex(load_z_re(linear_idx), load_z_im(linear_idx));
    var n = iter_count[linear_idx];
    var m = orbit_index[linear_idx];
    var glitched = glitch_flags[linear_idx] != 0u;

    let orbit_len = uniforms.orbit_len;
    let reference_escaped = uniforms.reference_escaped != 0u;
    let chunk_end = min(uniforms.chunk_start_iter + uniforms.chunk_size, uniforms.max_iterations);

    // Safety limit
    var loop_count = 0u;
    let max_loops = uniforms.chunk_size * 4u;

    loop {
        if n >= chunk_end {
            break;
        }

        loop_count = loop_count + 1u;
        if loop_count > max_loops {
            glitched = true;
            break;
        }

        // Reference exhaustion
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let z_m = reference_orbit[m % orbit_len];
        let z_m_re = z_m.x;
        let z_m_im = z_m.y;

        let z_m_hdr_re = HDRFloat(z_m_re, 0.0, 0);
        let z_m_hdr_im = HDRFloat(z_m_im, 0.0, 0);
        let z_re_full = hdr_add(z_m_hdr_re, dz.re);
        let z_im_full = hdr_add(z_m_hdr_im, dz.im);
        let z = HDRComplex(z_re_full, z_im_full);

        // Compute magnitudes as HDRFloat (preserves precision)
        let z_mag_sq_hdr = hdr_complex_norm_sq_hdr(z);
        let dz_mag_sq_hdr = hdr_complex_norm_sq_hdr(dz);

        // For output, convert to f32
        let z_mag_sq = hdr_to_f32(z_mag_sq_hdr);

        let z_m_mag_sq = z_m_re * z_m_re + z_m_im * z_m_im;

        // Escape check - use HDRFloat comparison
        let escape_radius_sq_hdr = hdr_from_f32_const(uniforms.escape_radius_sq);
        if hdr_greater_than(z_mag_sq_hdr, escape_radius_sq_hdr) {
            escaped_buf[linear_idx] = 1u;
            results[linear_idx] = n;
            glitch_flags[linear_idx] = select(0u, 1u, glitched);
            z_norm_sq[linear_idx] = z_mag_sq;
            store_z_re(linear_idx, dz.re);
            store_z_im(linear_idx, dz.im);
            iter_count[linear_idx] = n;
            orbit_index[linear_idx] = m;
            return;
        }

        // Glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < uniforms.tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // Rebase check: when z crosses near origin, delta becomes larger than full value.
        // Reset to use z as the new delta and restart reference orbit index.
        // NOTE: Rebasing is a precision technique, NOT a Mandelbrot iteration.
        // The iteration count n should NOT be reset during rebase.
        if z_mag_sq < dz_mag_sq {
            dz = z;
            m = 0u;
            continue;
        }

        // Delta iteration: dz' = 2*z_m*dz + dz^2 + dc
        // where z_m is f32 reference orbit value
        let two_z_dz_re = hdr_mul_f32(hdr_sub(hdr_mul_f32(dz.re, z_m_re), hdr_mul_f32(dz.im, z_m_im)), 2.0);
        let two_z_dz_im = hdr_mul_f32(hdr_add(hdr_mul_f32(dz.re, z_m_im), hdr_mul_f32(dz.im, z_m_re)), 2.0);
        let dz_sq = hdr_complex_square(dz);

        dz = HDRComplex(
            hdr_add(hdr_add(two_z_dz_re, dz_sq.re), dc.re),
            hdr_add(hdr_add(two_z_dz_im, dz_sq.im), dc.im)
        );

        m = m + 1u;
        n = n + 1u;
    }

    // Save state for next chunk
    store_z_re(linear_idx, dz.re);
    store_z_im(linear_idx, dz.im);
    iter_count[linear_idx] = n;
    orbit_index[linear_idx] = m;
    glitch_flags[linear_idx] = select(0u, 1u, glitched);

    // If we reached max_iterations, write final results
    if n >= uniforms.max_iterations {
        escaped_buf[linear_idx] = 1u;  // Mark as "done" even though didn't escape
        results[linear_idx] = uniforms.max_iterations;
        z_norm_sq[linear_idx] = 0.0;
    }
}
