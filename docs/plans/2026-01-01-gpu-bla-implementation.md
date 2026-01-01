# GPU BLA Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Bivariate Linear Approximation (BLA) to the GPU renderer, enabling iteration skipping at deep zoom levels (10^300+).

**Architecture:** CPU computes BLA table from reference orbit, serializes to GPU-friendly format, uploads via new storage buffer (binding 11). Shader performs backward lookup from highest level, applies valid BLA to skip iterations.

**Tech Stack:** Rust (wgpu, bytemuck), WGSL compute shader, existing HDRFloat/HDRComplex infrastructure.

---

## Task 1: Create GpuBlaEntry Data Structure

**Files:**
- Create: `fractalwonder-gpu/src/bla_upload.rs`
- Modify: `fractalwonder-gpu/src/lib.rs`

**Step 1: Write the failing test**

Add to `fractalwonder-gpu/src/bla_upload.rs`:

```rust
//! GPU BLA table serialization.

use bytemuck::{Pod, Zeroable};
use fractalwonder_compute::bla::BlaEntry;

/// GPU-serializable BLA entry (64 bytes, 16 f32-equivalent values).
/// Layout: A (6), B (6), r_sq (3), l (1) = 16 values
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
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

impl GpuBlaEntry {
    /// Convert from CPU BlaEntry to GPU format.
    pub fn from_bla_entry(entry: &BlaEntry) -> Self {
        Self {
            a_re_head: entry.a.re.head,
            a_re_tail: entry.a.re.tail,
            a_re_exp: entry.a.re.exp,
            a_im_head: entry.a.im.head,
            a_im_tail: entry.a.im.tail,
            a_im_exp: entry.a.im.exp,
            b_re_head: entry.b.re.head,
            b_re_tail: entry.b.re.tail,
            b_re_exp: entry.b.re.exp,
            b_im_head: entry.b.im.head,
            b_im_tail: entry.b.im.tail,
            b_im_exp: entry.b.im.exp,
            r_sq_head: entry.r_sq.head,
            r_sq_tail: entry.r_sq.tail,
            r_sq_exp: entry.r_sq.exp,
            l: entry.l,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_bla_entry_size_is_64_bytes() {
        assert_eq!(std::mem::size_of::<GpuBlaEntry>(), 64);
    }

    #[test]
    fn gpu_bla_entry_from_bla_entry_preserves_values() {
        let entry = BlaEntry::from_orbit_point(1.5, 0.5);
        let gpu_entry = GpuBlaEntry::from_bla_entry(&entry);

        assert_eq!(gpu_entry.a_re_head, entry.a.re.head);
        assert_eq!(gpu_entry.a_re_tail, entry.a.re.tail);
        assert_eq!(gpu_entry.a_re_exp, entry.a.re.exp);
        assert_eq!(gpu_entry.a_im_head, entry.a.im.head);
        assert_eq!(gpu_entry.b_re_head, entry.b.re.head);
        assert_eq!(gpu_entry.r_sq_head, entry.r_sq.head);
        assert_eq!(gpu_entry.l, entry.l);
    }
}
```

**Step 2: Run test to verify it compiles and passes**

Run: `cargo test -p fractalwonder-gpu gpu_bla_entry --no-default-features`

Expected: PASS (tests should pass with the implementation above)

**Step 3: Add module to lib.rs**

In `fractalwonder-gpu/src/lib.rs`, add after line 7:

```rust
mod bla_upload;
pub use bla_upload::GpuBlaEntry;
```

**Step 4: Run all GPU tests**

Run: `cargo test -p fractalwonder-gpu --no-default-features`

Expected: All tests pass

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/bla_upload.rs fractalwonder-gpu/src/lib.rs
git commit -m "feat(gpu): add GpuBlaEntry data structure for BLA serialization"
```

---

## Task 2: Add BLA Buffer to ProgressiveGpuBuffers

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs:109-140` (struct definition)
- Modify: `fractalwonder-gpu/src/buffers.rs:145-299` (new function)

**Step 1: Add bla_data field to struct**

In `fractalwonder-gpu/src/buffers.rs`, add after line 127 (after `final_values`):

```rust
    // BLA acceleration data (read-only)
    pub bla_data: wgpu::Buffer,
    pub bla_entry_count: u32,
```

**Step 2: Update new() function signature**

Change the `new` function signature at line 145 to:

