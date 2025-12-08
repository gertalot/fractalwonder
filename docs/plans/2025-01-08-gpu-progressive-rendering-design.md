# GPU Progressive Rendering Redesign

## Problem

The current tile-based GPU rendering underutilizes the GPU. Small tiles (32×32 or 256×256) don't provide enough parallel work — the GPU has thousands of cores but we're feeding it hundreds of pixels at a time. This shows up as larger tiles rendering considerably faster than smaller tiles.

Additionally, high iteration counts can cause browser GPU timeouts (2-5 seconds limit), and users see no progress until tiles complete.

## Goals

1. **Progressive rendering** — User sees image appear as stripes, not waiting for full completion
2. **No GPU timeouts** — Chunk iterations to keep each dispatch under timeout threshold
3. **Maximal GPU utilization** — Feed the GPU all pixels in a row-set per dispatch

## Design

### Venetian Blinds Spatial Pattern

Instead of rendering tiles, render horizontal row-sets across the full image width:

```
Row-set 0: rows 0, 16, 32, 48, 64, ...
Row-set 1: rows 1, 17, 33, 49, 65, ...
Row-set 2: rows 2, 18, 34, 50, 66, ...
...
Row-set 15: rows 15, 31, 47, 63, 79, ...
```

Each row-set completes fully (all iteration chunks) before starting the next. User sees stripes appear progressively across the image.

**Why this pattern:**
- At deep zooms, low iteration counts show all-black pixels — interleaving row-sets would show no useful progress
- Completing one row-set fully before moving to the next shows meaningful visual feedback

### Iteration Chunking

For high max_iterations (e.g., 500,000), split into chunks:

```
Dispatch 1: iterations 0 - 100,000
Dispatch 2: iterations 100,000 - 200,000
Dispatch 3: iterations 200,000 - 300,000
...
```

Each dispatch runs for tens of milliseconds, well under the browser's GPU timeout threshold.

### GPU State Persistence

Keep iteration state on GPU between dispatches:

```wgsl
@group(0) @binding(5) var<storage, read_write> z_re: array<f32>;
@group(0) @binding(6) var<storage, read_write> z_im: array<f32>;
@group(0) @binding(7) var<storage, read_write> iter_count: array<u32>;
@group(0) @binding(8) var<storage, read_write> escaped: array<u32>;
```

Shader logic:
1. Check `escaped[idx]` — skip if already escaped
2. Load previous `z_re`, `z_im`, `iter_count`
3. Iterate for `chunk_size` iterations (or until escape)
4. Write updated state back to buffers
5. Only read back to CPU when row-set is fully complete

**Memory estimate:**
- 1920×1080 image, 16 row-sets → 130,560 pixels per row-set
- ~20 bytes per pixel (z_re, z_im, iter, escaped, z_norm_sq)
- ~2.5 MB per row-set, ~40 MB for full image state

### Render Loop

```
for row_set in 0..gpu_progressive_row_sets:
    initialize_state_buffers(row_set)

    chunks = max_iterations / gpu_iterations_per_dispatch
    for chunk in 0..chunks:
        dispatch_compute(row_set, chunk)
        device.poll(Wait)  # Ensure dispatch completes

    read_back_results(row_set)
    colorize_and_display(row_set)  # Immediate canvas update
```

### Configuration

Add to `FractalConfig` in `fractalwonder-ui/src/config.rs`:

```rust
/// Iterations per GPU dispatch (prevents timeout).
pub gpu_iterations_per_dispatch: u32,  // default: 100_000

/// Number of row-sets for progressive rendering (venetian blinds).
pub gpu_progressive_row_sets: u32,     // default: 16
```

### Cancellation

When user pans or zooms mid-render:
- Immediately abort current render loop
- Discard GPU state buffers
- Start fresh render at new viewport

No attempt to reuse partial results or queue renders.

### Glitch Handling

Unchanged from current behavior:
- GPU detects glitches, writes to `glitch_flags` buffer
- CPU reads flags, stores in `glitched_tiles`
- User can manually trigger subdivision (x-ray mode + "d" key)
- No automatic re-render with new references

## Buffer Layout Changes

### Current (tile-based)

