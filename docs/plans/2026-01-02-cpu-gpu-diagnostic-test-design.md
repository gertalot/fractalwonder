# CPU/GPU Diagnostic Test Design

Compare CPU and GPU Mandelbrot renderers by running both on identical pixels and printing raw `MandelbrotData` output.

## Purpose

Diagnose rendering differences between CPU and GPU pipelines. The test uses **production code paths only** — no replicated logic.

## Test Parameters

```rust
// Viewport (full precision, 1067 bits)
center_x: "0.273000307495579097715200094310253922494103490187797182966812629706330340783242"
center_y: "0.005838718497531293679839354462882728828030188792949767250660666951674130465532"
width:    "1.38277278476513331960149825811900065907944121299848E-281"
height:   "7.97822331184022584815185255533429968247789646588334E-282"

// Image dimensions
image_width:  766
image_height: 432

// Pixels to test
row:       350
col_start: 580
col_end:   611  // 32 pixels total

// Iteration parameters
max_iterations: 10_000_000
tau_sq:         1e-6
```

## Output Format

```
Pixel 580:
  CPU: iterations=4523, max_iterations=10000000, escaped=true, glitched=false,
       final_z_norm_sq=256.3401489, surface_normal_re=0.7071068, surface_normal_im=0.7071068
  GPU: iterations=4523, max_iterations=10000000, escaped=true, glitched=false,
       final_z_norm_sq=256.3098755, surface_normal_re=0.7053421, surface_normal_im=0.7088692
  DIFF: final_z_norm_sq=0.0302734, surface_normal_re=0.0017647, surface_normal_im=0.0017624

Pixel 581:
  ...
```

All 7 `MandelbrotData` fields shown for both renderers. DIFF line shows fields that differ (omitted if identical).

## Implementation

### Shared Setup

1. Parse center coordinates as `BigFloat` at 1067-bit precision
2. Compute `ReferenceOrbit::compute(&center, max_iterations)` — production code
3. Build `BlaTable::build(&orbit, dc_max)` — production code

### CPU Path

Uses production tile renderer:

```rust
use fractalwonder_compute::perturbation::tile::{render_tile_hdr, TileConfig};

// Compute delta_origin for tile at (580, 350) — matches coordinator.rs logic
let norm_x = 580.0 / 766.0 - 0.5;
let norm_y = 350.0 / 432.0 - 0.5;
let delta_origin = (
    BigFloat::from(norm_x).mul(&viewport.width),
    BigFloat::from(norm_y).mul(&viewport.height),
);
let delta_origin_hdr = (
    HDRFloat::from_bigfloat(&delta_origin.0),
    HDRFloat::from_bigfloat(&delta_origin.1),
);

// Compute delta_step — matches coordinator.rs logic
let delta_step = (
    HDRFloat::from_bigfloat(&viewport.width.div_f64(766.0)),
    HDRFloat::from_bigfloat(&viewport.height.div_f64(432.0)),
);

let config = TileConfig {
    size: (32, 1),  // 32 pixels wide, 1 row
    max_iterations,
    tau_sq,
    bla_enabled: true,
};

let result = render_tile_hdr(&orbit, Some(&bla_table), delta_origin_hdr, delta_step, &config);
```

### GPU Path

Uses production progressive renderer:

