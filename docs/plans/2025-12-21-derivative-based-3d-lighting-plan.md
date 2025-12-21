# Derivative-Based 3D Lighting Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Sobel-based slope shading with mathematically correct derivative-based Blinn-Phong lighting.

**Architecture:** Track derivative ρ = dz/dc during perturbation iteration alongside z. Store final z and ρ in MandelbrotData. Compute normals in colorizer as u = normalize(z/ρ). Apply Blinn-Phong lighting.

**Tech Stack:** Rust, WGSL shaders, WebGPU

---

## Task 1: Extend MandelbrotData with Derivative Fields

**Files:**
- Modify: `fractalwonder-core/src/compute_data.rs:40-54`

**Step 1: Add new fields to MandelbrotData**

```rust
/// Data computed for a Mandelbrot pixel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MandelbrotData {
    /// Number of iterations before escape (or max_iterations if didn't escape)
    pub iterations: u32,
    /// Maximum iterations used for this computation (for colorizer normalization)
    pub max_iterations: u32,
    /// Whether the point escaped the set
    pub escaped: bool,
    /// Whether this pixel was computed with a glitched reference orbit.
    #[serde(default)]
    pub glitched: bool,
    /// |z|² at escape for smooth iteration coloring. Interior points store 0.0.
    #[serde(default)]
    pub final_z_norm_sq: f32,
    /// Real part of z at escape (for derivative-based lighting)
    #[serde(default)]
    pub final_z_re: f32,
    /// Imaginary part of z at escape (for derivative-based lighting)
    #[serde(default)]
    pub final_z_im: f32,
    /// Real part of derivative ρ = dz/dc at escape
    #[serde(default)]
    pub final_derivative_re: f32,
    /// Imaginary part of derivative ρ = dz/dc at escape
    #[serde(default)]
    pub final_derivative_im: f32,
}
```

**Step 2: Run tests to verify compilation**

```bash
cargo check --workspace
```

**Step 3: Commit**

```bash
git add fractalwonder-core/src/compute_data.rs
git commit -m "feat(core): add derivative fields to MandelbrotData"
```

---

## Task 2: Add Reference Orbit Derivative Computation

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs:8-60`

**Step 1: Extend ReferenceOrbit struct**

Add derivative storage to ReferenceOrbit:

```rust
/// A pre-computed reference orbit for perturbation rendering.
#[derive(Clone)]
pub struct ReferenceOrbit {
    /// Reference point C as f64 (for on-the-fly computation after escape/rebase)
    pub c_ref: (f64, f64),
    /// Pre-computed orbit values Z_n as f64
    pub orbit: Vec<(f64, f64)>,
    /// Pre-computed derivative values Der_n = dZ_n/dC as f64
    pub derivative: Vec<(f64, f64)>,
    /// Iteration at which reference escaped (None if never escaped)
    pub escaped_at: Option<u32>,
}
```

**Step 2: Update ReferenceOrbit::compute to track derivative**

Modify the compute function to track Der_m alongside Z_m:

```rust
impl ReferenceOrbit {
    /// Compute a reference orbit using BigFloat precision.
    pub fn compute(c_ref: &(BigFloat, BigFloat), max_iterations: u32) -> Self {
        let precision = c_ref.0.precision_bits();
        let mut orbit = Vec::with_capacity(max_iterations as usize);
        let mut derivative = Vec::with_capacity(max_iterations as usize);

        let mut x = BigFloat::zero(precision);
        let mut y = BigFloat::zero(precision);
        // Derivative: Der_0 = 0
        let mut der_x = BigFloat::zero(precision);
        let mut der_y = BigFloat::zero(precision);

        let escape_radius_sq = BigFloat::with_precision(65536.0, precision);
        let one = BigFloat::with_precision(1.0, precision);
        let two = BigFloat::with_precision(2.0, precision);

        let mut escaped_at = None;

        for n in 0..max_iterations {
            // Store current Z_n and Der_n as f64
            orbit.push((x.to_f64(), y.to_f64()));
            derivative.push((der_x.to_f64(), der_y.to_f64()));

            // Check escape: |z|^2 > 65536
            let x_sq = x.mul(&x);
            let y_sq = y.mul(&y);
            if x_sq.add(&y_sq).gt(&escape_radius_sq) {
                escaped_at = Some(n);
                break;
            }

            // Derivative update: Der' = 2*Z*Der + 1
            // (der_x + i*der_y)' = 2*(x + i*y)*(der_x + i*der_y) + 1
            // Real: 2*(x*der_x - y*der_y) + 1
            // Imag: 2*(x*der_y + y*der_x)
            let new_der_x = two.mul(&x.mul(&der_x).sub(&y.mul(&der_y))).add(&one);
            let new_der_y = two.mul(&x.mul(&der_y).add(&y.mul(&der_x)));

            // z = z^2 + c
            let new_x = x_sq.sub(&y_sq).add(&c_ref.0);
            let new_y = two.mul(&x).mul(&y).add(&c_ref.1);

            x = new_x;
            y = new_y;
            der_x = new_der_x;
            der_y = new_der_y;
        }

        Self {
            c_ref: (c_ref.0.to_f64(), c_ref.1.to_f64()),
            orbit,
            derivative,
            escaped_at,
        }
    }
}
```

**Step 3: Run tests**

```bash
cargo test --package fractalwonder-compute -- --nocapture
```

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "feat(compute): track derivative in reference orbit"
```