```rust
pub struct PerturbationHDRBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,
    pub results: wgpu::Buffer,         // tile_size²
    pub glitch_flags: wgpu::Buffer,    // tile_size²
    pub z_norm_sq: wgpu::Buffer,       // tile_size²
    pub staging_*: wgpu::Buffer,       // for readback
}
```

### New (row-set based)

```rust
pub struct ProgressiveGpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,

    // Persistent state (read-write, not read back until complete)
    pub z_re: wgpu::Buffer,            // row_set_pixels
    pub z_im: wgpu::Buffer,            // row_set_pixels
    pub iter_count: wgpu::Buffer,      // row_set_pixels
    pub escaped: wgpu::Buffer,         // row_set_pixels

    // Results (read back on row-set completion)
    pub results: wgpu::Buffer,         // row_set_pixels
    pub glitch_flags: wgpu::Buffer,    // row_set_pixels
    pub z_norm_sq: wgpu::Buffer,       // row_set_pixels
    pub staging_*: wgpu::Buffer,       // for readback
}
```

### Uniform Changes

```rust
pub struct ProgressiveGpuUniforms {
    // Image dimensions
    pub image_width: u32,
    pub image_height: u32,

    // Row-set info
    pub row_set_index: u32,            // Which row-set (0..15)
    pub row_set_count: u32,            // Total row-sets (16)
    pub row_set_pixel_count: u32,      // Pixels in this row-set

    // Iteration chunking
    pub chunk_start_iter: u32,         // Starting iteration for this dispatch
    pub chunk_size: u32,               // Iterations this dispatch (100,000)
    pub max_iterations: u32,           // Total max iterations

    // Fractal parameters (HDRFloat components)
    pub dc_origin_*: ...,
    pub dc_step_*: ...,
    pub escape_radius_sq: f32,
    pub tau_sq: f32,

    // Reference orbit
    pub reference_escaped: u32,
    pub orbit_len: u32,
}
```

## Shader Changes

### Pixel Indexing

```wgsl
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let linear_idx = id.x;
    if linear_idx >= uniforms.row_set_pixel_count {
        return;
    }

    // Convert linear index to image coordinates
    let row_within_set = linear_idx / uniforms.image_width;
    let col = linear_idx % uniforms.image_width;
    let global_row = row_within_set * uniforms.row_set_count + uniforms.row_set_index;

    // ... rest of iteration logic
}
```

### State Management

```wgsl
// Early exit if already escaped
if escaped[linear_idx] != 0u {
    return;
}

// Load previous state
var z = vec2<f32>(z_re[linear_idx], z_im[linear_idx]);
var n = iter_count[linear_idx];

// Iterate for this chunk
let end_iter = min(n + uniforms.chunk_size, uniforms.max_iterations);
while n < end_iter {
    // ... delta iteration logic ...

    if escape_condition {
        escaped[linear_idx] = 1u;
        results[linear_idx] = n;
        z_norm_sq[linear_idx] = magnitude;
        // Still write z state for consistency
        z_re[linear_idx] = z.x;
        z_im[linear_idx] = z.y;
        iter_count[linear_idx] = n;
        return;
    }
    n++;
}

// Write updated state (not escaped yet)
z_re[linear_idx] = z.x;
z_im[linear_idx] = z.y;
iter_count[linear_idx] = n;
```

## Research References

- [wgpu discussion on long-running shaders](https://github.com/gfx-rs/wgpu/discussions/4988): Browser kills GPU after 2-5 seconds
- [FractalShark](https://github.com/mattsaccount364/FractalShark): CUDA fractal renderer with cooperative groups
- [AMD GPUOpen](https://gpuopen.com/learn/optimizing-gpu-occupancy-resource-usage-large-thread-groups/): Workgroup sizing for variable iteration workloads
- [Compute shader state persistence](https://computergraphics.stackexchange.com/questions/3600/compute-shaders-one-time-only-versus-persistent-buffers): Standard practice for multi-dispatch state

## Implementation Notes

- Workgroup size: Use 64 (1D) instead of 8×8 (2D) since we're processing linear row-sets
- The variable iteration depth per pixel means some threads escape early — 64 is the recommended sweet spot
- Buffer sizes should be calculated as `(image_height / row_set_count) * image_width` rounded up
