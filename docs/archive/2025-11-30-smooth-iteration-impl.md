# Smooth Iteration Count Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate banding in Mandelbrot exterior by implementing smooth iteration count coloring.

**Architecture:** Store `|z|²` at escape in GPU shader, pass through new buffer to MandelbrotData, use smooth formula `μ = n + 1 - log₂(ln(|z|))` in colorizer.

**Tech Stack:** WGSL compute shader, wgpu buffers, Rust f32/f64

**Design doc:** `docs/plans/2025-11-30-smooth-iteration-design.md`

---

### Task 1: Add `final_z_norm_sq` field to MandelbrotData

**Files:**
- Modify: `fractalwonder-core/src/compute_data.rs:39-51`

**Step 1: Add the field**

Add `final_z_norm_sq: f32` to `MandelbrotData`:

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
    /// When true, the colorizer can render this pixel distinctively (e.g., cyan overlay).
    #[serde(default)]
    pub glitched: bool,
    /// |z|² at escape for smooth iteration coloring. Interior points store 0.0.
    #[serde(default)]
    pub final_z_norm_sq: f32,
}
```

**Step 2: Run tests to verify no breakage**

Run: `cargo test -p fractalwonder-core`

Expected: All tests pass (serde default handles missing field in existing data)

**Step 3: Commit**

```bash
git add fractalwonder-core/src/compute_data.rs
git commit -m "feat(core): add final_z_norm_sq to MandelbrotData for smooth coloring"
```

---

### Task 2: Add z_norm_sq buffers to GpuBuffers

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs:49-117`

**Step 1: Add buffer fields to struct**

Add two new fields to `GpuBuffers`:

```rust
pub struct GpuBuffers {
    pub uniforms: wgpu::Buffer,
    pub reference_orbit: wgpu::Buffer,
    pub results: wgpu::Buffer,
    pub glitch_flags: wgpu::Buffer,
    pub staging_results: wgpu::Buffer,
    pub staging_glitches: wgpu::Buffer,
    pub z_norm_sq: wgpu::Buffer,           // NEW
    pub staging_z_norm_sq: wgpu::Buffer,   // NEW
    pub orbit_capacity: u32,
    pub pixel_count: u32,
}
```

**Step 2: Create buffers in GpuBuffers::new()**

Add buffer creation after `staging_glitches`:

```rust
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

**Step 3: Add to struct initialization**

Update the `Self { ... }` return to include new fields:

```rust
        Self {
            uniforms,
            reference_orbit,
            results,
            glitch_flags,
            staging_results,
            staging_glitches,
            z_norm_sq,
            staging_z_norm_sq,
            orbit_capacity: orbit_len,
            pixel_count,
        }
```

**Step 4: Run check to verify compilation**

Run: `cargo check -p fractalwonder-gpu`

Expected: Compiles (may have warnings about unused fields - that's fine)

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): add z_norm_sq buffers for smooth iteration data"
```

---

### Task 3: Add binding 4 to pipeline layout

**Files:**
- Modify: `fractalwonder-gpu/src/pipeline.rs:16-64`

**Step 1: Add binding entry**

Add a 5th entry to the `entries` array in `create_bind_group_layout`:

```rust
                // binding 4: z_norm_sq (read-write storage)
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

**Step 2: Run check**

Run: `cargo check -p fractalwonder-gpu`

Expected: Compiles

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/pipeline.rs
git commit -m "feat(gpu): add binding 4 for z_norm_sq buffer"
```

---

### Task 4: Update WGSL shader

**Files:**
- Modify: `fractalwonder-gpu/src/shaders/delta_iteration.wgsl`

**Step 1: Add binding declaration**

After line 20 (`glitch_flags`), add:

```wgsl
@group(0) @binding(4) var<storage, read_write> z_norm_sq: array<f32>;
```

**Step 2: Store z_sq at escape**

Find the escape check (around line 82-86):

```wgsl
        // Escape check
        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            return;
        }
```

Change to:

```wgsl
        // Escape check
        if z_sq > uniforms.escape_radius_sq {
            results[idx] = n;
            glitch_flags[idx] = select(0u, 1u, glitched);
            z_norm_sq[idx] = z_sq;
            return;
        }
```