---

## Task 3: Add Derivative Tracking to f64 Perturbation

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs:436-541`

**Step 1: Update compute_pixel_perturbation to track δρ**

The formula is: `δρ_{n+1} = 2·Z_m·δρ_n + 2·δz_n·Der_m + 2·δz_n·δρ_n`

```rust
pub fn compute_pixel_perturbation(
    orbit: &ReferenceOrbit,
    delta_c: (f64, f64),
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    // δz starts at origin
    let mut dz = (0.0_f64, 0.0_f64);
    // δρ starts at origin (derivative delta)
    let mut drho = (0.0_f64, 0.0_f64);
    let mut m: usize = 0;
    let mut glitched = false;

    let orbit_len = orbit.orbit.len();
    if orbit_len == 0 {
        return MandelbrotData {
            iterations: 0,
            max_iterations,
            escaped: false,
            glitched: true,
            final_z_norm_sq: 0.0,
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
        };
    }

    let reference_escaped = orbit.escaped_at.is_some();
    let mut n: u32 = 0;

    while n < max_iterations {
        if reference_escaped && m >= orbit_len {
            glitched = true;
        }

        let z_m = orbit.orbit[m % orbit_len];
        let der_m = orbit.derivative[m % orbit_len];

        // Full values: z = Z_m + δz, ρ = Der_m + δρ
        let z = (z_m.0 + dz.0, z_m.1 + dz.1);
        let rho = (der_m.0 + drho.0, der_m.1 + drho.1);

        let z_mag_sq = z.0 * z.0 + z.1 * z.1;
        let z_m_mag_sq = z_m.0 * z_m.0 + z_m.1 * z_m.1;
        let dz_mag_sq = dz.0 * dz.0 + dz.1 * dz.1;

        // 1. Escape check
        if z_mag_sq > 65536.0 {
            return MandelbrotData {
                iterations: n,
                max_iterations,
                escaped: true,
                glitched,
                final_z_norm_sq: z_mag_sq as f32,
                final_z_re: z.0 as f32,
                final_z_im: z.1 as f32,
                final_derivative_re: rho.0 as f32,
                final_derivative_im: rho.1 as f32,
            };
        }

        // 2. Pauldelbrot glitch detection
        if z_m_mag_sq > 1e-20 && z_mag_sq < tau_sq * z_m_mag_sq {
            glitched = true;
        }

        // 3. Rebase check
        if z_mag_sq < dz_mag_sq {
            dz = z;
            drho = rho;  // Also rebase derivative
            m = 0;
            continue;
        }

        // 4. Delta iteration: δz' = 2·Z_m·δz + δz² + δc
        let two_z_dz = (
            2.0 * (z_m.0 * dz.0 - z_m.1 * dz.1),
            2.0 * (z_m.0 * dz.1 + z_m.1 * dz.0),
        );
        let dz_sq = (dz.0 * dz.0 - dz.1 * dz.1, 2.0 * dz.0 * dz.1);
        dz = (
            two_z_dz.0 + dz_sq.0 + delta_c.0,
            two_z_dz.1 + dz_sq.1 + delta_c.1,
        );

        // 5. Derivative delta iteration: δρ' = 2·Z_m·δρ + 2·δz·Der_m + 2·δz·δρ
        // Note: use OLD dz value (before update) for δρ calculation
        // Actually, we need the NEW dz for the next iteration's δρ
        // Let's reconsider: δρ_{n+1} uses δz_n (the value BEFORE this iteration's update)
        // So we need to compute δρ update BEFORE δz update
        // Let me fix the order...

        m += 1;
        n += 1;
    }

    MandelbrotData {
        iterations: max_iterations,
        max_iterations,
        escaped: false,
        glitched,
        final_z_norm_sq: 0.0,
        final_z_re: 0.0,
        final_z_im: 0.0,
        final_derivative_re: 0.0,
        final_derivative_im: 0.0,
    }
}
```

**IMPORTANT:** The derivative update must use the OLD δz value before updating δz. The correct order is:

```rust
// Store old dz for derivative calculation
let old_dz = dz;

