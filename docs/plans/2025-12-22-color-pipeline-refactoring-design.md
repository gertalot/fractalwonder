# ColorPipeline Refactoring Design

## Problem Statement

`ParallelRenderer` has three separate code paths for rendering (CPU tiles, GPU tiled, GPU progressive) with colorization logic duplicated and inconsistent across them. The CPU path cannot use histogram caching because callbacks are captured at construction time before the cache exists. The GPU tiled renderer is dead code.

## Goals

1. Unify colorization into a single `ColorPipeline` component used by all render paths
2. Enable histogram caching for CPU path (currently only GPU has this)
3. Delete dead GPU tiled renderer code (~435 lines)
4. Reduce ParallelRenderer field count from 16 to 12

## Non-Goals

- Creating a `ComputeBackend` trait abstraction (over-engineering for two backends)
- Changing WorkerPool's API
- Modifying how GPU or CPU compute works (only colorization changes)

## Design

### New Component: ColorPipeline

**Location:** `src/rendering/colorizers/pipeline.rs`

Groups five scattered colorization fields into one coherent unit:

| Current Field | Type |
|---------------|------|
| `options` | `ColorOptions` |
| `palette` | `Palette` |
| `colorizer` | `ColorizerKind` |
| `cached_context` | `Option<SmoothIterationContext>` |
| `xray_enabled` | `bool` |

**Responsibilities:**

1. **Chunk colorization** - Called during progressive rendering. Uses cached histogram if available, falls back to simple colorization if not.

2. **Final colorization** - Called when render completes. Runs full pipeline (builds histogram, applies shading), caches context for next render.

3. **Option management** - Updates internal state when user changes palette/settings.

4. **Cache invalidation** - Clears histogram cache on navigation (zoom/pan).

**Methods:**

- `new(options: ColorOptions) -> Self`
- `set_options(&mut self, options: ColorOptions)` - Updates options, rebuilds palette
- `set_xray(&mut self, enabled: bool)`
- `colorize_chunk(&self, data: &[ComputeData]) -> Vec<[u8; 4]>` - Uses cached histogram
- `colorize_final(&mut self, data: &[ComputeData], width, height, zoom_level) -> Vec<[u8; 4]>` - Full pipeline, updates cache
- `invalidate_cache(&mut self)` - Clears histogram cache

### ParallelRenderer Changes

**Before (16 fields):**
```
config, worker_pool, progressive_gpu_renderer, gpu_renderer,
gpu_init_attempted, gpu_in_use, options, palette, colorizer,
cached_context, xray_enabled, tile_results, gpu_result_buffer,
canvas_ctx, canvas_size, progress, render_generation, current_viewport
```

**After (12 fields):**
```
config, worker_pool, progressive_gpu_renderer,
gpu_init_attempted, gpu_in_use,
pipeline: Rc<RefCell<ColorPipeline>>,
tile_results, gpu_result_buffer,
canvas_ctx, canvas_size,
progress, render_generation, current_viewport
```

**Changes:**
- 5 colorization fields replaced by 1 `pipeline` field
- Dead `gpu_renderer` field deleted
- `pipeline` is `Rc<RefCell<ColorPipeline>>` so callbacks can share it

### Callback Fix

**Current problem:**
CPU callbacks are closures captured at construction, before `cached_context` exists.

**Solution:**
Pass `Rc<RefCell<ColorPipeline>>` to closures. At call time, borrow the pipeline and access current state including cache.

```
on_tile_complete closure:
  - Captures: Rc<RefCell<ColorPipeline>>
  - At call time: pipeline.borrow().colorize_chunk(data)

on_render_complete closure:
  - Captures: Rc<RefCell<ColorPipeline>>
  - At call time: pipeline.borrow_mut().colorize_final(data, ...)
```

GPU path uses the same pattern - both paths now share identical colorization logic.

### Dead Code Deletion

**parallel_renderer.rs deletions:**

