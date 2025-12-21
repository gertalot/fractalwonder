# Derivative-Based 3D Lighting Design

## Overview

Replace the current Sobel-based slope shading with mathematically correct derivative-based normal computation. This tracks the derivative `ρ = dz/dc` during Mandelbrot iteration, enabling proper surface normals for Blinn-Phong lighting.

## Motivation

The current Sobel-based approach computes gradients in image-space from iteration counts, which creates visible "hard edges" at the light/shadow boundary. The derivative-based approach computes true surface normals from the mathematical derivative of the iteration, producing smooth, physically correct lighting.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Derivative tracking | Perturbation-based (δρ alongside δz) | Industry standard (Kalles Fraktaler), efficient for deep zoom |
| Platform support | Full CPU/GPU parity | Maintains architectural consistency |
| Data storage | Raw z and ρ in MandelbrotData | Keeps computation and coloring separate |
| Orbit buffer | Extended to include Der_m | Single buffer upload, simpler than separate buffers |
| Lighting model | Blinn-Phong (diffuse + specular) | Full 3D look with highlights |
| Existing Sobel code | Remove entirely | No legacy code, clean replacement |
| Settings UI | None (tweak in code) | Matches current approach |
| Interior points | Skip shading (stay black) | No meaningful derivative exists |
| Glitched pixels | Shade anyway | Tackle glitch handling separately |
| Small derivative handling | None needed | Normalization makes magnitude irrelevant |

## Data Structures

### MandelbrotData (fractalwonder-core/src/compute_data.rs)

```rust
pub struct MandelbrotData {
    pub iterations: u32,
    pub max_iterations: u32,
    pub escaped: bool,
    pub glitched: bool,
    pub final_z_norm_sq: f32,       // existing
    // New fields:
    pub final_z_re: f32,            // Real part of z at escape
    pub final_z_im: f32,            // Imaginary part of z at escape
    pub final_derivative_re: f32,   // Real part of ρ at escape
    pub final_derivative_im: f32,   // Imaginary part of ρ at escape
}
```

For interior points (`escaped = false`), the new fields are set to `0.0`.

### Reference Orbit Buffer

Current format per iteration point (6 × f32):
```
[Z_re_head, Z_re_tail, Z_im_head, Z_im_tail, Z_re_exp, Z_im_exp]
```

New format per iteration point (12 × f32):
```
[Z_re_head, Z_re_tail, Z_im_head, Z_im_tail, Z_re_exp, Z_im_exp,
 Der_re_head, Der_re_tail, Der_im_head, Der_im_tail, Der_re_exp, Der_im_exp]
```

This doubles the orbit buffer size but keeps a single upload.

### ShadingSettings (fractalwonder-ui/src/rendering/colorizers/settings.rs)

```rust
pub struct ShadingSettings {
    pub enabled: bool,
    pub light_azimuth: f64,    // Horizontal angle (0 = right, π/2 = top)
    pub light_elevation: f64,  // Vertical angle (0 = horizon, π/2 = overhead)
    pub ambient: f64,          // Ambient light level (default: 0.15)
    pub diffuse: f64,          // Diffuse strength (default: 0.7)
    pub specular: f64,         // Specular strength (default: 0.3)
    pub shininess: f64,        // Specular exponent (default: 32.0)
}
```

## Formulas

### Reference Orbit (high precision, computed once per frame)

```
Z₀ = 0
Der₀ = 0

Z_{m+1} = Z_m² + C
Der_{m+1} = 2·Z_m·Der_m + 1
```

### Perturbation Iteration (low precision, per pixel)

```
δz₀ = 0
δρ₀ = 0

δz_{n+1} = 2·Z_m·δz_n + δz_n² + δc              (existing, unchanged)
δρ_{n+1} = 2·Z_m·δρ_n + 2·δz_n·Der_m + 2·δz_n·δρ_n   (new)
```

### At Escape

```
final_z = Z_m + δz_n
final_derivative = Der_m + δρ_n
```

### Normal Computation (in colorizer)

```rust
let u = z / rho;                              // Complex division
let normal_2d = u / u.norm();                 // Normalize
let n = Vec3::new(normal_2d.re, normal_2d.im, 1.0).normalize();
```

### Blinn-Phong Lighting

```rust
let l = light_direction;                      // Unit vector toward light
let v = Vec3::new(0.0, 0.0, 1.0);            // View direction (straight down)
let h = (l + v).normalize();                  // Half vector

let n_dot_l = n.dot(l).max(0.0);
let n_dot_h = n.dot(h).max(0.0);

let diffuse = settings.diffuse * n_dot_l;
let specular = settings.specular * n_dot_h.powf(settings.shininess);
let shade = settings.ambient + diffuse + specular;

let final_color = base_color * shade.clamp(0.0, max_brightness);
```

## Files to Modify

### fractalwonder-core
- `compute_data.rs` - Add 4 new f32 fields to `MandelbrotData`

### fractalwonder-compute (CPU)
- `perturbation.rs` - Add δρ tracking to:
  - `compute_pixel_perturbation_bigfloat`
  - `compute_pixel_perturbation_hdr`
  - `compute_pixel_perturbation` (f64)
- Reference orbit computation - Add Der_m tracking alongside Z_m

### fractalwonder-gpu
- `perturbation_hdr_renderer.rs` - Extend orbit buffer to include Der_m
- `progressive_renderer.rs` - Same orbit buffer changes
- `shaders/delta_iteration_hdr.wgsl` - Add δρ iteration, output final derivative
- `shaders/progressive_iteration.wgsl` - Same shader changes

### fractalwonder-ui
- `rendering/colorizers/shading.rs` - Replace Sobel with Blinn-Phong
- `rendering/colorizers/settings.rs` - Update `ShadingSettings` struct

## Memory Impact

- **Per pixel:** +16 bytes (4 × f32 for z and ρ components)
- **Orbit buffer:** 2× size (adds Der_m alongside Z_m)

Orbit buffer is typically small compared to pixel data, so overall memory impact is modest.

## References

- [Mandelbrot set techniques (Univ-Toulouse)](https://www.math.univ-toulouse.fr/~cheritat/wiki-draw/index.php/Mandelbrot_set) - Normal map algorithm
- [Kalles Fraktaler Manual](https://mathr.co.uk/kf/manual.html) - Derivative tracking with perturbation
- [Deep Zoom Theory (mathr.co.uk)](https://mathr.co.uk/web/deep-zoom.html) - Perturbation and derivatives
- [Distance to Julia Set (Inigo Quilez)](https://iquilezles.org/articles/distancefractals/) - Derivative formulas
