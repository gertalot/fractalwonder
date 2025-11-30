// Delta iteration compute shader with FloatExp arithmetic.
// For deep zoom (> 10^20) using perturbation theory.

// --- BEGIN FLOATEXP LIBRARY ---
struct FloatExp {
    m: f32,
    e: i32,
}

struct ComplexFE {
    re: FloatExp,
    im: FloatExp,
}

const FE_ZERO: FloatExp = FloatExp(0.0, 0);
const CFE_ZERO: ComplexFE = ComplexFE(FloatExp(0.0, 0), FloatExp(0.0, 0));

fn fe_from_f32(x: f32) -> FloatExp {
    if x == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(x, 0));
}

fn fe_normalize(x: FloatExp) -> FloatExp {
    if x.m == 0.0 { return FE_ZERO; }
    let abs_m = abs(x.m);
    let e_adjust = i32(floor(log2(abs_m))) + 1;
    let new_m = x.m * exp2(f32(-e_adjust));
    return FloatExp(new_m, x.e + e_adjust);
}

fn fe_neg(a: FloatExp) -> FloatExp {
    return FloatExp(-a.m, a.e);
}

fn fe_mul(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 || b.m == 0.0 { return FE_ZERO; }
    return fe_normalize(FloatExp(a.m * b.m, a.e + b.e));
}

fn fe_add(a: FloatExp, b: FloatExp) -> FloatExp {
    if a.m == 0.0 { return b; }
    if b.m == 0.0 { return a; }
    let exp_diff = a.e - b.e;
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

fn fe_sub(a: FloatExp, b: FloatExp) -> FloatExp {
    return fe_add(a, fe_neg(b));
}

fn fe_to_f32(x: FloatExp) -> f32 {
    if x.m == 0.0 { return 0.0; }
    let clamped_e = clamp(x.e, -126, 127);
    return x.m * exp2(f32(clamped_e));
}

fn cfe_mul(a: ComplexFE, b: ComplexFE) -> ComplexFE {
    return ComplexFE(
        fe_sub(fe_mul(a.re, b.re), fe_mul(a.im, b.im)),
        fe_add(fe_mul(a.re, b.im), fe_mul(a.im, b.re))
    );
}

fn cfe_add(a: ComplexFE, b: ComplexFE) -> ComplexFE {
    return ComplexFE(fe_add(a.re, b.re), fe_add(a.im, b.im));
}

fn cfe_norm_sq(a: ComplexFE) -> f32 {
    let re_sq = fe_mul(a.re, a.re);
    let im_sq = fe_mul(a.im, a.im);
    let sum = fe_add(re_sq, im_sq);
    return fe_to_f32(sum);
}

fn vec2_to_cfe(v: vec2<f32>) -> ComplexFE {
    return ComplexFE(fe_from_f32(v.x), fe_from_f32(v.y));
}
// --- END FLOATEXP LIBRARY ---

struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,

    dc_origin_re_m: f32,
    dc_origin_re_e: i32,
    dc_origin_im_m: f32,
    dc_origin_im_e: i32,

    dc_step_re_m: f32,
    dc_step_re_e: i32,
    dc_step_im_m: f32,
    dc_step_im_e: i32,

    adam7_step: u32,
    reference_escaped: u32,
    _padding: vec2<u32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(4) var<storage, read_write> z_norm_sq: array<f32>;

fn get_adam7_pass(x: u32, y: u32) -> u32 {
    let row = y % 8u;
    let col = x % 8u;
    let matrix = array<array<u32, 8>, 8>(
        array<u32, 8>(1u, 6u, 4u, 6u, 2u, 6u, 4u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(5u, 6u, 5u, 6u, 5u, 6u, 5u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(3u, 6u, 4u, 6u, 3u, 6u, 4u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
        array<u32, 8>(5u, 6u, 5u, 6u, 5u, 6u, 5u, 6u),
        array<u32, 8>(7u, 7u, 7u, 7u, 7u, 7u, 7u, 7u),
    );
    return matrix[row][col];
}

const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFFu;

fn compute_pixel_dc(px: u32, py: u32) -> ComplexFE {
    let origin_re = FloatExp(uniforms.dc_origin_re_m, uniforms.dc_origin_re_e);
    let origin_im = FloatExp(uniforms.dc_origin_im_m, uniforms.dc_origin_im_e);
    let step_re = FloatExp(uniforms.dc_step_re_m, uniforms.dc_step_re_e);
    let step_im = FloatExp(uniforms.dc_step_im_m, uniforms.dc_step_im_e);

    let px_fe = fe_from_f32(f32(px));
    let py_fe = fe_from_f32(f32(py));

    return ComplexFE(
        fe_add(origin_re, fe_mul(px_fe, step_re)),
        fe_add(origin_im, fe_mul(py_fe, step_im))
    );
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= uniforms.width || gid.y >= uniforms.height {
        return;
    }

    let idx = gid.y * uniforms.width + gid.x;

    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        results[idx] = SENTINEL_NOT_COMPUTED;
        glitch_flags[idx] = 0u;
        return;
    }

    let dc = compute_pixel_dc(gid.x, gid.y);
    var dz = CFE_ZERO;
    var m: u32 = 0u;
    let orbit_len = arrayLength(&reference_orbit);
    var glitched = false;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        if uniforms.reference_escaped != 0u && m >= orbit_len {
            glitched = true;
        }

        let Z = reference_orbit[m % orbit_len];
        let Z_cfe = vec2_to_cfe(Z);
        let z = cfe_add(Z_cfe, dz);

        let z_sq = cfe_norm_sq(z);
        let Z_sq = dot(Z, Z);
        let dz_sq = cfe_norm_sq(dz);

        // Escape
        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            z_norm_sq[idx] = z_sq;
            return;
        }

        // Glitch detection
        if Z_sq > 1e-20 && z_sq < uniforms.tau_sq * Z_sq {
            glitched = true;
        }

        // Rebase: when |z| < |dz|, we've lost precision
        if z_sq < dz_sq {
            dz = z;
            m = 0u;
            continue;
        }

        // Delta iteration: δz' = 2Zδz + δz² + δc
        let two_Z = vec2_to_cfe(Z * 2.0);
        let two_Z_dz = cfe_mul(two_Z, dz);
        let dz_squared = cfe_mul(dz, dz);
        dz = cfe_add(cfe_add(two_Z_dz, dz_squared), dc);

        m = m + 1u;
    }

    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
    z_norm_sq[idx] = 0.0;
}