```rust
    pub fn new(device: &wgpu::Device, orbit_len: u32, row_set_pixel_count: u32, bla_entry_count: u32) -> Self {
```

**Step 3: Add bla_data buffer creation**

After `sync_staging` buffer creation (around line 277), add:

```rust
        // BLA data: 16 f32s per entry (64 bytes)
        let bla_data = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("progressive_bla_data"),
            size: (bla_entry_count as usize * 16 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
```

**Step 4: Add fields to Self return**

In the `Self { ... }` block (around line 279), add:

```rust
            bla_data,
            bla_entry_count,
```

**Step 5: Run cargo check**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: Errors about callers of `ProgressiveGpuBuffers::new()` needing update

**Step 6: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add bla_data buffer to ProgressiveGpuBuffers"
```

---

## Task 3: Add BLA Fields to Uniforms

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs:8-51` (struct definition)
- Modify: `fractalwonder-gpu/src/buffers.rs:53-104` (new function)

**Step 1: Add BLA fields to ProgressiveGpuUniforms struct**

After line 50 (`pub _pad6: [u32; 2],`), add:

```rust
    // BLA configuration
    pub bla_enabled: u32,
    pub bla_num_levels: u32,
    pub bla_level_offsets: [u32; 32],
```

**Step 2: Update new() function signature**

Add parameters to `new()` function at line 55:

```rust
        bla_enabled: bool,
        bla_num_levels: u32,
        bla_level_offsets: &[usize],
```

**Step 3: Add BLA fields to Self return**

In the `Self { ... }` return block, replace `_pad6: [0, 0],` with:

```rust
            _pad6: [0, 0],
            bla_enabled: if bla_enabled { 1 } else { 0 },
            bla_num_levels,
            bla_level_offsets: {
                let mut offsets = [0u32; 32];
                for (i, &offset) in bla_level_offsets.iter().take(32).enumerate() {
                    offsets[i] = offset as u32;
                }
                offsets
            },
```

**Step 4: Run cargo check**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: Errors about callers needing the new parameters

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add BLA fields to ProgressiveGpuUniforms"
```

---

## Task 4: Add Binding 11 to Pipeline

**Files:**
- Modify: `fractalwonder-gpu/src/progressive_pipeline.rs:21-146`

**Step 1: Add binding 11 entry**

After the binding 10 entry (line 144), add before the closing `]`:

```rust
                // binding 11: bla_data (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 11,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
```

**Step 2: Run cargo check**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: PASS (just adding layout entry)

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/progressive_pipeline.rs
git commit -m "feat(gpu): add binding 11 for BLA data in pipeline layout"
```

---

## Task 5: Update Renderer to Accept and Upload BLA

**Files:**
- Modify: `fractalwonder-gpu/src/progressive_renderer.rs`

**Step 1: Add BLA table caching field**

In `ProgressiveGpuRenderer` struct (around line 17), add:

```rust
    cached_bla_entry_count: u32,
```

**Step 2: Initialize in new()**

In `new()` function, add to `Self { ... }`:

```rust
            cached_bla_entry_count: 0,
```

**Step 3: Add bla_table parameter to render_row_set**

Update `render_row_set` signature to add:

```rust
        bla_table: Option<&fractalwonder_compute::bla::BlaTable>,
```

**Step 4: Update buffer creation check**

Replace the `needs_new_buffers` logic (around line 71-88) with:

```rust
        let bla_entry_count = bla_table.map(|t| t.entries.len() as u32).unwrap_or(0);

        let needs_new_buffers = self.buffers.as_ref().map(|b| b.orbit_capacity).unwrap_or(0)
            < orbit.len() as u32
            || self.cached_row_set_pixel_count < row_set_pixel_count
            || self.cached_bla_entry_count < bla_entry_count;

        if needs_new_buffers {
            log::info!(
                "Creating progressive buffers for orbit len {}, row_set pixels {}, bla entries {}",
                orbit.len(),
                row_set_pixel_count,
                bla_entry_count
            );
            self.buffers = Some(ProgressiveGpuBuffers::new(
                &self.context.device,
                orbit.len() as u32,
                row_set_pixel_count,
                bla_entry_count.max(1), // At least 1 to avoid zero-size buffer
            ));
            self.cached_orbit_id = None;
            self.cached_row_set_pixel_count = row_set_pixel_count;
            self.cached_bla_entry_count = bla_entry_count;
        }
```

