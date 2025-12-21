# ColorPipeline Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Unify colorization logic into a single ColorPipeline component, enabling histogram caching for CPU path and deleting ~435 lines of dead code.

**Architecture:** Extract 5 scattered colorization fields from ParallelRenderer into a new ColorPipeline struct. Both CPU and GPU render paths will use this shared pipeline via `Rc<RefCell<ColorPipeline>>`, enabling callbacks to access cached histogram state.

**Tech Stack:** Rust, Leptos, wasm-bindgen

---

## Phase 1: Delete Dead Code

### Task 1.1: Remove Dead GPU Tiled Renderer Field

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Remove gpu_renderer field declaration**

Find and delete line 31-33:
```rust
    /// GPU renderer for perturbation with HDRFloat deltas (tiled mode, currently unused)
    #[allow(dead_code)]
    gpu_renderer: Rc<RefCell<Option<GpuPerturbationHDRRenderer>>>,
```

**Step 2: Remove gpu_renderer initialization**

Find and delete lines 64-65:
```rust
        let gpu_renderer: Rc<RefCell<Option<GpuPerturbationHDRRenderer>>> =
            Rc::new(RefCell::new(None));
```

**Step 3: Remove gpu_renderer from struct initialization**

Find and delete line 173:
```rust
            gpu_renderer,
```

**Step 4: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compilation errors about unused import and missing references

### Task 1.2: Remove start_gpu_render Function and Call Site

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Remove start_gpu_render call**

Find and delete lines 302-305:
```rust
        } else if use_gpu {
            // Use old tiled GPU renderer
            log::info!("Using tiled GPU renderer (zoom={zoom:.2e})");
            self.start_gpu_render(viewport, canvas, tile_size);
```

**Step 2: Remove start_gpu_render function**

Delete the entire function from line ~316 to ~453 (the function marked with `#[allow(dead_code)]` starting with `fn start_gpu_render`).

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Compilation errors about schedule_tile being unused

### Task 1.3: Remove schedule_tile Function

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Remove schedule_tile function**

Delete the entire function from approximately line ~593 to ~901 (the function marked with `#[allow(dead_code)]` starting with `fn schedule_tile`).

**Step 2: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Warnings about unused imports

### Task 1.4: Clean Up Unused Imports

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Remove GpuPerturbationHDRRenderer import**

Find the import line containing `GpuPerturbationHDRRenderer` and remove it from the import list.

**Step 2: Run clippy to find any remaining unused items**

Run: `cargo clippy -p fractalwonder-ui -- -D warnings`
Expected: PASS (or warnings unrelated to this change)

**Step 3: Run tests**

Run: `cargo test -p fractalwonder-ui`
Expected: All tests pass

**Step 4: Commit Phase 1**

```bash
git add -A
git commit -m "refactor: delete dead GPU tiled renderer code

Removes ~435 lines of unused code:
- gpu_renderer field
- start_gpu_render() function
- schedule_tile() function
- GpuPerturbationHDRRenderer import"
```

---

## Phase 2: Create ColorPipeline

### Task 2.1: Create ColorPipeline Module File

**Files:**
- Create: `fractalwonder-ui/src/rendering/colorizers/pipeline.rs`

**Step 1: Create the file with struct definition**

```rust
//! Unified colorization pipeline with histogram caching.

use super::{ColorOptions, ColorizerKind, Palette, SmoothIterationContext};
use fractalwonder_core::ComputeData;

/// Unified colorization pipeline.
///
/// Groups all colorization state into one component that can be shared
/// between CPU and GPU render paths via `Rc<RefCell<ColorPipeline>>`.
pub struct ColorPipeline {
    colorizer: ColorizerKind,
    options: ColorOptions,
    palette: Palette,
    cached_context: Option<SmoothIterationContext>,
    xray_enabled: bool,
}
```

**Step 2: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Warning about unused struct (not yet exported)

### Task 2.2: Implement ColorPipeline Constructor and Accessors

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/pipeline.rs`

**Step 1: Add constructor and basic accessors**

Add after the struct definition:

```rust
impl ColorPipeline {
    /// Create a new pipeline with default colorizer.
    pub fn new(options: ColorOptions) -> Self {
        let palette = options.palette();
        Self {
            colorizer: ColorizerKind::default(),
            options,
            palette,
            cached_context: None,
            xray_enabled: false,
        }
    }

    /// Get current color options.
    pub fn options(&self) -> &ColorOptions {
        &self.options
    }