// Update δz
let two_z_dz = (
    2.0 * (z_m.0 * dz.0 - z_m.1 * dz.1),
    2.0 * (z_m.0 * dz.1 + z_m.1 * dz.0),
);
let dz_sq = (dz.0 * dz.0 - dz.1 * dz.1, 2.0 * dz.0 * dz.1);
dz = (
    two_z_dz.0 + dz_sq.0 + delta_c.0,
    two_z_dz.1 + dz_sq.1 + delta_c.1,
);

// Update δρ using OLD δz: δρ' = 2·Z_m·δρ + 2·δz·Der_m + 2·δz·δρ
// Term 1: 2·Z_m·δρ (complex mult)
let two_z_drho = (
    2.0 * (z_m.0 * drho.0 - z_m.1 * drho.1),
    2.0 * (z_m.0 * drho.1 + z_m.1 * drho.0),
);
// Term 2: 2·δz·Der_m (complex mult, using old_dz)
let two_dz_der = (
    2.0 * (old_dz.0 * der_m.0 - old_dz.1 * der_m.1),
    2.0 * (old_dz.0 * der_m.1 + old_dz.1 * der_m.0),
);
// Term 3: 2·δz·δρ (complex mult, using old_dz)
let two_dz_drho = (
    2.0 * (old_dz.0 * drho.0 - old_dz.1 * drho.1),
    2.0 * (old_dz.0 * drho.1 + old_dz.1 * drho.0),
);
drho = (
    two_z_drho.0 + two_dz_der.0 + two_dz_drho.0,
    two_z_drho.1 + two_dz_der.1 + two_dz_drho.1,
);
```

**Step 2: Run tests**

```bash
cargo test --package fractalwonder-compute -- --nocapture
```

**Step 3: Fix any test failures**

Update test helper functions to include new fields.

**Step 4: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "feat(compute): add derivative tracking to f64 perturbation"
```

---

## Task 4: Add Derivative Tracking to HDR Perturbation

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs:191-293`

**Step 1: Update compute_pixel_perturbation_hdr**

Same pattern as f64 but using HDRComplex:

```rust
pub fn compute_pixel_perturbation_hdr(
    orbit: &ReferenceOrbit,
    delta_c: HDRComplex,
    max_iterations: u32,
    tau_sq: f64,
) -> MandelbrotData {
    let mut dz = HDRComplex::ZERO;
    let mut drho = HDRComplex::ZERO;  // Derivative delta
    let mut m: usize = 0;
    let mut glitched = false;

    // ... (similar structure to f64 version)

    // At escape, combine reference + delta for both z and derivative:
    // z = Z_m + δz
    // ρ = Der_m + δρ

    // Store as f32 in MandelbrotData
}
```

**Step 2: Run tests**

```bash
cargo test --package fractalwonder-compute -- --nocapture
```

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "feat(compute): add derivative tracking to HDR perturbation"
```