**Step 5: Add BLA upload after orbit upload**

After the orbit upload block (around line 135), add:

```rust
        // Upload BLA table if provided and orbit changed
        if self.cached_orbit_id != Some(orbit_id) {
            if let Some(bla) = bla_table {
                let gpu_entries: Vec<crate::GpuBlaEntry> = bla
                    .entries
                    .iter()
                    .map(crate::GpuBlaEntry::from_bla_entry)
                    .collect();
                self.context.queue.write_buffer(
                    &buffers.bla_data,
                    0,
                    bytemuck::cast_slice(&gpu_entries),
                );
            }
        }
```

**Step 6: Update dispatch_chunk call**

Add BLA parameters to `dispatch_chunk` call:

```rust
            self.dispatch_chunk(
                image_width,
                image_height,
                row_set_index,
                row_set_count,
                row_set_pixel_count,
                chunk_start_iter,
                chunk_size,
                max_iterations,
                tau_sq,
                dc_origin,
                dc_step,
                reference_escaped,
                orbit.len() as u32,
                bla_table.is_some(),
                bla_table.map(|t| t.num_levels as u32).unwrap_or(0),
                bla_table.map(|t| &t.level_offsets[..]).unwrap_or(&[]),
            );
```

**Step 7: Run cargo check**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: Errors about dispatch_chunk signature mismatch

**Step 8: Commit**

```bash
git add fractalwonder-gpu/src/progressive_renderer.rs
git commit -m "feat(gpu): add BLA table upload to renderer"
```

---

## Task 6: Update dispatch_chunk for BLA

**Files:**
- Modify: `fractalwonder-gpu/src/progressive_renderer.rs:262-374`

**Step 1: Update dispatch_chunk signature**

Add BLA parameters:

```rust
    fn dispatch_chunk(
        &self,
        image_width: u32,
        image_height: u32,
        row_set_index: u32,
        row_set_count: u32,
        row_set_pixel_count: u32,
        chunk_start_iter: u32,
        chunk_size: u32,
        max_iterations: u32,
        tau_sq: f32,
        dc_origin: ((f32, f32, i32), (f32, f32, i32)),
        dc_step: ((f32, f32, i32), (f32, f32, i32)),
        reference_escaped: bool,
        orbit_len: u32,
        bla_enabled: bool,
        bla_num_levels: u32,
        bla_level_offsets: &[usize],
    ) {
```

**Step 2: Update uniforms creation**

Replace the `ProgressiveGpuUniforms::new` call with:

```rust
        let uniforms = ProgressiveGpuUniforms::new(
            image_width,
            image_height,
            row_set_index,
            row_set_count,
            row_set_pixel_count,
            chunk_start_iter,
            chunk_size,
            max_iterations,
            tau_sq,
            dc_origin,
            dc_step,
            reference_escaped,
            orbit_len,
            bla_enabled,
            bla_num_levels,
            bla_level_offsets,
        );
```

**Step 3: Add binding 11 to bind group entries**

After binding 10 entry (around line 350), add:

```rust
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: buffers.bla_data.as_entire_binding(),
                    },
```

**Step 4: Run cargo check**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: PASS or errors about callers of render_row_set

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/progressive_renderer.rs
git commit -m "feat(gpu): wire BLA parameters through dispatch_chunk"
```

---

## Task 7: Add BLA Shader Structures

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl`

**Step 1: Add BLA struct and buffer binding**

After line 252 (`@group(0) @binding(10) var<storage, read_write> final_values: array<f32>;`), add:

```wgsl
// BLA (Bivariate Linear Approximation) data
// 16 f32s per entry: A (6), B (6), r_sq (3), l (1)
@group(0) @binding(11) var<storage, read> bla_data: array<f32>;

struct BlaEntry {
    a: HDRComplex,
    b: HDRComplex,
    r_sq: HDRFloat,
    l: u32,
}

struct BlaResult {
    valid: bool,
    entry: BlaEntry,
}
```

**Step 2: Add BLA fields to Uniforms struct**

After line 226 (`_pad6b: u32,`), add:

```wgsl
    bla_enabled: u32,
    bla_num_levels: u32,
    bla_level_offsets: array<u32, 32>,
```

**Step 3: Run cargo check to verify WGSL syntax**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: PASS (WGSL is validated at compile time via include_str)

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): add BLA structs and buffer binding to shader"
```

---

## Task 8: Add BLA Load Function

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl`

