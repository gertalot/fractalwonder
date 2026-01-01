# GPU BLA (Bivariate Linear Approximation) Design

## Overview

Add BLA acceleration to the GPU renderer, enabling iteration skipping during deep zoom
rendering. This matches the CPU's existing BLA capability and provides significant
performance improvements at extreme zoom levels (10^300+).

### Goals

1. **Full HDRFloat precision** - Support 10^2000+ zoom with HDRComplex coefficients
2. **Match CPU behavior** - Same binary tree structure, validity checks, and results
3. **Minimal overhead** - Single buffer upload, one new binding
4. **Clean integration** - Follow existing orbit upload pattern

### Non-Goals

- GPU-side BLA table computation (CPU computes, GPU consumes)
- f32-only fast path (not useful at deep zoom)
- SoA memory layout (access pattern doesn't benefit)

## Research Summary

Professional implementations ([FractalShark](https://github.com/mattsaccount364/FractalShark),
[Fraktaler 3](https://fraktaler.mathr.co.uk/)) use:

- Single flat buffer with all BLA entries contiguous
- Level offsets stored separately (uniforms or small buffer)
- Backward lookup from highest level down
- Early-reject optimization: if level 0 fails, skip all levels

GPU branching concerns are minimal because:
- Adjacent pixels have similar |δz| values, leading to coherent warp behavior
- The lookup is a simple descending loop, not random branching
- Modern GPUs (Volta+) have Independent Thread Scheduling

Sources:
- [Phil Thompson's BLA article](https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html)
- [Claude Heiland-Allen's deep zoom theory](https://mathr.co.uk/blog/2022-02-21_deep_zoom_theory_and_practice_again.html)

## Data Structures

### CPU Side - GpuBlaEntry

```rust
// In fractalwonder-gpu/src/bla_upload.rs

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuBlaEntry {
    // Coefficient A (HDRComplex) - multiplies δz
    pub a_re_head: f32,
    pub a_re_tail: f32,
    pub a_re_exp: i32,
    pub a_im_head: f32,
    pub a_im_tail: f32,
    pub a_im_exp: i32,

    // Coefficient B (HDRComplex) - multiplies δc
    pub b_re_head: f32,
    pub b_re_tail: f32,
    pub b_re_exp: i32,
    pub b_im_head: f32,
    pub b_im_tail: f32,
    pub b_im_exp: i32,

    // Validity radius squared (HDRFloat)
    pub r_sq_head: f32,
    pub r_sq_tail: f32,
    pub r_sq_exp: i32,

    // Iterations to skip
    pub l: u32,
}
// Total: 16 values = 64 bytes per entry
```

### Uniform Extension

```rust
// Added to ProgressiveGpuUniforms
pub bla_enabled: u32,
pub bla_num_levels: u32,
pub bla_level_offsets: [u32; 32],  // Max 32 levels
```

### Memory Layout

```
Buffer: bla_entries (binding 11)
┌─────────────────────────────────────────────────────┐
│ Level 0: entries[0..orbit_len]        (1-skip each) │
│ Level 1: entries[offset_1..offset_2]  (2-skip each) │
│ Level 2: entries[offset_2..offset_3]  (4-skip each) │
│ ...                                                 │
│ Level N: entries[offset_n..end]       (2^N-skip)    │
└─────────────────────────────────────────────────────┘
Total size: ~2 × orbit_len entries (binary tree property)
```

## Shader Implementation

### BLA Load Function

```wgsl
fn bla_load(idx: u32) -> BlaEntry {
    let base = idx * 16u;
    return BlaEntry(
        HDRComplex(
            HDRFloat(bla_data[base], bla_data[base+1u], bitcast<i32>(bla_data[base+2u])),
            HDRFloat(bla_data[base+3u], bla_data[base+4u], bitcast<i32>(bla_data[base+5u]))
        ),
        HDRComplex(
            HDRFloat(bla_data[base+6u], bla_data[base+7u], bitcast<i32>(bla_data[base+8u])),
            HDRFloat(bla_data[base+9u], bla_data[base+10u], bitcast<i32>(bla_data[base+11u]))
        ),
        HDRFloat(bla_data[base+12u], bla_data[base+13u], bitcast<i32>(bla_data[base+14u])),
        bitcast<u32>(bla_data[base+15u])
    );
}
```

### BLA Lookup Function

```wgsl
fn bla_find_valid(m: u32, dz_mag_sq: HDRFloat) -> BlaResult {
    // Quick reject: if level 0 fails, all levels fail
    let base_entry = bla_load(m);
    if hdr_greater_equal(dz_mag_sq, base_entry.r_sq) {
        return BlaResult(false, base_entry);
    }

    // Search from highest level down
    for (var level = i32(uniforms.bla_num_levels) - 1; level >= 0; level--) {
        let skip = 1u << u32(level);

        // Alignment: m must be multiple of skip
        if (m % skip) != 0u { continue; }

        // Bounds: don't skip past orbit end
        if m + skip > uniforms.orbit_len { continue; }

        let idx = uniforms.bla_level_offsets[level] + m / skip;
        let entry = bla_load(idx);

        // Validity: |δz|² < r²
        if hdr_less_than(dz_mag_sq, entry.r_sq) {
            return BlaResult(true, entry);
        }
    }

    return BlaResult(false, base_entry);
}
```

### Integration in Main Loop

```wgsl
if uniforms.bla_enabled != 0u {
    let dz_mag_sq = hdr_complex_norm_sq_hdr(dz);
    let bla = bla_find_valid(m, dz_mag_sq);

    if bla.valid {
        // Apply: δz_new = A·δz + B·δc
        let a_dz = hdr_complex_mul(bla.entry.a, dz);
        let b_dc = hdr_complex_mul(bla.entry.b, delta_c);
        dz = hdr_complex_add(a_dz, b_dc);

        m = m + bla.entry.l;
        n = n + bla.entry.l;
        continue;
    }
}
// ... normal delta iteration
```

## Rust Integration

### Buffer Creation

```rust
// In fractalwonder-gpu/src/buffers.rs
let bla_data = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("progressive_bla_data"),
    size: (max_bla_entries * std::mem::size_of::<GpuBlaEntry>()) as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
});
```

### Upload Function

```rust
// In fractalwonder-gpu/src/progressive_renderer.rs
pub fn upload_bla_table(&self, context: &GpuContext, bla_table: &BlaTable) {
    let gpu_entries: Vec<GpuBlaEntry> = bla_table
        .entries
        .iter()
        .map(GpuBlaEntry::from_bla_entry)
        .collect();

    context.queue.write_buffer(
        &self.buffers.bla_data,
        0,
        bytemuck::cast_slice(&gpu_entries),
    );
}
```

## File Changes

| File | Change |
|------|--------|
| `fractalwonder-gpu/src/bla_upload.rs` | **New** - GpuBlaEntry, serialization |
| `fractalwonder-gpu/src/buffers.rs` | Add `bla_data` buffer |
| `fractalwonder-gpu/src/progressive_pipeline.rs` | Add binding 11 |
| `fractalwonder-gpu/src/progressive_renderer.rs` | Add `upload_bla_table()` |
| `fractalwonder-gpu/src/uniforms.rs` | Add BLA fields |
| `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl` | BLA lookup + application |
| `fractalwonder-gpu/src/lib.rs` | Export new module |

## Implementation Order

1. Data structures - `GpuBlaEntry` and serialization
2. Buffer + binding - Add storage buffer, pipeline binding
3. Uniforms - Add BLA metadata fields
4. Upload logic - `upload_bla_table()` function
5. Shader: structs - BLA entry struct, load function
6. Shader: lookup - `bla_find_valid()` function
7. Shader: integration - Wire into main iteration loop
8. Tests - Unit, integration, visual regression
9. Benchmarks - Verify speedup

## Testing Strategy

### Unit Tests

- Verify `GpuBlaEntry` is 64 bytes (aligned)
- Verify CPU→GPU serialization preserves values

### Integration Tests

- Render same tile with CPU BLA and GPU BLA, compare iteration counts
- Render with GPU BLA enabled vs disabled, verify identical results

### Performance Benchmarks

- Measure speedup at 10^-300 zoom (expect 2-10x improvement)

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Shader divergence slowdown | Early-reject check minimizes branching |
| Memory overflow for huge orbits | Cap BLA levels, warn user |
| Precision mismatch vs CPU | Identical HDRFloat ops, thorough testing |
| Buffer binding limit | Currently 12/16, headroom available |