---

## Task 5: Add Derivative Tracking to BigFloat Perturbation

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs:74-187`

**Step 1: Update compute_pixel_perturbation_bigfloat**

Same pattern using BigFloat arithmetic.

**Step 2: Run tests**

```bash
cargo test --package fractalwonder-compute -- --nocapture
```

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "feat(compute): add derivative tracking to BigFloat perturbation"
```

---

## Task 6: Update BLA Perturbation Functions

**Files:**
- Modify: `fractalwonder-compute/src/perturbation.rs:296-418` (HDR BLA)
- Modify: `fractalwonder-compute/src/perturbation.rs:543-650` (f64 BLA)

**Step 1: Add derivative tracking to BLA functions**

BLA skips iterations, so derivative must also be approximated. For now, track derivative normally during non-BLA iterations and skip during BLA (less accurate but functional).

**Step 2: Run all tests**

```bash
cargo test --package fractalwonder-compute -- --nocapture
```

**Step 3: Commit**

```bash
git add fractalwonder-compute/src/perturbation.rs
git commit -m "feat(compute): add derivative tracking to BLA perturbation"
```

---

## Task 7: Extend GPU Orbit Buffer for Derivatives

**Files:**
- Modify: `fractalwonder-gpu/src/progressive_renderer.rs:94-118`

**Step 1: Update orbit buffer format**

Change from 6 f32s per point to 12 f32s (add Der_re, Der_im as HDRFloat):

```rust
// Orbit stored as 12 f32s per point:
// [Z_re_head, Z_re_tail, Z_im_head, Z_im_tail, Z_re_exp, Z_im_exp,
//  Der_re_head, Der_re_tail, Der_im_head, Der_im_tail, Der_re_exp, Der_im_exp]
let orbit_data: Vec<[f32; 12]> = orbit
    .iter()
    .zip(orbit_derivative.iter())
    .map(|(&(z_re, z_im), &(der_re, der_im))| {
        let z_re_hdr = fractalwonder_core::HDRFloat::from_f64(z_re);
        let z_im_hdr = fractalwonder_core::HDRFloat::from_f64(z_im);
        let der_re_hdr = fractalwonder_core::HDRFloat::from_f64(der_re);
        let der_im_hdr = fractalwonder_core::HDRFloat::from_f64(der_im);
        [
            z_re_hdr.head, z_re_hdr.tail,
            z_im_hdr.head, z_im_hdr.tail,
            f32::from_bits(z_re_hdr.exp as u32), f32::from_bits(z_im_hdr.exp as u32),
            der_re_hdr.head, der_re_hdr.tail,
            der_im_hdr.head, der_im_hdr.tail,
            f32::from_bits(der_re_hdr.exp as u32), f32::from_bits(der_im_hdr.exp as u32),
        ]
    })
    .collect();
```

**Step 2: Update render_row_set signature**

Add derivative orbit parameter.

**Step 3: Run build check**

```bash
cargo check --package fractalwonder-gpu
```

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/progressive_renderer.rs
git commit -m "feat(gpu): extend orbit buffer for derivative data"
```

---

## Task 8: Add GPU Derivative State Buffers

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs`

**Step 1: Add derivative buffers to ProgressiveGpuBuffers**

Add buffers for δρ_re and δρ_im (same format as z_re, z_im).

**Step 2: Add output buffers for final derivative**

Add buffers for final_z_re, final_z_im, final_derivative_re, final_derivative_im.

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add derivative state and output buffers"
```

---

## Task 9: Update GPU Shader for Derivative Tracking

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl`

**Step 1: Add derivative state buffers**

