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
    if n > 127 { return 1.0e38; } // saturate to large value
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

// Create HDRFloat from parts (for uniforms)
fn hdr_from_parts(head: f32, tail: f32, exp: i32) -> HDRFloat {
    return HDRFloat(head, tail, exp);
}

// ============================================================
// Delta Iteration Shader for HDRFloat Perturbation Rendering
// ============================================================

struct Uniforms {
    image_width: u32,
    image_height: u32,
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

    tile_offset_x: u32,
    tile_offset_y: u32,
    tile_width: u32,
    tile_height: u32,

    reference_escaped: u32,
    orbit_len: u32,
    _pad4a: u32,
    _pad4b: u32,
    _pad4c: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
// Orbit stored as 12 f32s per point:
// [Z_re_head, Z_re_tail, Z_im_head, Z_im_tail, Z_re_exp, Z_im_exp,
//  Der_re_head, Der_re_tail, Der_im_head, Der_im_tail, Der_re_exp, Der_im_exp]
// This uses full HDRFloat representation matching CPU: value = (head + tail) × 2^exp
@group(0) @binding(1) var<storage, read> reference_orbit: array<f32>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(4) var<storage, read_write> z_norm_sq: array<f32>;

const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFFu;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) local_id: vec3<u32>) {
    let local_x = local_id.x;
    let local_y = local_id.y;

    // Bounds check against tile size
    if local_x >= uniforms.tile_width || local_y >= uniforms.tile_height {
        return;
    }

    // Global pixel position for δc calculation
    let global_x = uniforms.tile_offset_x + local_x;
    let global_y = uniforms.tile_offset_y + local_y;

    // Tile-local buffer index
    let tile_idx = local_y * uniforms.tile_width + local_x;

    // Construct δc for this pixel using GLOBAL position
    let dc_origin_re = hdr_from_parts(uniforms.dc_origin_re_head, uniforms.dc_origin_re_tail, uniforms.dc_origin_re_exp);
    let dc_origin_im = hdr_from_parts(uniforms.dc_origin_im_head, uniforms.dc_origin_im_tail, uniforms.dc_origin_im_exp);
    let dc_step_re = hdr_from_parts(uniforms.dc_step_re_head, uniforms.dc_step_re_tail, uniforms.dc_step_re_exp);
    let dc_step_im = hdr_from_parts(uniforms.dc_step_im_head, uniforms.dc_step_im_tail, uniforms.dc_step_im_exp);

    // δc = dc_origin + global_pixel_pos * dc_step
    let x_hdr = HDRFloat(f32(global_x), 0.0, 0);
    let y_hdr = HDRFloat(f32(global_y), 0.0, 0);
    let dc_re = hdr_add(dc_origin_re, hdr_mul(x_hdr, dc_step_re));
    let dc_im = hdr_add(dc_origin_im, hdr_mul(y_hdr, dc_step_im));
    let dc = HDRComplex(dc_re, dc_im);

    // δz starts at origin
    var dz = HDR_COMPLEX_ZERO;
    var m: u32 = 0u;
    var glitched: bool = false;

    let orbit_len = uniforms.orbit_len;
    let reference_escaped = uniforms.reference_escaped != 0u;

    // Use a while loop with explicit iteration counter to avoid counting rebase steps.
    // The for loop would increment n even when continue is called after rebase,
    // which incorrectly counts rebasing as a Mandelbrot iteration.
    var n: u32 = 0u;

    // Safety: limit total loop iterations (including rebases) to prevent runaway execution.
    // At extreme zoom levels, precision issues could cause repeated rebasing.
    // Allow 4x max_iterations for rebases, which is generous but bounded.
    var total_loops: u32 = 0u;
    let max_total_loops = uniforms.max_iterations * 4u;

    loop {
        if n >= uniforms.max_iterations {
            break;
        }

        // Safety check: prevent infinite loops from repeated rebasing
        total_loops = total_loops + 1u;
        if total_loops > max_total_loops {
            // Mark as glitched since we couldn't complete normally
            glitched = true;
            break;
        }

        // Reference exhaustion detection
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        // Load Z_m from orbit (12 f32s per point, Z is at indices 0-5)
        let orbit_idx = (m % orbit_len) * 12u;
        let z_m_re_head = reference_orbit[orbit_idx];
        let z_m_re_tail = reference_orbit[orbit_idx + 1u];
        let z_m_im_head = reference_orbit[orbit_idx + 2u];
        let z_m_im_tail = reference_orbit[orbit_idx + 3u];
        let z_m_re_exp = bitcast<i32>(bitcast<u32>(reference_orbit[orbit_idx + 4u]));
        let z_m_im_exp = bitcast<i32>(bitcast<u32>(reference_orbit[orbit_idx + 5u]));

        // Load Der_m from orbit (at indices 6-11)
        let der_m_re_head = reference_orbit[orbit_idx + 6u];
        let der_m_re_tail = reference_orbit[orbit_idx + 7u];
        let der_m_im_head = reference_orbit[orbit_idx + 8u];
        let der_m_im_tail = reference_orbit[orbit_idx + 9u];
        let der_m_re_exp = bitcast<i32>(bitcast<u32>(reference_orbit[orbit_idx + 10u]));
        let der_m_im_exp = bitcast<i32>(bitcast<u32>(reference_orbit[orbit_idx + 11u]));

        // Full z = Z_m + δz (reconstruct Z_m as HDRFloat with proper exponent)
        let z_m_hdr_re = HDRFloat(z_m_re_head, z_m_re_tail, z_m_re_exp);
        let z_m_hdr_im = HDRFloat(z_m_im_head, z_m_im_tail, z_m_im_exp);
        let z_re = hdr_add(z_m_hdr_re, dz.re);
        let z_im = hdr_add(z_m_hdr_im, dz.im);
        let z = HDRComplex(z_re, z_im);

        // Compute magnitudes as HDRFloat (preserves precision)
        let z_mag_sq_hdr = hdr_complex_norm_sq_hdr(z);
        let dz_mag_sq_hdr = hdr_complex_norm_sq_hdr(dz);

        // For output, convert to f32
        let z_mag_sq = hdr_to_f32(z_mag_sq_hdr);
        // Use full HDRFloat precision for glitch detection
        let z_m_mag_sq = hdr_to_f32(hdr_complex_norm_sq_hdr(HDRComplex(z_m_hdr_re, z_m_hdr_im)));

        // 1. Escape check - use HDRFloat comparison
        let escape_radius_sq_hdr = hdr_from_f32_const(uniforms.escape_radius_sq);
        if hdr_greater_than(z_mag_sq_hdr, escape_radius_sq_hdr) {
            results[tile_idx] = n;
            glitch_flags[tile_idx] = select(0u, 1u, glitched);
            z_norm_sq[tile_idx] = z_mag_sq;
            return;
        }

        // 2. Pauldelbrot glitch detection - use HDRFloat comparison
        let z_m_mag_sq_hdr = hdr_from_f32_const(z_m_mag_sq);
        let threshold_hdr = hdr_from_f32_const(1e-20);
        if hdr_greater_than(z_m_mag_sq_hdr, threshold_hdr) {
            let tau_z_m_sq_hdr = hdr_mul_f32(z_m_mag_sq_hdr, uniforms.tau_sq);
            if hdr_less_than(z_mag_sq_hdr, tau_z_m_sq_hdr) {
                glitched = true;
            }
        }

        // 3. Rebase check: when z crosses near origin, delta becomes larger than full value.
        // Reset to use z as the new delta and restart reference orbit index.
        // NOTE: Rebasing is a precision technique, NOT a Mandelbrot iteration.
        // The iteration count n should NOT increment during rebase.
        // Use HDRFloat comparison to preserve precision for very small values
        if hdr_less_than(z_mag_sq_hdr, dz_mag_sq_hdr) {
            dz = z;
            m = 0u;
            // Do NOT increment n - rebase is not a real iteration
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        // Use z_m HDRFloat values for full precision multiplication
        let two_z_dz_re = hdr_mul_f32(hdr_sub(hdr_mul(dz.re, z_m_hdr_re), hdr_mul(dz.im, z_m_hdr_im)), 2.0);
        let two_z_dz_im = hdr_mul_f32(hdr_add(hdr_mul(dz.re, z_m_hdr_im), hdr_mul(dz.im, z_m_hdr_re)), 2.0);

        // δz²
        let dz_sq = hdr_complex_square(dz);

        // δz' = 2·Z·δz + δz² + δc
        dz = HDRComplex(
            hdr_add(hdr_add(two_z_dz_re, dz_sq.re), dc.re),
            hdr_add(hdr_add(two_z_dz_im, dz_sq.im), dc.im)
        );

        m = m + 1u;
        n = n + 1u; // Only increment iteration count after a real iteration
    }

    // Reached max iterations
    results[tile_idx] = uniforms.max_iterations;
    glitch_flags[tile_idx] = select(0u, 1u, glitched);
    z_norm_sq[tile_idx] = 0.0;
}