```rust
use fractalwonder_gpu::{GpuContext, GpuAvailability, ProgressiveRenderer};

// Initialize GPU — production code
let context = match GpuContext::try_init().await {
    GpuAvailability::Available(ctx) => ctx,
    GpuAvailability::Unavailable(e) => panic!("GPU unavailable: {e}"),
};
let mut renderer = ProgressiveRenderer::new(context);

// Compute dc_origin and dc_step — matches parallel_renderer.rs:411-431
let vp_width = HDRFloat::from_bigfloat(&viewport.width);
let vp_height = HDRFloat::from_bigfloat(&viewport.height);
let half = HDRFloat::from_f64(0.5);
let origin_re = vp_width.mul(&half).neg();
let origin_im = vp_height.mul(&half).neg();
let step_re = vp_width.div_f64(766.0);
let step_im = vp_height.div_f64(432.0);

let dc_origin = (
    (origin_re.head, origin_re.tail, origin_re.exp),
    (origin_im.head, origin_im.tail, origin_im.exp),
);
let dc_step = (
    (step_re.head, step_re.tail, step_re.exp),
    (step_im.head, step_im.tail, step_im.exp),
);

// Configure row_set to include row 350
// With row_set_count=1, we render entire image in one pass
let result = renderer.render_row_set(
    &orbit.orbit,
    &orbit.derivative,
    orbit_id,
    dc_origin,
    dc_step,
    766,   // image_width
    432,   // image_height
    0,     // row_set_index (only one)
    1,     // row_set_count (render all rows)
    max_iterations,
    10000, // iterations_per_dispatch
    tau_sq as f32,
    reference_escaped,
    Some(&bla_table),
).await?;

// Extract row 350, columns 580-611
// Row 350 starts at index: 350 * 766 = 268100
// Columns 580-611 are at indices 268100+580 to 268100+611
let start_idx = 350 * 766 + 580;
let gpu_pixels: Vec<_> = result.data[start_idx..start_idx+32].to_vec();
```

### Comparison Logic

```rust
fn print_comparison(col: u32, cpu: &MandelbrotData, gpu: &MandelbrotData) {
    println!("Pixel {}:", col);
    println!("  CPU: iterations={}, max_iterations={}, escaped={}, glitched={},",
             cpu.iterations, cpu.max_iterations, cpu.escaped, cpu.glitched);
    println!("       final_z_norm_sq={}, surface_normal_re={}, surface_normal_im={}",
             cpu.final_z_norm_sq, cpu.surface_normal_re, cpu.surface_normal_im);
    println!("  GPU: iterations={}, max_iterations={}, escaped={}, glitched={},",
             gpu.iterations, gpu.max_iterations, gpu.escaped, gpu.glitched);
    println!("       final_z_norm_sq={}, surface_normal_re={}, surface_normal_im={}",
             gpu.final_z_norm_sq, gpu.surface_normal_re, gpu.surface_normal_im);

    let mut diffs = Vec::new();
    if cpu.iterations != gpu.iterations {
        diffs.push(format!("iterations={}", (cpu.iterations as i64 - gpu.iterations as i64).abs()));
    }
    if cpu.max_iterations != gpu.max_iterations {
        diffs.push(format!("max_iterations"));
    }
    if cpu.escaped != gpu.escaped {
        diffs.push(format!("escaped"));
    }
    if cpu.glitched != gpu.glitched {
        diffs.push(format!("glitched"));
    }
    if cpu.final_z_norm_sq != gpu.final_z_norm_sq {
        diffs.push(format!("final_z_norm_sq={:.7}", (cpu.final_z_norm_sq - gpu.final_z_norm_sq).abs()));
    }
    if cpu.surface_normal_re != gpu.surface_normal_re {
        diffs.push(format!("surface_normal_re={:.7}", (cpu.surface_normal_re - gpu.surface_normal_re).abs()));
    }
    if cpu.surface_normal_im != gpu.surface_normal_im {
        diffs.push(format!("surface_normal_im={:.7}", (cpu.surface_normal_im - gpu.surface_normal_im).abs()));
    }

    if diffs.is_empty() {
        println!("  (identical)");
    } else {
        println!("  DIFF: {}", diffs.join(", "));
    }
    println!();
}
```

## Test Location

`fractalwonder-gpu/tests/cpu_gpu_comparison.rs`

## Run Command

```bash
cargo test -p fractalwonder-gpu cpu_gpu_comparison -- --nocapture
```

## Notes

- Test uses `row_set_count=1` to render entire image, avoiding row-set boundary complexity
- GPU rendering is async; test uses `#[tokio::test]`
- Both paths use identical `ReferenceOrbit` and `BlaTable` (computed once, shared)
- Delta computation formulas verified against production code in:
  - `fractalwonder-ui/src/workers/perturbation/coordinator.rs:253-262`
  - `fractalwonder-ui/src/rendering/parallel_renderer.rs:411-431`