```wgsl
// Add after existing state buffers (line ~240)
@group(0) @binding(10) var<storage, read_write> drho_re: array<f32>;
@group(0) @binding(11) var<storage, read_write> drho_im: array<f32>;

// Add output buffers for final values
@group(0) @binding(12) var<storage, read_write> final_z_re_buf: array<f32>;
@group(0) @binding(13) var<storage, read_write> final_z_im_buf: array<f32>;
@group(0) @binding(14) var<storage, read_write> final_der_re_buf: array<f32>;
@group(0) @binding(15) var<storage, read_write> final_der_im_buf: array<f32>;
```

**Step 2: Load derivative from extended orbit buffer**

```wgsl
// After loading z_m (line ~332-342)
let orbit_idx = (m % orbit_len) * 12u;  // Changed from 6u to 12u
// ... z_m loading unchanged ...
let der_m_re_head = reference_orbit[orbit_idx + 6u];
let der_m_re_tail = reference_orbit[orbit_idx + 7u];
let der_m_im_head = reference_orbit[orbit_idx + 8u];
let der_m_im_tail = reference_orbit[orbit_idx + 9u];
let der_m_re_exp = bitcast<i32>(bitcast<u32>(reference_orbit[orbit_idx + 10u]));
let der_m_im_exp = bitcast<i32>(bitcast<u32>(reference_orbit[orbit_idx + 11u]));

let der_m_hdr_re = HDRFloat(der_m_re_head, der_m_re_tail, der_m_re_exp);
let der_m_hdr_im = HDRFloat(der_m_im_head, der_m_im_tail, der_m_im_exp);
```

**Step 3: Add derivative delta iteration**

After the δz update (line ~393-396), add δρ update:

```wgsl
// Store old dz for derivative calculation
let old_dz = dz;

// Existing δz update...
dz = HDRComplex(...);

// Derivative delta: δρ' = 2·Z_m·δρ + 2·δz·Der_m + 2·δz·δρ
// Term 1: 2·Z_m·δρ
let two_z_drho_re = hdr_mul_f32(hdr_sub(hdr_mul(drho.re, z_m_hdr_re), hdr_mul(drho.im, z_m_hdr_im)), 2.0);
let two_z_drho_im = hdr_mul_f32(hdr_add(hdr_mul(drho.re, z_m_hdr_im), hdr_mul(drho.im, z_m_hdr_re)), 2.0);

// Term 2: 2·δz·Der_m (using old_dz)
let two_dz_der_re = hdr_mul_f32(hdr_sub(hdr_mul(old_dz.re, der_m_hdr_re), hdr_mul(old_dz.im, der_m_hdr_im)), 2.0);
let two_dz_der_im = hdr_mul_f32(hdr_add(hdr_mul(old_dz.re, der_m_hdr_im), hdr_mul(old_dz.im, der_m_hdr_re)), 2.0);

// Term 3: 2·δz·δρ (using old_dz)
let two_dz_drho_re = hdr_mul_f32(hdr_sub(hdr_mul(old_dz.re, drho.re), hdr_mul(old_dz.im, drho.im)), 2.0);
let two_dz_drho_im = hdr_mul_f32(hdr_add(hdr_mul(old_dz.re, drho.im), hdr_mul(old_dz.im, drho.re)), 2.0);

drho = HDRComplex(
    hdr_add(hdr_add(two_z_drho_re, two_dz_der_re), two_dz_drho_re),
    hdr_add(hdr_add(two_z_drho_im, two_dz_der_im), two_dz_drho_im)
);
```

**Step 4: Store final values at escape**

```wgsl
// At escape (line ~360-368)
// Compute full values
let z = HDRComplex(z_re_full, z_im_full);
let rho_re = hdr_add(der_m_hdr_re, drho.re);
let rho_im = hdr_add(der_m_hdr_im, drho.im);

// Store final values as f32
final_z_re_buf[linear_idx] = hdr_to_f32(z_re_full);
final_z_im_buf[linear_idx] = hdr_to_f32(z_im_full);
final_der_re_buf[linear_idx] = hdr_to_f32(rho_re);
final_der_im_buf[linear_idx] = hdr_to_f32(rho_im);
```

