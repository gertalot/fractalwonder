# GPU Tiled Progressive Rendering Design

## Problem

At deep zoom levels, the GPU kills its own jobs due to a 2-5 second watchdog timeout (TDR). The current implementation dispatches the entire image in a single compute shader invocation per Adam7 pass. Even with Adam7's pixel subsampling, high iteration counts (100K+) cause timeouts.

## Solution

Replace Adam7 progressive rendering with spatial tiling. Break the image into small tiles (64×64), render each tile as a separate GPU dispatch, display tiles progressively as they complete.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Progressive strategy | Tile-level (ditch Adam7) | At 64×64, Adam7's preview benefit is minimal. Center-out tiles provide "important stuff first" UX. Simpler code. |
| Tile size | Fixed 64×64 | 4K pixels × 100K iterations ≈ 400ms. Safe margin for timeout. Simple, predictable. |
| GPU buffer size | Tile-sized (reused) | Matches production renderers (Fraktaler 3, FractalShark). Small fixed allocation. |
| CPU accumulation | Full-image `Vec<ComputeData>` | Enables re-colorization. Single contiguous buffer vs. many `TileResult` entries. |
| Tile ordering | Center-out | Reuse existing `generate_tiles()`. Best UX - center appears first. |
| Colorization | Quick per-tile, full pipeline at end | Immediate feedback during render, proper effects after completion. |

## Data Flow

```
generate_tiles(width, height, 64) → tiles sorted center-out
    ↓
for each tile:
    GPU dispatch (64×64 buffer) → ComputeData
    Copy into full-image buffer at (tile.x, tile.y)
    Quick colorize tile → RGBA (no pre/post process)
    Draw tile to canvas
    ↓
All tiles done:
    Run full colorizer pipeline on complete buffer
    Redraw entire canvas
```

## Components to Modify

### fractalwonder-gpu

**`perturbation_hdr_renderer.rs`:**
- Change `render()` to accept tile bounds (`PixelRect`) instead of full dimensions
- Allocate fixed 64×64 buffers (or max tile size for edge tiles)
- Return `Vec<ComputeData>` for tile only

**`buffers.rs` (`PerturbationHDRBuffers`):**
- Change to fixed tile-sized allocation (64×64 = 4096 pixels)
- Reuse across tile dispatches

**`shaders/delta_iteration_hdr.wgsl`:**
- Add tile offset uniforms (`tile_offset_x`, `tile_offset_y`)
- Compute global pixel position: `global_x = tile_offset_x + local_x`
- Use global position for δc calculation
- Write to local tile buffer position

**Remove Adam7:**
- Remove `Adam7Pass` from render signature
- Remove `Adam7Accumulator`
- Remove Adam7 filtering logic from shader

### fractalwonder-ui

**`parallel_renderer.rs`:**
- Replace `start_gpu_render()` Adam7 loop with tile loop
- Use `generate_tiles(width, height, 64)` for tile list
- Accumulate results into full-image `Vec<ComputeData>`
- Quick colorize + draw after each tile
- Full colorizer pipeline after all tiles complete

**`tiles.rs`:**
- No changes needed - reuse `generate_tiles()` as-is

## Uniform Changes

Current:
```wgsl
struct Uniforms {
    width: u32,
    height: u32,
    // ... dc_origin, dc_step for full image
}
```

New:
```wgsl
struct Uniforms {
    image_width: u32,      // Full image dimensions (for δc calculation)
    image_height: u32,
    tile_offset_x: u32,    // Tile position in image
    tile_offset_y: u32,
    tile_width: u32,       // Tile dimensions (for bounds check)
    tile_height: u32,
    // ... dc_origin, dc_step unchanged (still for full image)
}
```

## Shader Changes

```wgsl
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) local_id: vec3<u32>) {
    // Bounds check against tile size
    if local_id.x >= uniforms.tile_width || local_id.y >= uniforms.tile_height {
        return;
    }

    // Global pixel position for δc calculation
    let global_x = uniforms.tile_offset_x + local_id.x;
    let global_y = uniforms.tile_offset_y + local_id.y;

    // δc uses global position
    let dc_re = hdr_add(dc_origin_re, hdr_mul(HDRFloat(f32(global_x), 0.0, 0), dc_step_re));
    let dc_im = hdr_add(dc_origin_im, hdr_mul(HDRFloat(f32(global_y), 0.0, 0), dc_step_im));

    // ... iteration loop unchanged ...

    // Write to tile-local buffer position
    let tile_idx = local_id.y * uniforms.tile_width + local_id.x;
    results[tile_idx] = iterations;
    glitch_flags[tile_idx] = select(0u, 1u, glitched);
    z_norm_sq[tile_idx] = z_mag_sq;
}
```

## CPU Accumulation

```rust
// In parallel_renderer.rs
let mut full_image: Vec<ComputeData> = vec![ComputeData::default(); (width * height) as usize];

for tile in tiles {
    let tile_result = gpu_renderer.render_tile(&tile, ...).await?;

    // Copy tile results into full image at correct offset
    for ty in 0..tile.height {
        for tx in 0..tile.width {
            let tile_idx = (ty * tile.width + tx) as usize;
            let image_idx = ((tile.y + ty) * width + (tile.x + tx)) as usize;
            full_image[image_idx] = tile_result.data[tile_idx].clone();
        }
    }

    // Quick colorize and draw tile
    let rgba = quick_colorize(&tile_result.data, &options, &palette);
    draw_tile_to_canvas(&ctx, &rgba, &tile);
}

// Full pipeline after all tiles
let final_rgba = colorizer.run_pipeline(&full_image, &options, &palette, width, height, zoom);
draw_full_frame(&ctx, &final_rgba, width, height);
```

## Glitch Handling

Unchanged. Glitched pixels are marked in `ComputeData.glitched` but no subdivision occurs. This matches current behavior.

## Performance Characteristics

| Image Size | Tiles (64×64) | Dispatches |
|------------|---------------|------------|
| 1920×1080 | 30×17 = 510 | 510 |
| 3840×2160 | 60×34 = 2040 | 2040 |
| 1024×1024 | 16×16 = 256 | 256 |

Each dispatch bounded to ~4K pixels. At 100K iterations, expect ~100-400ms per tile depending on GPU.

## References

- [Fraktaler 3](https://mathr.co.uk/web/fraktaler.html) - tiles computed with OpenCL, downloaded to host, assembled into larger image
- [FractalShark](https://github.com/mattsaccount364/FractalShark) - allocates one large host image, writes each CUDA tile to host RAM
- [TDR (Timeout Detection and Recovery)](https://www.yosoygames.com.ar/wp/2020/04/its-time-we-talk-about-tdr/) - GPU watchdog kills shaders running >2-5 seconds