    /// Update color options. Rebuilds palette cache.
    pub fn set_options(&mut self, options: ColorOptions) {
        self.palette = options.palette();
        self.options = options;
    }

    /// Set xray mode.
    pub fn set_xray(&mut self, enabled: bool) {
        self.xray_enabled = enabled;
    }

    /// Get xray mode.
    pub fn xray_enabled(&self) -> bool {
        self.xray_enabled
    }

    /// Invalidate histogram cache (call on navigation).
    pub fn invalidate_cache(&mut self) {
        self.cached_context = None;
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Warnings about unused methods

### Task 2.3: Implement colorize_chunk Method

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/pipeline.rs`

**Step 1: Add colorize_chunk method**

Add to the impl block:

```rust
    /// Colorize a chunk during progressive rendering.
    ///
    /// Uses cached histogram from previous render if available.
    /// Does NOT update cache - intermediate results shouldn't pollute cache.
    pub fn colorize_chunk(&self, data: &[ComputeData]) -> Vec<[u8; 4]> {
        data.iter()
            .map(|d| {
                // Handle xray mode for glitched pixels
                if self.xray_enabled {
                    if let ComputeData::Mandelbrot(m) = d {
                        if m.glitched {
                            if m.max_iterations == 0 {
                                return [0, 255, 255, 255];
                            }
                            let normalized = m.iterations as f64 / m.max_iterations as f64;
                            let brightness = (64.0 + normalized * 191.0) as u8;
                            return [0, brightness, brightness, 255];
                        }
                    }
                }

                // Use cached context if available, otherwise simple colorization
                if let Some(ref ctx) = self.cached_context {
                    self.colorizer
                        .colorize_with_cached_histogram(d, ctx, &self.options, &self.palette)
                } else {
                    self.colorizer.colorize(d, &self.options, &self.palette)
                }
            })
            .collect()
    }
```

**Step 2: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS

### Task 2.4: Implement colorize_final Method

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/pipeline.rs`

**Step 1: Add colorize_final method**

Add to the impl block:

```rust
    /// Colorize complete frame with full pipeline.
    ///
    /// Builds fresh histogram, applies shading, updates cache for next render.
    pub fn colorize_final(
        &mut self,
        data: &[ComputeData],
        width: usize,
        height: usize,
        zoom_level: f64,
    ) -> Vec<[u8; 4]> {
        // Build new context (histogram) from complete data
        let context = self.colorizer.create_context(data, &self.options);

        // Run full pipeline with new context
        let pixels = self.colorizer.run_pipeline_with_context(
            data,
            &context,
            &self.options,
            &self.palette,
            width,
            height,
            zoom_level,
            self.xray_enabled,
        );

        // Cache context for next progressive render
        self.cached_context = Some(context);

        pixels
    }
```

**Step 2: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS

### Task 2.5: Export ColorPipeline from Module

**Files:**
- Modify: `fractalwonder-ui/src/rendering/colorizers/mod.rs`

**Step 1: Add module declaration**

Add after line 6 (`pub mod smooth_iteration;`):
```rust
pub mod pipeline;
```

**Step 2: Add public export**

Add to the pub use section (around line 14):
```rust
pub use pipeline::ColorPipeline;
```

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS

**Step 4: Run tests**

Run: `cargo test -p fractalwonder-ui`
Expected: All tests pass

**Step 5: Commit Phase 2**

```bash
git add -A
git commit -m "feat: add ColorPipeline for unified colorization

New struct that groups colorization state:
- colorizer, options, palette, cached_context, xray_enabled
- colorize_chunk(): uses cached histogram for progressive rendering
- colorize_final(): full pipeline, updates cache"
```

---

## Phase 3: Migrate ParallelRenderer

### Task 3.1: Add ColorPipeline Field to ParallelRenderer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Add import**

Add to the colorizers import (around line 5-7):
```rust
use crate::rendering::colorizers::{
    colorize_with_palette, ColorOptions, ColorPipeline, ColorizerKind, Palette, SmoothIterationContext,
};
```

**Step 2: Add pipeline field to struct**

Add after the `colorizer` field (around line 51):
```rust
    /// Unified colorization pipeline (replaces options, palette, colorizer, cached_context, xray_enabled)
    pipeline: Rc<RefCell<ColorPipeline>>,
```

**Step 3: Initialize pipeline in new()**

Add after the existing initialization code (around line 78, after `cached_context` init):
```rust
        let pipeline = Rc::new(RefCell::new(ColorPipeline::new(ColorOptions::default())));
```

**Step 4: Add pipeline to struct initialization**

Add to the Ok(Self { ... }) block:
```rust
            pipeline,
```

**Step 5: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Warnings about unused field

### Task 3.2: Update CPU Tile Callback to Use Pipeline

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Clone pipeline for tile callback**

In `new()`, add after the existing clones (around line 86):
```rust
        let pipeline_tile = Rc::clone(&pipeline);
```

**Step 2: Replace tile callback implementation**

Replace the `on_tile_complete` closure (lines ~87-113) with:

```rust
        let on_tile_complete = move |result: TileResult| {
            if let Some(ctx) = ctx_clone.borrow().as_ref() {
                let pipeline = pipeline_tile.borrow();
                let pixels: Vec<u8> = pipeline.colorize_chunk(&result.data).into_iter().flatten().collect();

                // Draw to canvas
                let _ = draw_pixels_to_canvas(
                    ctx,
                    &pixels,
                    result.tile.width,
                    result.tile.x as f64,
                    result.tile.y as f64,
                );

                // Store result for re-colorizing
                results_clone.borrow_mut().push(result);
            }
        };
```

**Step 3: Remove unused clones**

Remove these lines that are no longer needed:
```rust
        let xray_clone = Rc::clone(&xray_enabled);
        let options_clone = Rc::clone(&options);
        let palette_clone = Rc::clone(&palette);
        let colorizer_clone = Rc::clone(&colorizer);
```

**Step 4: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS (with warnings about unused fields)

### Task 3.3: Update CPU Render Complete Callback to Use Pipeline

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Clone pipeline for render complete callback**

Add after existing clones for render complete (around line 118):
```rust
        let pipeline_complete = Rc::clone(&pipeline);
```

**Step 2: Replace render complete callback**

Replace the `set_render_complete_callback` closure (lines ~127-164) with:

```rust
        worker_pool.borrow().set_render_complete_callback(move || {
            let ctx_ref = canvas_ctx_complete.borrow();
            let Some(ctx) = ctx_ref.as_ref() else {
                return;
            };

            // Compute zoom level from stored viewport
            let zoom_level = if let Some(ref viewport) = *current_viewport_complete.borrow() {
                let reference_width = config.default_viewport(viewport.precision_bits()).width;
                reference_width.to_f64() / viewport.width.to_f64()
            } else {
                1.0
            };

            // Assemble all tiles into a single full-image buffer
            let (width, height) = canvas_size_complete.get();
            let tiles = tile_results_complete.borrow();
            let full_buffer = assemble_tiles_to_buffer(&tiles, width as usize, height as usize);

            // Run full pipeline (builds histogram, applies shading, updates cache)
            let mut pipeline = pipeline_complete.borrow_mut();
            let final_pixels = pipeline.colorize_final(
                &full_buffer,
                width as usize,
                height as usize,
                zoom_level,
            );

            // Draw full frame
            let pixel_bytes: Vec<u8> = final_pixels.into_iter().flatten().collect();
            let _ = draw_full_frame(ctx, &pixel_bytes, width, height);
        });
```

**Step 3: Remove unused clones**

Remove these lines:
```rust
        let options_complete = Rc::clone(&options);
        let palette_complete = Rc::clone(&palette);
        let colorizer_complete = Rc::clone(&colorizer);
        let xray_complete = Rc::clone(&xray_enabled);
```

**Step 4: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS

### Task 3.4: Update GPU Progressive Callbacks to Use Pipeline

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Update start_progressive_gpu_render clones**

In `start_progressive_gpu_render()`, replace these clones:
```rust
        let options = Rc::clone(&self.options);
        let palette = Rc::clone(&self.palette);
        let colorizer = Rc::clone(&self.colorizer);
        let cached_context = Rc::clone(&self.cached_context);
```

With:
```rust
        let pipeline = Rc::clone(&self.pipeline);
```

**Step 2: Update orbit callback clones**

In the orbit callback closure, replace:
```rust
                let options = Rc::clone(&options);
                let palette = Rc::clone(&palette);
                let colorizer = Rc::clone(&colorizer);
                let cached_context = Rc::clone(&cached_context);
```

With:
```rust
                let pipeline = Rc::clone(&pipeline);
```

**Step 3: Update schedule_row_set call**

Update the `schedule_row_set` call to pass `pipeline` instead of the four separate parameters.

**Step 4: Update schedule_row_set function signature**

Replace these parameters:
```rust
    options: Rc<RefCell<ColorOptions>>,
    palette: Rc<RefCell<Palette>>,
    colorizer: Rc<RefCell<ColorizerKind>>,
    cached_context: Rc<RefCell<Option<SmoothIterationContext>>>,
```

With:
```rust
    pipeline: Rc<RefCell<ColorPipeline>>,
```

**Step 5: Update schedule_row_set clones**

Replace:
```rust
    let options_spawn = Rc::clone(&options);
    let palette_spawn = Rc::clone(&palette);
    let colorizer_spawn = Rc::clone(&colorizer);
    let cached_context_spawn = Rc::clone(&cached_context);
```

With:
```rust
    let pipeline_spawn = Rc::clone(&pipeline);
```

**Step 6: Update row colorization in schedule_row_set**

Replace the colorization block (lines ~1084-1130) with:

```rust
                // Draw progress: colorize rows using pipeline
                if let Ok(ctx) = get_2d_context(&canvas_element_spawn) {
                    let pipeline = pipeline_spawn.borrow();

                    let mut data_idx = 0;
                    for local_row in 0..rows_per_set {
                        let global_row = local_row * row_set_count + row_set_index;
                        if global_row >= height {
                            break;
                        }

                        let row_end = (data_idx + width as usize).min(result.data.len());
                        let row_pixels: Vec<u8> = pipeline
                            .colorize_chunk(&result.data[data_idx..row_end])
                            .into_iter()
                            .flatten()
                            .collect();

                        let _ = draw_pixels_to_canvas(&ctx, &row_pixels, width, 0.0, global_row as f64);
                        data_idx += width as usize;
                    }
                }
```

**Step 7: Update final colorization in schedule_row_set**

Replace the final colorization block (lines ~1140-1171) with:

```rust
                if is_final {
                    let (final_pixels, full_buffer_clone) = {
                        let full_buffer = gpu_result_buffer_spawn.borrow();
                        let reference_width = config
                            .default_viewport(viewport_spawn.precision_bits())
                            .width;
                        let zoom_level = reference_width.to_f64() / viewport_spawn.width.to_f64();

                        let mut pipeline = pipeline_spawn.borrow_mut();
                        let final_pixels = pipeline.colorize_final(
                            &full_buffer,
                            width as usize,
                            height as usize,
                            zoom_level,
                        );

                        (final_pixels, full_buffer.clone())
                    };

                    // Store for recolorize
                    tile_results_spawn.borrow_mut().clear();
                    tile_results_spawn.borrow_mut().push(TileResult {
                        tile: PixelRect::new(0, 0, width, height),
                        data: full_buffer_clone,
                        compute_time_ms: elapsed_ms,
                    });

                    // Draw final image
                    if let Ok(ctx) = get_2d_context(&canvas_element_spawn) {
                        let pixel_bytes: Vec<u8> = final_pixels.into_iter().flatten().collect();
                        let _ = draw_full_frame(&ctx, &pixel_bytes, width, height);
                    }

                    log::info!(
                        "Progressive render complete: {} row-sets in {:.1}ms",
                        row_set_count,
                        elapsed_ms
                    );
                }
```

**Step 8: Update recursive schedule_row_set call**

Update the recursive call to pass `pipeline_spawn` instead of the four separate parameters.

**Step 9: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS

### Task 3.5: Remove Old Colorization Fields from ParallelRenderer

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Remove old fields from struct**

Remove these fields:
```rust
    options: Rc<RefCell<ColorOptions>>,
    palette: Rc<RefCell<Palette>>,
    colorizer: Rc<RefCell<ColorizerKind>>,
    cached_context: Rc<RefCell<Option<SmoothIterationContext>>>,
    xray_enabled: Rc<Cell<bool>>,
```

**Step 2: Remove old field initializations from new()**

Remove:
```rust
        let default_options = ColorOptions::default();
        let palette: Rc<RefCell<Palette>> = Rc::new(RefCell::new(default_options.palette()));
        let options: Rc<RefCell<ColorOptions>> = Rc::new(RefCell::new(default_options));
        let colorizer: Rc<RefCell<ColorizerKind>> = Rc::new(RefCell::new(ColorizerKind::default()));
        let cached_context: Rc<RefCell<Option<SmoothIterationContext>>> =
            Rc::new(RefCell::new(None));
        let xray_enabled: Rc<Cell<bool>> = Rc::new(Cell::new(false));
```

**Step 3: Remove old fields from struct initialization**

Remove from the Ok(Self { ... }) block:
```rust
            options,
            palette,
            colorizer,
            xray_enabled,
            cached_context,
```

**Step 4: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: Errors about methods using old fields

### Task 3.6: Update ParallelRenderer Methods to Use Pipeline

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Update set_xray_enabled**

Replace:
```rust
    pub fn set_xray_enabled(&self, enabled: bool) {
        self.xray_enabled.set(enabled);
    }
```

With:
```rust
    pub fn set_xray_enabled(&self, enabled: bool) {
        self.pipeline.borrow_mut().set_xray(enabled);
    }
```

**Step 2: Update recolorize method**

Replace the method to use pipeline:
```rust
    pub fn recolorize(&self) {
        let ctx_ref = self.canvas_ctx.borrow();
        let Some(ctx) = ctx_ref.as_ref() else {
            return;
        };

        let zoom_level = if let Some(ref viewport) = *self.current_viewport.borrow() {
            let reference_width = self
                .config
                .default_viewport(viewport.precision_bits())
                .width;
            reference_width.to_f64() / viewport.width.to_f64()
        } else {
            1.0
        };

        let (width, height) = self.canvas_size.get();
        let tiles = self.tile_results.borrow();
        let full_buffer = assemble_tiles_to_buffer(&tiles, width as usize, height as usize);

        let mut pipeline = self.pipeline.borrow_mut();
        let final_pixels = pipeline.colorize_final(
            &full_buffer,
            width as usize,
            height as usize,
            zoom_level,
        );

        let pixel_bytes: Vec<u8> = final_pixels.into_iter().flatten().collect();
        let _ = draw_full_frame(ctx, &pixel_bytes, width, height);
    }
```

**Step 3: Update set_color_options method (if exists)**

Find and update any method that sets color options to use:
```rust
        self.pipeline.borrow_mut().set_options(options);
```

**Step 4: Update any other methods referencing old fields**

Search for remaining uses of `self.options`, `self.palette`, `self.colorizer`, `self.cached_context`, `self.xray_enabled` and update them to use `self.pipeline`.

**Step 5: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS

**Step 6: Run clippy**

Run: `cargo clippy -p fractalwonder-ui -- -D warnings`
Expected: PASS

**Step 7: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 8: Commit Phase 3**

```bash
git add -A
git commit -m "refactor: migrate ParallelRenderer to use ColorPipeline

- Replace 5 colorization fields with single pipeline field
- CPU tile callback now uses pipeline.colorize_chunk()
- CPU render complete uses pipeline.colorize_final()
- GPU progressive uses pipeline for both chunk and final
- Both paths now share histogram cache"
```

---

## Phase 4: Clean Up and Verify

### Task 4.1: Remove Unused Imports

**Files:**
- Modify: `fractalwonder-ui/src/rendering/parallel_renderer.rs`

**Step 1: Run clippy to identify unused imports**

Run: `cargo clippy -p fractalwonder-ui -- -D warnings`

**Step 2: Remove any unused imports flagged**

Common candidates:
- `colorize_with_palette` (if no longer used)
- Individual types now accessed via ColorPipeline

**Step 3: Verify compilation**

Run: `cargo check -p fractalwonder-ui`
Expected: PASS

### Task 4.2: Run Full Test Suite

**Step 1: Format code**

Run: `cargo fmt --all`

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS

**Step 3: Run all tests**

Run: `cargo test --workspace --all-targets --all-features`
Expected: All tests pass

**Step 4: Commit cleanup**

```bash
git add -A
git commit -m "chore: clean up unused imports after refactor"
```

### Task 4.3: Manual Verification

**Step 1: Start dev server**

Verify `trunk serve` is running on localhost:8080.

**Step 2: Test CPU rendering with histogram**

1. Disable GPU in settings
2. Enable histogram coloring
3. Navigate to a new location
4. Observe: Colors should remain consistent as tiles arrive (using cached histogram)

**Step 3: Test GPU rendering with histogram**

1. Enable GPU in settings
2. Enable histogram coloring
3. Navigate to a new location
4. Observe: Colors should remain consistent as row-sets arrive

**Step 4: Test recolorize**

1. Change palette while viewing a render
2. Observe: Image should recolorize with new palette

**Step 5: Document results**

If all tests pass, the refactoring is complete.

---

## Summary

| Phase | Tasks | Lines Changed |
|-------|-------|---------------|
| Phase 1 | Delete dead code | -435 lines |
| Phase 2 | Create ColorPipeline | +120 lines |
| Phase 3 | Migrate ParallelRenderer | Net -50 lines |
| Phase 4 | Clean up and verify | Minor |

**Total:** Net reduction of ~365 lines, unified colorization, histogram caching for CPU path.
