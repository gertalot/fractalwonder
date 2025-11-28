// Delta iteration compute shader for f32 perturbation rendering.

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
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> reference_orbit: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;
@group(0) @binding(3) var<storage, read_write> glitch_flags: array<u32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= uniforms.width || gid.y >= uniforms.height) {
        return;
    }

    let idx = gid.y * uniforms.width + gid.x;

    // Compute delta-c for this pixel
    let dc = vec2<f32>(
        uniforms.dc_origin_re + f32(gid.x) * uniforms.dc_step_re,
        uniforms.dc_origin_im + f32(gid.y) * uniforms.dc_step_im
    );

    var dz = vec2<f32>(0.0, 0.0);
    var m: u32 = 0u;
    let orbit_len = arrayLength(&reference_orbit);
    var glitched = false;

    for (var n: u32 = 0u; n < uniforms.max_iterations; n++) {
        let Z = reference_orbit[m];
        let z = Z + dz;

        let z_sq = dot(z, z);
        let Z_sq = dot(Z, Z);
        let dz_sq = dot(dz, dz);

        // Escape check
        if (z_sq > uniforms.escape_radius_sq) {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            return;
        }

        // Pauldelbrot glitch detection: |z|^2 < tau^2 * |Z|^2
        if (Z_sq > 1e-20 && z_sq < uniforms.tau_sq * Z_sq) {
            glitched = true;
        }

        // Rebase check: |z|^2 < |dz|^2
        if (z_sq < dz_sq) {
            dz = z;
            m = 0u;
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
        if (m >= orbit_len) {
            m = 0u;
        }
    }

    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
}
