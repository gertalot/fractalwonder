// Delta iteration compute shader for f32 perturbation rendering.
// Supports Adam7 progressive rendering via adam7_step uniform.

struct Uniforms {
    width: u32,
    height: u32,
    max_iterations: u32,
    escape_radius_sq: f32,
    tau_sq: f32,
    dc_origin_re: f32,
    dc_origin_im: f32,
    dc_step_re: f32,
    dc_step_im: f32,
    adam7_step: u32,          // 0 = compute all, 1-7 = Adam7 pass
    reference_escaped: u32,   // 1 if reference orbit escaped (short orbit), 0 otherwise
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;
@group(0) @binding(4) var<storage, read_write> z_norm_sq: array<f32>;

// Adam7 interlacing matrix (8x8 pattern, values 1-7)
fn get_adam7_pass(x: u32, y: u32) -> u32 {
    // Row-major 8x8 matrix indexed by [y % 8][x % 8]
    let row = y % 8u;
    let col = x % 8u;

    // Pattern encodes which pass (1-7) each pixel belongs to
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

// Sentinel value for uncomputed pixels
const SENTINEL_NOT_COMPUTED: u32 = 0xFFFFFFFFu;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x >= uniforms.width || gid.y >= uniforms.height {
        return;
    }

    let idx = gid.y * uniforms.width + gid.x;

    // Adam7 early exit: skip pixels not in current pass
    if uniforms.adam7_step > 0u && get_adam7_pass(gid.x, gid.y) != uniforms.adam7_step {
        // Write sentinel to indicate "not computed this pass"
        results[idx] = SENTINEL_NOT_COMPUTED;
        glitch_flags[idx] = 0u;
        return;
    }

    // Compute delta-c for this pixel
    let dc = vec2<f32>(
        uniforms.dc_origin_re + f32(gid.x) * uniforms.dc_step_re,
        uniforms.dc_origin_im + f32(gid.y) * uniforms.dc_step_im
    );

    var dz = vec2<f32>(0.0, 0.0);
    var m: u32 = 0u;
    let orbit_len = arrayLength(&reference_orbit);
    var glitched = false;

    // Use a while loop with explicit iteration counter to avoid counting rebase steps.
    // The for loop would increment n even when continue is called after rebase,
    // which incorrectly counts rebasing as a Mandelbrot iteration.
    var n: u32 = 0u;
    loop {
        if n >= uniforms.max_iterations {
            break;
        }

        // Reference exhaustion detection: m exceeded orbit length
        // Only applies when reference escaped (short orbit), not when reference is in-set
        if uniforms.reference_escaped != 0u && m >= orbit_len {
            glitched = true;
        }

        let Z = reference_orbit[m % orbit_len];
        let z = Z + dz;

        let z_sq = dot(z, z);
        let Z_sq = dot(Z, Z);
        let dz_sq = dot(dz, dz);

        // Escape check
        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            z_norm_sq[idx] = z_sq;
            return;
        }

        // Pauldelbrot glitch detection: |z|^2 < tau^2 * |Z|^2
        if Z_sq > 1e-20 && z_sq < uniforms.tau_sq * Z_sq {
            glitched = true;
        }

        // Rebase check: |z|^2 < |dz|^2
        // NOTE: Rebasing is a precision technique, NOT a Mandelbrot iteration.
        // The iteration count n should NOT increment during rebase.
        if z_sq < dz_sq {
            dz = z;
            m = 0u;
            // Do NOT increment n - rebase is not a real iteration
            continue;
        }

        // Delta iteration: dz' = 2*Z*dz + dz^2 + dc
        let two_Z_dz_re = 2.0 * (Z.x * dz.x - Z.y * dz.y);
        let two_Z_dz_im = 2.0 * (Z.x * dz.y + Z.y * dz.x);
        let dz_sq_re = dz.x * dz.x - dz.y * dz.y;
        let dz_sq_im = 2.0 * dz.x * dz.y;

        dz = vec2<f32>(
            two_Z_dz_re + dz_sq_re + dc.x,
            two_Z_dz_im + dz_sq_im + dc.y
        );

        m = m + 1u;
        n = n + 1u; // Only increment iteration count after a real iteration
    }

    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
    z_norm_sq[idx] = 0.0;
}