**Step 5: Update rebase to include derivative**

```wgsl
// At rebase (line ~381-384)
if hdr_less_than(z_mag_sq_hdr, dz_mag_sq_hdr) {
    dz = z;
    // Also rebase derivative
    drho = HDRComplex(
        hdr_add(der_m_hdr_re, drho.re),
        hdr_add(der_m_hdr_im, drho.im)
    );
    m = 0u;
    continue;
}
```

**Step 6: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): add derivative tracking to progressive shader"
```

---

## Task 10: Update GPU Result Readback

**Files:**
- Modify: `fractalwonder-gpu/src/progressive_renderer.rs:150-176`

**Step 1: Read derivative buffers and construct MandelbrotData**

```rust
let data: Vec<ComputeData> = iterations
    .iter()
    .zip(glitch_data.iter())
    .zip(z_norm_sq_data.iter())
    .zip(final_z_re_data.iter())
    .zip(final_z_im_data.iter())
    .zip(final_der_re_data.iter())
    .zip(final_der_im_data.iter())
    .map(|((((((iter, glitch), z_sq), z_re), z_im), der_re), der_im)| {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: *iter,
            max_iterations,
            escaped: *iter < max_iterations,
            glitched: *glitch != 0,
            final_z_norm_sq: *z_sq,
            final_z_re: *z_re,
            final_z_im: *z_im,
            final_derivative_re: *der_re,
            final_derivative_im: *der_im,
        })
    })
    .collect();
```

**Step 2: Commit**

```bash
git add fractalwonder-gpu/src/progressive_renderer.rs
git commit -m "feat(gpu): read derivative values in result conversion"
```

---

## Task 11: Update ShadingSettings for Blinn-Phong

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/settings.rs`

**Step 1: Replace ShadingSettings struct**

```rust
/// Settings for derivative-based Blinn-Phong lighting.
#[derive(Clone, Debug)]
pub struct ShadingSettings {
    pub enabled: bool,
    /// Light azimuth angle in radians (0 = right, π/2 = top)
    pub light_azimuth: f64,
    /// Light elevation angle in radians (0 = horizon, π/2 = overhead)
    pub light_elevation: f64,
    /// Ambient light level [0, 1]
    pub ambient: f64,
    /// Diffuse reflection strength [0, 1]
    pub diffuse: f64,
    /// Specular reflection strength [0, 1]
    pub specular: f64,
    /// Specular exponent (shininess)
    pub shininess: f64,
}

impl Default for ShadingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            light_azimuth: std::f64::consts::FRAC_PI_4,  // 45°
            light_elevation: std::f64::consts::FRAC_PI_4,  // 45°
            ambient: 0.15,
            diffuse: 0.7,
            specular: 0.3,
            shininess: 32.0,
        }
    }
}

impl ShadingSettings {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }
}
```

**Step 2: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/settings.rs
git commit -m "feat(colorizer): update ShadingSettings for Blinn-Phong"
```

---

## Task 12: Replace Sobel Shading with Derivative-Based Blinn-Phong

**Files:**
- Rewrite: `fractalwonder-ui/src/rendering/colorizers/shading.rs`

**Step 1: Implement new shading module**

```rust
//! Derivative-based 3D lighting using Blinn-Phong shading model.

use super::ShadingSettings;
use fractalwonder_core::{ComputeData, MandelbrotData};

/// Check if a compute data point is interior (didn't escape).
fn is_interior(data: &ComputeData) -> bool {
    match data {
        ComputeData::Mandelbrot(m) => !m.escaped,
        ComputeData::TestImage(_) => false,
    }
}

/// Compute light direction vector from azimuth and elevation angles.
fn light_direction(azimuth: f64, elevation: f64) -> (f64, f64, f64) {
    let cos_elev = elevation.cos();
    (
        azimuth.cos() * cos_elev,
        azimuth.sin() * cos_elev,
        elevation.sin(),
    )
}