**Step 1: Add bla_load function**

After the BlaResult struct definition, add:

```wgsl
fn bla_load(idx: u32) -> BlaEntry {
    let base = idx * 16u;
    return BlaEntry(
        HDRComplex(
            HDRFloat(bla_data[base], bla_data[base + 1u], bitcast<i32>(bitcast<u32>(bla_data[base + 2u]))),
            HDRFloat(bla_data[base + 3u], bla_data[base + 4u], bitcast<i32>(bitcast<u32>(bla_data[base + 5u])))
        ),
        HDRComplex(
            HDRFloat(bla_data[base + 6u], bla_data[base + 7u], bitcast<i32>(bitcast<u32>(bla_data[base + 8u]))),
            HDRFloat(bla_data[base + 9u], bla_data[base + 10u], bitcast<i32>(bitcast<u32>(bla_data[base + 11u])))
        ),
        HDRFloat(bla_data[base + 12u], bla_data[base + 13u], bitcast<i32>(bitcast<u32>(bla_data[base + 14u]))),
        bitcast<u32>(bla_data[base + 15u])
    );
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): add bla_load function to shader"
```

---

## Task 9: Add BLA Lookup Function

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl`

**Step 1: Add hdr_complex_mul function (needed for BLA application)**

After the existing HDR functions (around line 150), add:

```wgsl
fn hdr_complex_mul(a: HDRComplex, b: HDRComplex) -> HDRComplex {
    // (a.re + i*a.im) * (b.re + i*b.im) = (a.re*b.re - a.im*b.im) + i*(a.re*b.im + a.im*b.re)
    let re = hdr_sub(hdr_mul(a.re, b.re), hdr_mul(a.im, b.im));
    let im = hdr_add(hdr_mul(a.re, b.im), hdr_mul(a.im, b.re));
    return HDRComplex(re, im);
}

fn hdr_complex_add(a: HDRComplex, b: HDRComplex) -> HDRComplex {
    return HDRComplex(hdr_add(a.re, b.re), hdr_add(a.im, b.im));
}
```

**Step 2: Add bla_find_valid function**

After bla_load, add:

```wgsl
fn bla_find_valid(m: u32, dz_mag_sq: HDRFloat, orbit_len: u32) -> BlaResult {
    let num_levels = uniforms.bla_num_levels;

    // Empty result for early returns
    let empty_entry = BlaEntry(HDR_COMPLEX_ZERO, HDR_COMPLEX_ZERO, HDR_ZERO, 0u);

    if num_levels == 0u {
        return BlaResult(false, empty_entry);
    }

    // Quick reject: if level 0 entry is invalid, all levels are invalid
    let base_entry = bla_load(m);
    if !hdr_less_than(dz_mag_sq, base_entry.r_sq) {
        return BlaResult(false, empty_entry);
    }

    // Search from highest level down
    for (var level = i32(num_levels) - 1; level >= 0; level--) {
        let skip = 1u << u32(level);

        // Alignment: m must be multiple of skip
        if (m % skip) != 0u {
            continue;
        }

        // Bounds: don't skip past orbit end
        if m + skip > orbit_len {
            continue;
        }

        let level_offset = uniforms.bla_level_offsets[level];
        let idx = level_offset + m / skip;
        let entry = bla_load(idx);

        // Validity: |δz|² < r²
        if hdr_less_than(dz_mag_sq, entry.r_sq) {
            return BlaResult(true, entry);
        }
    }

    return BlaResult(false, empty_entry);
}
```

**Step 3: Run cargo check**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: PASS

**Step 4: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): add bla_find_valid lookup function to shader"
```

---

## Task 10: Integrate BLA into Main Iteration Loop

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/progressive_iteration.wgsl`

**Step 1: Add BLA check in iteration loop**

After the rebase check (around line 446, after `continue;`), add the BLA check:

```wgsl
        // BLA acceleration: try to skip multiple iterations
        if uniforms.bla_enabled != 0u {
            let bla = bla_find_valid(m, dz_mag_sq_hdr, orbit_len);

            if bla.valid {
                // Apply: δz_new = A·δz + B·δc
                let a_dz = hdr_complex_mul(bla.entry.a, dz);
                let b_dc = hdr_complex_mul(bla.entry.b, dc);
                dz = hdr_complex_add(a_dz, b_dc);

                // Skip iterations
                m = m + bla.entry.l;
                n = n + bla.entry.l;
                continue;
            }
        }
