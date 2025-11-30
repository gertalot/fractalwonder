// Direct Mandelbrot iteration using FloatExp arithmetic.
// For zoom levels < 10^20 where perturbation is not needed.

// Include FloatExp library (copy contents from floatexp.wgsl here since WGSL doesn't have #include)
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
// --- END FLOATEXP LIBRARY ---

struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,

    c_origin_re_m: f32,
    c_origin_re_e: i32,
    c_origin_im_m: f32,
    c_origin_im_e: i32,

    c_step_re_m: f32,
    c_step_re_e: i32,
    c_step_im_m: f32,
    c_step_im_e: i32,

    adam7_step: u32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read_write> results: array<u32>;
@group(0) @binding(2) var<storage, read_write> z_norm_sq: array<f32>;

// Adam7 interlacing pattern
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

fn compute_pixel_c(px: u32, py: u32) -> ComplexFE {
    let origin_re = FloatExp(uniforms.c_origin_re_m, uniforms.c_origin_re_e);
    let origin_im = FloatExp(uniforms.c_origin_im_m, uniforms.c_origin_im_e);
    let step_re = FloatExp(uniforms.c_step_re_m, uniforms.c_step_re_e);
    let step_im = FloatExp(uniforms.c_step_im_m, uniforms.c_step_im_e);

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

    // Adam7 early exit
    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        results[idx] = SENTINEL_NOT_COMPUTED;
        return;
    }

    let c = compute_pixel_c(gid.x, gid.y);
    var z = CFE_ZERO;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        let z_sq = cfe_norm_sq(z);

        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            z_norm_sq[idx] = z_sq;
            return;
        }

        // z = z² + c
        z = cfe_add(cfe_mul(z, z), c);
    }

    results[idx] = uniforms.max_iterations;
    z_norm_sq[idx] = cfe_norm_sq(z);
}