/// Compute surface normal from z and derivative at escape.
/// Returns (nx, ny, nz) normalized vector.
fn compute_normal(m: &MandelbrotData) -> Option<(f64, f64, f64)> {
    let z_re = m.final_z_re as f64;
    let z_im = m.final_z_im as f64;
    let rho_re = m.final_derivative_re as f64;
    let rho_im = m.final_derivative_im as f64;

    // u = z / ρ (complex division)
    let rho_norm_sq = rho_re * rho_re + rho_im * rho_im;
    if rho_norm_sq < 1e-20 {
        return None;  // Degenerate case
    }

    // z / ρ = (z_re + i*z_im) / (rho_re + i*rho_im)
    //       = (z_re*rho_re + z_im*rho_im + i*(z_im*rho_re - z_re*rho_im)) / |ρ|²
    let u_re = (z_re * rho_re + z_im * rho_im) / rho_norm_sq;
    let u_im = (z_im * rho_re - z_re * rho_im) / rho_norm_sq;

    // Normalize u to unit vector in 2D
    let u_norm = (u_re * u_re + u_im * u_im).sqrt();
    if u_norm < 1e-20 {
        return None;
    }
    let u_re = u_re / u_norm;
    let u_im = u_im / u_norm;

    // 3D normal: (u_re, u_im, 1) normalized
    let n_len = (u_re * u_re + u_im * u_im + 1.0).sqrt();
    Some((u_re / n_len, u_im / n_len, 1.0 / n_len))
}

/// Apply Blinn-Phong shading to compute final shade value.
fn blinn_phong(
    normal: (f64, f64, f64),
    light: (f64, f64, f64),
    settings: &ShadingSettings,
) -> f64 {
    let (nx, ny, nz) = normal;
    let (lx, ly, lz) = light;

    // Diffuse: N · L
    let n_dot_l = (nx * lx + ny * ly + nz * lz).max(0.0);

    // View direction: straight down (0, 0, 1)
    let vz = 1.0;

    // Half vector: H = normalize(L + V)
    let hx = lx;
    let hy = ly;
    let hz = lz + vz;
    let h_len = (hx * hx + hy * hy + hz * hz).sqrt();
    let (hx, hy, hz) = (hx / h_len, hy / h_len, hz / h_len);

    // Specular: (N · H)^shininess
    let n_dot_h = (nx * hx + ny * hy + nz * hz).max(0.0);
    let specular = n_dot_h.powf(settings.shininess);

    // Combine
    settings.ambient + settings.diffuse * n_dot_l + settings.specular * specular
}

/// Apply derivative-based Blinn-Phong shading to a pixel buffer.
pub fn apply_slope_shading(
    pixels: &mut [[u8; 4]],
    data: &[ComputeData],
    _smooth_iters: &[f64],  // Not used in derivative-based approach
    settings: &ShadingSettings,
    width: usize,
    height: usize,
    _zoom_level: f64,  // Not needed - derivative is zoom-independent
) {
    if !settings.enabled {
        return;
    }

    let light = light_direction(settings.light_azimuth, settings.light_elevation);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;

            // Skip interior pixels
            if is_interior(&data[idx]) {
                continue;
            }

            let m = match &data[idx] {
                ComputeData::Mandelbrot(m) => m,
                _ => continue,
            };

            // Compute normal from derivative
            let normal = match compute_normal(m) {
                Some(n) => n,
                None => continue,  // Skip if degenerate
            };

            // Compute Blinn-Phong shade
            let shade = blinn_phong(normal, light, settings);

            // Apply shade to pixel
            pixels[idx] = apply_shade(pixels[idx], shade);
        }
    }
}