```

**Step 2: Run cargo check**

Run: `cargo check -p fractalwonder-gpu --no-default-features`

Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/shaders/progressive_iteration.wgsl
git commit -m "feat(gpu): integrate BLA into main iteration loop"
```

---

## Task 11: Fix Upstream Callers

**Files:**
- Search for callers of `render_row_set` and `ProgressiveGpuBuffers::new`

**Step 1: Find all callers**

Run: `grep -r "render_row_set\|ProgressiveGpuBuffers::new" --include="*.rs"`

**Step 2: Update each caller**

For each caller, add the `bla_table: None` parameter (or pass actual BLA table if available).

**Step 3: Run full build**

Run: `cargo build --workspace`

Expected: PASS

**Step 4: Run tests**

Run: `cargo test --workspace`

Expected: All tests pass

**Step 5: Commit**

```bash
git add -A
git commit -m "fix(gpu): update all callers to pass BLA parameters"
```

---

## Task 12: Add Integration Test

**Files:**
- Create: `fractalwonder-gpu/tests/bla_gpu_test.rs`

**Step 1: Write integration test**

```rust
//! GPU BLA integration tests.

#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use fractalwonder_compute::bla::BlaTable;
    use fractalwonder_compute::ReferenceOrbit;
    use fractalwonder_core::{BigFloat, HDRFloat};
    use fractalwonder_gpu::GpuBlaEntry;

    #[test]
    fn bla_table_serializes_to_gpu_format() {
        // Create a simple reference orbit
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);

        // Compute BLA table
        let dc_max = HDRFloat::from_f64(1e-10);
        let bla_table = BlaTable::compute(&orbit, &dc_max);

        // Convert to GPU format
        let gpu_entries: Vec<GpuBlaEntry> = bla_table
            .entries
            .iter()
            .map(GpuBlaEntry::from_bla_entry)
            .collect();

        // Verify we got entries
        assert!(!gpu_entries.is_empty());
        assert_eq!(gpu_entries.len(), bla_table.entries.len());

        // Verify first entry matches
        let first_cpu = &bla_table.entries[0];
        let first_gpu = &gpu_entries[0];
        assert_eq!(first_gpu.l, first_cpu.l);
        assert_eq!(first_gpu.a_re_head, first_cpu.a.re.head);
    }
}
```

**Step 2: Run the test**

Run: `cargo test -p fractalwonder-gpu bla_table_serializes --no-default-features`

Expected: PASS

**Step 3: Commit**

```bash
git add fractalwonder-gpu/tests/bla_gpu_test.rs
git commit -m "test(gpu): add BLA serialization integration test"
```

---

## Task 13: Run Full Test Suite and Quality Checks

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Fix any warnings.

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: All tests pass

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```

---

## Task 14: Final Verification and Summary Commit

**Step 1: Verify build**

Run: `cargo build --workspace --release`

Expected: PASS

**Step 2: Create summary commit**

```bash
git add -A
git commit -m "feat(gpu): complete GPU BLA implementation

Adds Bivariate Linear Approximation to GPU renderer:
- GpuBlaEntry serialization format (64 bytes per entry)
- New storage buffer (binding 11) for BLA data
- Shader functions: bla_load, bla_find_valid
- Integration into main iteration loop
- Full test coverage

BLA enables iteration skipping at deep zoom levels (10^300+),
providing significant performance improvements."
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | GpuBlaEntry data structure | bla_upload.rs, lib.rs |
| 2 | BLA buffer in ProgressiveGpuBuffers | buffers.rs |
| 3 | BLA fields in uniforms | buffers.rs |
| 4 | Binding 11 in pipeline | progressive_pipeline.rs |
| 5 | BLA upload in renderer | progressive_renderer.rs |
| 6 | dispatch_chunk BLA params | progressive_renderer.rs |
| 7 | Shader BLA structs | progressive_iteration.wgsl |
| 8 | Shader bla_load | progressive_iteration.wgsl |
| 9 | Shader bla_find_valid | progressive_iteration.wgsl |
| 10 | Main loop integration | progressive_iteration.wgsl |
| 11 | Fix upstream callers | various |
| 12 | Integration test | tests/bla_gpu_test.rs |
| 13 | Quality checks | - |
| 14 | Final verification | - |

**Estimated commits:** 12-14
**Test coverage:** Unit tests + integration test + existing test suite