**Step 3: Store 0.0 at max iterations**

Find the end of function (around line 117-118):

```wgsl
    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
```

Change to:

```wgsl
    results[idx] = uniforms.max_iterations;
    glitch_flags[idx] = select(0u, 1u, glitched);
    z_norm_sq[idx] = 0.0;
```

**Step 4: Run check**

Run: `cargo check -p fractalwonder-gpu`

Expected: Compiles (shader is embedded at compile time via include_str!)

**Step 5: Commit**

```bash
git add fractalwonder-gpu/src/shaders/delta_iteration.wgsl
git commit -m "feat(gpu): store z_norm_sq at escape in shader"
```

---

### Task 5: Update renderer to use new buffer

**Files:**
- Modify: `fractalwonder-gpu/src/renderer.rs`

**Step 1: Add bind group entry**

In the `render()` method, find the `create_bind_group` call (around line 121-145). Add entry for binding 4:

```rust
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: buffers.z_norm_sq.as_entire_binding(),
                    },
```

**Step 2: Add copy to staging buffer**

After the existing `copy_buffer_to_buffer` calls (around line 169-176), add:

```rust
        encoder.copy_buffer_to_buffer(
            &buffers.z_norm_sq,
            0,
            &buffers.staging_z_norm_sq,
            0,
            (pixel_count * std::mem::size_of::<f32>()) as u64,
        );
```

**Step 3: Add helper method for reading f32 buffer**

Add this method to `impl GpuRenderer` (after `read_buffer`):

```rust
    async fn read_buffer_f32(
        &self,
        buffer: &wgpu::Buffer,
        _count: usize,
    ) -> Result<Vec<f32>, GpuError> {
        let slice = buffer.slice(..);

        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        #[cfg(not(target_arch = "wasm32"))]
        self.context.device.poll(wgpu::Maintain::Wait);

        rx.await
            .map_err(|_| GpuError::Unavailable("Channel closed".into()))?
            .map_err(GpuError::BufferMap)?;

        let data = {
            let view = slice.get_mapped_range();
            bytemuck::cast_slice(&view).to_vec()
        };
        buffer.unmap();

        Ok(data)
    }
```

**Step 4: Read z_norm_sq buffer**

After reading `glitch_data` (around line 186), add:

```rust
        let z_norm_sq_data = self
            .read_buffer_f32(&buffers.staging_z_norm_sq, pixel_count)
            .await?;
```

**Step 5: Update MandelbrotData creation**

Change the data mapping (around line 189-200) to include z_norm_sq:

```rust
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

**Step 6: Run tests**

Run: `cargo test -p fractalwonder-gpu`

Expected: All tests pass

**Step 7: Commit**

```bash
git add fractalwonder-gpu/src/renderer.rs
git commit -m "feat(gpu): read z_norm_sq buffer into MandelbrotData"
```

---

### Task 6: Implement smooth iteration formula in colorizer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs`

**Step 1: Update colorize_mandelbrot to use smooth formula**

Replace the `colorize_mandelbrot` method:

```rust
    fn colorize_mandelbrot(&self, data: &MandelbrotData, palette: &Palette) -> [u8; 4] {
        // Interior points are black
        if !data.escaped {
            return [0, 0, 0, 255];
        }

        // Avoid division by zero
        if data.max_iterations == 0 {
            return [0, 0, 0, 255];
        }

        // Smooth iteration count: μ = n + 1 - log₂(ln(|z|))
        // Since we have |z|²: ln(|z|) = ln(|z|²) / 2
        let smooth = if data.final_z_norm_sq > 1.0 {
            let z_norm_sq = data.final_z_norm_sq as f64;
            let log_z = z_norm_sq.ln() / 2.0;              // ln(|z|)
            let nu = log_z.ln() / std::f64::consts::LN_2; // log₂(ln(|z|))
            data.iterations as f64 + 1.0 - nu
        } else {
            // Fallback for edge cases
            data.iterations as f64
        };

        let t = (smooth / data.max_iterations as f64).clamp(0.0, 1.0);
        let [r, g, b] = palette.sample(t);
        [r, g, b, 255]
    }
```