/// Apply shade value to a pixel.
/// shade: 1.0 = full brightness, 0.0 = black
fn apply_shade(base: [u8; 4], shade: f64) -> [u8; 4] {
    let shade = shade.clamp(0.0, 2.0);  // Allow some overbright for specular
    let apply = |c: u8| -> u8 {
        (c as f64 * shade).clamp(0.0, 255.0) as u8
    };
    [apply(base[0]), apply(base[1]), apply(base[2]), base[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_direction_horizontal() {
        let (x, y, z) = light_direction(0.0, 0.0);
        assert!((x - 1.0).abs() < 0.01);
        assert!(y.abs() < 0.01);
        assert!(z.abs() < 0.01);
    }

    #[test]
    fn light_direction_overhead() {
        let (x, y, z) = light_direction(0.0, std::f64::consts::FRAC_PI_2);
        assert!(x.abs() < 0.01);
        assert!(y.abs() < 0.01);
        assert!((z - 1.0).abs() < 0.01);
    }

    #[test]
    fn compute_normal_valid() {
        let m = MandelbrotData {
            iterations: 10,
            max_iterations: 100,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 100000.0,
            final_z_re: 100.0,
            final_z_im: 50.0,
            final_derivative_re: 10.0,
            final_derivative_im: 5.0,
        };
        let normal = compute_normal(&m);
        assert!(normal.is_some());
        let (nx, ny, nz) = normal.unwrap();
        // Should be normalized
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!((len - 1.0).abs() < 0.01);
    }

    #[test]
    fn blinn_phong_facing_light() {
        let normal = (0.0, 0.0, 1.0);  // Pointing straight up
        let light = (0.0, 0.0, 1.0);   // Light from above
        let settings = ShadingSettings::enabled();
        let shade = blinn_phong(normal, light, &settings);
        // Should be bright (ambient + diffuse + specular)
        assert!(shade > 0.8, "shade = {}", shade);
    }

    #[test]
    fn blinn_phong_away_from_light() {
        let normal = (0.0, 0.0, 1.0);  // Pointing up
        let light = (0.0, 0.0, -1.0);  // Light from below
        let settings = ShadingSettings::enabled();
        let shade = blinn_phong(normal, light, &settings);
        // Should be dark (ambient only, no diffuse/specular)
        assert!(shade < 0.3, "shade = {}", shade);
    }
}
```

**Step 2: Run tests**

```bash
cargo test --package fractalwonder-ui -- --nocapture
```

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/shading.rs
git commit -m "feat(colorizer): replace Sobel with derivative-based Blinn-Phong"
```

---

## Task 13: Update Caller Sites for New MandelbrotData

**Files:**
- Search for all places creating MandelbrotData and add new fields

**Step 1: Find all construction sites**

```bash
cargo check --workspace 2>&1 | grep "missing field"
```

**Step 2: Update each site with default values**

Add `final_z_re: 0.0, final_z_im: 0.0, final_derivative_re: 0.0, final_derivative_im: 0.0` to all MandelbrotData constructions.

**Step 3: Run full test suite**

```bash
cargo test --workspace -- --nocapture
```

**Step 4: Commit**

```bash
git add -A
git commit -m "fix: add new derivative fields to all MandelbrotData constructions"
```

---

## Task 14: Integration Testing

**Step 1: Run full build**

```bash
cargo build --workspace
```

**Step 2: Run WASM build**

```bash
wasm-pack test --headless --chrome
```

**Step 3: Manual testing**

1. Start `trunk serve`
2. Navigate to a zoom location
3. Enable shading
4. Verify 3D lighting effect is smooth without hard edges

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "fix: integration testing fixes"
```

---

## Task 15: Final Cleanup

**Step 1: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings -W clippy::all
```

**Step 2: Run rustfmt**

```bash
cargo fmt --all
```

**Step 3: Final commit**

```bash
git add -A
git commit -m "chore: clippy and formatting cleanup"
```

---

## Summary

This plan implements derivative-based 3D lighting in 15 tasks:

1. **Data structure**: Add derivative fields to MandelbrotData
2-6. **CPU computation**: Track derivatives in reference orbit and all perturbation functions
7-10. **GPU computation**: Extend orbit buffer and shader for derivative tracking
11-12. **Colorizer**: Replace Sobel with Blinn-Phong using derivative normals
13-15. **Integration**: Fix callers, test, cleanup

Each task is designed to be independently testable and committable.