| Location | Description |
|----------|-------------|
| Line 33 | `gpu_renderer` field |
| Lines 64-65 | `gpu_renderer` initialization |
| Line 173 | `gpu_renderer` in struct init |
| Line 305 | `start_gpu_render()` call |
| Lines 321-453 | `start_gpu_render()` function (~130 lines) |
| Lines 593-901 | `schedule_tile()` function (~300 lines) |

**Total:** ~435 lines deleted

**fractalwonder-gpu crate:**
Verify and delete if unused:
- `perturbation_hdr_renderer.rs`
- `perturbation_hdr_pipeline.rs`

## Data Flow

### Progressive Render (same for CPU and GPU)

```
1. User triggers render
   │
   ▼
2. ParallelRenderer.render()
   └─ Selects backend (CPU or GPU)
   │
   ▼
3. Backend produces chunks (tiles or row-sets)
   │
   ▼
4. on_chunk_complete callback
   ├─► pipeline.colorize_chunk(data) - uses cached histogram
   └─► Draw pixels to canvas
   │
   ▼
5. on_render_complete callback
   ├─► pipeline.colorize_final(data) - builds histogram, caches it
   └─► Draw full frame to canvas
```

### Histogram Caching

```
Render N:
  ├─ Chunks → colorize_chunk() uses cache from Render N-1
  └─ Complete → colorize_final() builds new cache

Render N+1:
  ├─ Chunks → colorize_chunk() uses cache from Render N
  └─ Complete → colorize_final() builds new cache

Navigation:
  └─ invalidate_cache() → next render starts without cache
```

## Migration Plan

### Phase 1: Delete Dead Code

1. Remove `gpu_renderer` field and all references
2. Remove `start_gpu_render()` function
3. Remove `schedule_tile()` function
4. Remove unused imports (`GpuPerturbationHDRRenderer`)
5. Check fractalwonder-gpu for orphaned files
6. Verify: `cargo check`, `cargo clippy`, `cargo test`

### Phase 2: Create ColorPipeline

1. Create `src/rendering/colorizers/pipeline.rs`
2. Implement ColorPipeline struct with all methods
3. Export from `colorizers/mod.rs`
4. Add tests for ColorPipeline
5. Verify: `cargo check`, `cargo clippy`, `cargo test`

### Phase 3: Migrate ParallelRenderer

1. Add `pipeline: Rc<RefCell<ColorPipeline>>` field
2. Remove 5 individual colorization fields
3. Update `new()` to create ColorPipeline
4. Update CPU callbacks to use pipeline
5. Update GPU callbacks to use pipeline
6. Update `recolorize()` to use pipeline
7. Update option setters to use pipeline
8. Verify: `cargo check`, `cargo clippy`, `cargo test`

### Phase 4: Verify Behavior

1. Manual test: CPU render with histogram enabled - should use cache during tile arrival
2. Manual test: GPU render with histogram enabled - should use cache during row-set arrival
3. Manual test: Navigation should show consistent coloring during progressive render
4. Manual test: Recolorize should work correctly

## File Changes Summary

| File | Change |
|------|--------|
| `colorizers/pipeline.rs` | **NEW** - ColorPipeline implementation |
| `colorizers/mod.rs` | **MODIFY** - Export ColorPipeline |
| `parallel_renderer.rs` | **MODIFY** - Use ColorPipeline, delete dead code |
| `fractalwonder-gpu/src/perturbation_hdr_renderer.rs` | **DELETE** (if orphaned) |
| `fractalwonder-gpu/src/perturbation_hdr_pipeline.rs` | **DELETE** (if orphaned) |

## Success Criteria

1. CPU progressive rendering uses cached histogram (visible: colors stay consistent as tiles arrive)
2. GPU progressive rendering continues working as before
3. ~435 lines of dead code removed from parallel_renderer.rs
4. ParallelRenderer has 12 fields instead of 16
5. All tests pass
6. No clippy warnings