**Step 2: Run existing tests**

Run: `cargo test -p fractalwonder-ui colorizers`

Expected: All tests pass (existing tests don't depend on smooth formula behavior)

**Step 3: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs
git commit -m "feat(colorizers): implement smooth iteration formula"
```

---

### Task 7: Update test helpers to include final_z_norm_sq

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs` (test section)

**Step 1: Update test helper functions**

Update `make_escaped` to include realistic `final_z_norm_sq`:

```rust
    fn make_escaped(iterations: u32, max_iterations: u32) -> ComputeData {
        // For smooth coloring, we need a realistic |z|² at escape
        // With escape radius 256, |z|² should be > 65536
        // Use a value that gives reasonable smooth offset
        let z_norm_sq = 100000.0_f32; // > 65536, gives smooth adjustment
        ComputeData::Mandelbrot(MandelbrotData {
            iterations,
            max_iterations,
            escaped: true,
            glitched: false,
            final_z_norm_sq: z_norm_sq,
        })
    }

    fn make_interior() -> ComputeData {
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 1000,
            max_iterations: 1000,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 0.0,
        })
    }
```

**Step 2: Add smooth iteration specific test**

Add a test that verifies smooth iteration produces fractional values:

```rust
    #[test]
    fn smooth_iteration_produces_gradual_change() {
        let colorizer = SmoothIterationColorizer;
        let palette = Palette::grayscale();

        // Two pixels with same iteration count but different |z|² at escape
        // should produce different colors due to smooth formula
        let data1 = ComputeData::Mandelbrot(MandelbrotData {
            iterations: 100,
            max_iterations: 1000,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 70000.0, // Just over escape threshold
        });

        let data2 = ComputeData::Mandelbrot(MandelbrotData {
            iterations: 100,
            max_iterations: 1000,
            escaped: true,
            glitched: false,
            final_z_norm_sq: 1000000.0, // Much larger
        });

        let color1 = colorizer.colorize(&data1, &(), &palette);
        let color2 = colorizer.colorize(&data2, &(), &palette);

        // With smooth formula, larger |z|² means lower μ, so darker color
        assert!(
            color1[0] > color2[0],
            "Larger z_norm_sq should produce darker color: {:?} vs {:?}",
            color1,
            color2
        );
    }
```

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui colorizers`

Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/rendering/colorizers/smooth_iteration.rs
git commit -m "test(colorizers): add smooth iteration tests"
```

---

### Task 8: Increase escape radius

**Files:**
- Modify: `fractalwonder-gpu/src/buffers.rs:36`

**Step 1: Update escape_radius_sq**

In `Uniforms::new()`, change:

```rust
            escape_radius_sq: 4.0,
```

To:

```rust
            escape_radius_sq: 65536.0, // 256² for smooth coloring
```

**Step 2: Run all tests**

Run: `cargo test --workspace`

Expected: All tests pass

**Step 3: Commit**

```bash
git add fractalwonder-gpu/src/buffers.rs
git commit -m "feat(gpu): increase escape radius to 256 for smoother gradients"
```

---

### Task 9: Final integration test

**Step 1: Run full test suite**

Run: `cargo test --workspace --all-features`

Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Expected: No warnings

**Step 3: Run fmt check**

Run: `cargo fmt --all -- --check`

Expected: No formatting issues

**Step 4: Visual verification (manual)**

1. Run `trunk serve`
2. Navigate to Mandelbrot view
3. Verify no visible banding in exterior regions
4. Verify interior remains black
5. Verify glitch detection still works (if visible)

**Step 5: Commit any final fixes if needed**

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add field to MandelbrotData | compute_data.rs |
| 2 | Add GPU buffers | buffers.rs |
| 3 | Add pipeline binding | pipeline.rs |
| 4 | Update WGSL shader | delta_iteration.wgsl |
| 5 | Update renderer readback | renderer.rs |
| 6 | Implement smooth formula | smooth_iteration.rs |
| 7 | Update tests | smooth_iteration.rs |
| 8 | Increase escape radius | buffers.rs |
| 9 | Final integration | - |

**Total: 9 tasks, ~8 commits**
