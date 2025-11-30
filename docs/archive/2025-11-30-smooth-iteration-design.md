# Smooth Iteration Count Design

**Date:** 2025-11-30
**Status:** Ready for implementation
**Increment:** Coloring Increment 2 (per docs/research/coloring.md)

## Overview

Implement smooth iteration count coloring to eliminate visible banding in the Mandelbrot exterior. This replaces the current linear `iterations / max_iterations` mapping with a continuous formula that produces smooth gradients.

## Mathematical Foundation

The smooth iteration count formula:

```
μ = n + 1 - log₂(ln(|z|))
```

Where:
- `n` = discrete iteration count at escape
- `|z|` = magnitude of z at escape
- Since we store `|z|²`: `ln(|z|) = ln(|z|²) / 2`

This formula produces continuous values that eliminate the discrete banding visible with integer iteration counts.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Storage type | `f32` in MandelbrotData | Matches GPU f32 arithmetic, halves memory/bandwidth |
| Colorizer math | Widen to `f64` | log functions need precision, widening is free |
| Interior points | Store `0.0` | Value unused (colorizer checks `escaped` first) |
| Escape radius | 256² = 65536 | Larger radius produces smoother gradients |

## Data Flow

```
GPU Shader                    Rust Buffers                MandelbrotData
───────────────────────────────────────────────────────────────────────────
z_sq = dot(z, z)    ──►    z_norm_sq: [f32]    ──►    final_z_norm_sq: f32
                           (new buffer)
                                                              │
                                                              ▼
                                                        Colorizer
                                                              │
                                                     (widen to f64)
                                                              │
                                                              ▼
                                                     μ = n + 1 - log₂(ln(|z|))
```

## Implementation Changes

### 1. WGSL Shader (`delta_iteration.wgsl`)

Add new output buffer and store `|z|²` at escape:

```wgsl
// Add binding 4
@group(0) @binding(4) var<storage, read_write> z_norm_sq: array<f32>;

// Update escape radius in Uniforms (or make configurable)
// escape_radius_sq: 65536.0  (was 4.0)

// At escape:
if z_sq > uniforms.escape_radius_sq {
    results[idx] = n;
    glitch_flags[idx] = select(0u, 1u, glitched);
    z_norm_sq[idx] = z_sq;  // Store |z|² for smooth coloring
    return;
}

// At max iterations (interior):
results[idx] = uniforms.max_iterations;
glitch_flags[idx] = select(0u, 1u, glitched);
z_norm_sq[idx] = 0.0;  // Unused by colorizer
```

### 2. GPU Buffers (`buffers.rs`)

Add storage and staging buffers:

```rust
pub struct GpuBuffers {
    // ... existing fields ...
    pub z_norm_sq: wgpu::Buffer,
    pub staging_z_norm_sq: wgpu::Buffer,
}

// In GpuBuffers::new():
let z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("z_norm_sq"),
    size: (pixel_count as usize * std::mem::size_of::<f32>()) as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    mapped_at_creation: false,
});

let staging_z_norm_sq = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("staging_z_norm_sq"),
    size: (pixel_count as usize * std::mem::size_of::<f32>()) as u64,
    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
});
```

### 3. Pipeline (`pipeline.rs`)

Add binding 4 to layout:

```rust
wgpu::BindGroupLayoutEntry {
    binding: 4,
    visibility: wgpu::ShaderStages::COMPUTE,
    ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only: false },
        has_dynamic_offset: false,
        min_binding_size: None,
    },
    count: None,
},
```

### 4. Renderer (`renderer.rs`)

Add bind group entry, copy, and readback:

```rust
// Bind group entry:
wgpu::BindGroupEntry {
    binding: 4,
    resource: buffers.z_norm_sq.as_entire_binding(),
},

// Copy to staging:
encoder.copy_buffer_to_buffer(
    &buffers.z_norm_sq, 0,
    &buffers.staging_z_norm_sq, 0,
    (pixel_count * std::mem::size_of::<f32>()) as u64,
);

// Read back f32 data:
let z_norm_sq_data: Vec<f32> = self
    .read_buffer_f32(&buffers.staging_z_norm_sq, pixel_count)
    .await?;

// Populate MandelbrotData:
let data: Vec<ComputeData> = iterations
    .iter()
    .zip(glitch_data.iter())
    .zip(z_norm_sq_data.iter())
    .map(|((&iter, &glitch_flag), &z_sq)| {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: iter,
            max_iterations,
            escaped: iter < max_iterations && iter != SENTINEL_NOT_COMPUTED,
            glitched: glitch_flag != 0,
            final_z_norm_sq: z_sq,
        })
    })
    .collect();
```

### 5. Compute Data (`compute_data.rs`)

Add field to MandelbrotData:

```rust
pub struct MandelbrotData {
    pub iterations: u32,
    pub max_iterations: u32,
    pub escaped: bool,
    #[serde(default)]
    pub glitched: bool,
    #[serde(default)]
    pub final_z_norm_sq: f32,  // |z|² at escape for smooth coloring
}
```

### 6. Colorizer (`smooth_iteration.rs`)

Implement smooth formula:

```rust
fn colorize_mandelbrot(&self, data: &MandelbrotData, palette: &Palette) -> [u8; 4] {
    if !data.escaped {
        return [0, 0, 0, 255];
    }

    if data.max_iterations == 0 {
        return [0, 0, 0, 255];
    }

    // Smooth iteration: μ = n + 1 - log₂(ln(|z|))
    let smooth = if data.final_z_norm_sq > 1.0 {
        let z_norm_sq = data.final_z_norm_sq as f64;
        let log_z = z_norm_sq.ln() / 2.0;              // ln(|z|)
        let nu = log_z.ln() / std::f64::consts::LN_2; // log₂(ln(|z|))
        data.iterations as f64 + 1.0 - nu
    } else {
        data.iterations as f64
    };

    let t = (smooth / data.max_iterations as f64).clamp(0.0, 1.0);
    let [r, g, b] = palette.sample(t);
    [r, g, b, 255]
}
```

## File Change Summary

| File | Type | Changes |
|------|------|---------|
| `fractalwonder-gpu/src/shaders/delta_iteration.wgsl` | Edit | Add binding 4, store z_sq, update escape radius |
| `fractalwonder-gpu/src/buffers.rs` | Edit | Add z_norm_sq + staging buffers |
| `fractalwonder-gpu/src/pipeline.rs` | Edit | Add binding 4 layout entry |
| `fractalwonder-gpu/src/renderer.rs` | Edit | Add bind group entry, copy, readback, populate field |
| `fractalwonder-core/src/compute_data.rs` | Edit | Add final_z_norm_sq field |
| `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs` | Edit | Implement smooth formula |

## Test Strategy

1. **Continuity**: Adjacent escaped pixels should have μ values differing by < 1.0
2. **Visual**: No visible banding in exterior regions
3. **Interior**: Interior points remain black
4. **Glitched**: Glitched pixels still render cyan in xray mode
5. **Edge cases**: Very low/high iteration counts handled correctly

## Acceptance Criteria

- [ ] No visible banding in exterior regions
- [ ] Interior points still black
- [ ] Glitch detection still works
- [ ] Performance: < 5% overhead (one extra f32 per pixel)
- [ ] All existing tests pass
