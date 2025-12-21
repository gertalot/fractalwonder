use crate::config::FractalConfig;
use crate::rendering::canvas_utils::{
    draw_full_frame, draw_pixels_to_canvas, get_2d_context, performance_now,
};
use crate::rendering::colorizers::{ColorOptions, ColorPipeline};
use crate::rendering::tiles::{calculate_tile_size, generate_tiles};
use crate::rendering::RenderProgress;
use crate::workers::{OrbitCompleteData, TileResult, WorkerPool};
use fractalwonder_core::{ComputeData, HDRFloat, MandelbrotData, PixelRect, Viewport};
use fractalwonder_gpu::{GpuAvailability, GpuContext, ProgressiveGpuRenderer};
use leptos::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Parallel renderer that distributes tiles across Web Workers.
pub struct ParallelRenderer {
    config: &'static FractalConfig,
    worker_pool: Rc<RefCell<WorkerPool>>,
    progress: RwSignal<RenderProgress>,
    canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>>,
    /// Stored tile results for re-colorizing without recompute
    tile_results: Rc<RefCell<Vec<TileResult>>>,
    /// Progressive GPU renderer for row-set based rendering
    progressive_gpu_renderer: Rc<RefCell<Option<ProgressiveGpuRenderer>>>,
    /// Whether perturbation GPU initialization has been attempted
    gpu_init_attempted: Rc<Cell<bool>>,
    /// Whether GPU is currently executing a render pass (temporarily taken from RefCell)
    gpu_in_use: Rc<Cell<bool>>,
    /// Canvas dimensions for GPU rendering
    canvas_size: Rc<Cell<(u32, u32)>>,
    /// Render generation counter for interruption handling
    render_generation: Rc<Cell<u32>>,
    /// Full-image ComputeData buffer for GPU tile accumulation
    gpu_result_buffer: Rc<RefCell<Vec<ComputeData>>>,
    /// Current viewport for zoom calculation in recolorize
    current_viewport: Rc<RefCell<Option<Viewport>>>,
    /// Unified colorization pipeline
    pipeline: Rc<RefCell<ColorPipeline>>,
}

impl ParallelRenderer {
    pub fn new(config: &'static FractalConfig) -> Result<Self, JsValue> {
        let progress = create_rw_signal(RenderProgress::default());
        let canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>> = Rc::new(RefCell::new(None));
        let tile_results: Rc<RefCell<Vec<TileResult>>> = Rc::new(RefCell::new(Vec::new()));
        let progressive_gpu_renderer: Rc<RefCell<Option<ProgressiveGpuRenderer>>> =
            Rc::new(RefCell::new(None));
        let gpu_init_attempted: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let gpu_in_use: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let canvas_size: Rc<Cell<(u32, u32)>> = Rc::new(Cell::new((0, 0)));
        let render_generation: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let gpu_result_buffer: Rc<RefCell<Vec<ComputeData>>> = Rc::new(RefCell::new(Vec::new()));
        let pipeline = Rc::new(RefCell::new(ColorPipeline::new(ColorOptions::default())));

        let ctx_clone = Rc::clone(&canvas_ctx);
        let results_clone = Rc::clone(&tile_results);
        let pipeline_tile = Rc::clone(&pipeline);
        let on_tile_complete = move |result: TileResult| {
            if let Some(ctx) = ctx_clone.borrow().as_ref() {
                let pipeline = pipeline_tile.borrow();
                let pixels: Vec<u8> = pipeline
                    .colorize_chunk(&result.data)
                    .into_iter()
                    .flatten()
                    .collect();

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

        let worker_pool = WorkerPool::new(config.id, on_tile_complete, progress)?;

        // Set up render complete callback to apply postprocessing (shading) when all tiles done
        let tile_results_complete = Rc::clone(&tile_results);
        let canvas_ctx_complete = Rc::clone(&canvas_ctx);
        let canvas_size_complete = Rc::clone(&canvas_size);
        let current_viewport: Rc<RefCell<Option<Viewport>>> = Rc::new(RefCell::new(None));
        let current_viewport_complete = Rc::clone(&current_viewport);
        let pipeline_complete = Rc::clone(&pipeline);
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
            let final_pixels =
                pipeline.colorize_final(&full_buffer, width as usize, height as usize, zoom_level);

            // Draw full frame
            let pixel_bytes: Vec<u8> = final_pixels.into_iter().flatten().collect();
            let _ = draw_full_frame(ctx, &pixel_bytes, width, height);
        });

        Ok(Self {
            config,
            worker_pool,
            progress,
            canvas_ctx,
            tile_results,
            progressive_gpu_renderer,
            gpu_init_attempted,
            gpu_in_use,
            canvas_size,
            render_generation,
            gpu_result_buffer,
            current_viewport,
            pipeline,
        })
    }

    /// Set x-ray mode enabled state.
    pub fn set_xray_enabled(&self, enabled: bool) {
        self.pipeline.borrow_mut().set_xray(enabled);
    }

    /// Re-colorize all stored tiles using full pipeline (no recompute).
    pub fn recolorize(&self) {
        let ctx_ref = self.canvas_ctx.borrow();
        let Some(ctx) = ctx_ref.as_ref() else {
            return;
        };

        // Compute zoom level from stored viewport
        let zoom_level = if let Some(ref viewport) = *self.current_viewport.borrow() {
            let reference_width = self
                .config
                .default_viewport(viewport.precision_bits())
                .width;
            reference_width.to_f64() / viewport.width.to_f64()
        } else {
            1.0
        };

        // Assemble all tiles into a single full-image buffer for unified histogram
        let (width, height) = self.canvas_size.get();
        let tiles = self.tile_results.borrow();
        let full_buffer = assemble_tiles_to_buffer(&tiles, width as usize, height as usize);

        // Run pipeline on full image (builds fresh histogram, applies shading)
        let mut pipeline = self.pipeline.borrow_mut();
        let final_pixels =
            pipeline.colorize_final(&full_buffer, width as usize, height as usize, zoom_level);

        // Draw full frame
        let pixel_bytes: Vec<u8> = final_pixels.into_iter().flatten().collect();
        let _ = draw_full_frame(ctx, &pixel_bytes, width, height);
    }

    pub fn progress(&self) -> RwSignal<RenderProgress> {
        self.progress
    }

    pub fn cancel(&self) {
        self.worker_pool.borrow_mut().cancel();
    }

    /// Subdivide quadtree cells that contain glitched tiles.
    pub fn subdivide_glitched_cells(&self) {
        self.worker_pool.borrow_mut().subdivide_glitched_cells();
    }

    /// Set color options from UI.
    pub fn set_color_options(&self, new_options: &ColorOptions) {
        self.pipeline.borrow_mut().set_options(new_options.clone());
    }

    pub fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            return;
        }

        // Store canvas size for histogram assembly in callbacks
        self.canvas_size.set((width, height));

        // Clear stored tile results from previous render
        self.tile_results.borrow_mut().clear();

        // Store viewport for zoom calculation in recolorize
        *self.current_viewport.borrow_mut() = Some(viewport.clone());

        // Store canvas context for tile callbacks
        if let Ok(ctx) = get_2d_context(canvas) {
            *self.canvas_ctx.borrow_mut() = Some(ctx);
        }

        // Calculate tile size based on zoom
        let reference_width = self
            .config
            .default_viewport(viewport.precision_bits())
            .width;
        let zoom = reference_width.to_f64() / viewport.width.to_f64();
        let tile_size = calculate_tile_size(zoom);

        // Generate tiles
        let tiles = generate_tiles(width, height, tile_size);

        // Start render with GPU perturbation or CPU fallback
        // Check runtime use_gpu option (user-controllable) AND config gpu_enabled (fractal type)
        let use_gpu = self.config.gpu_enabled && self.pipeline.borrow().options().use_gpu;
        let use_progressive = self.config.gpu_progressive_row_sets > 0;
        if use_gpu && use_progressive {
            // Use progressive GPU rendering (row-sets / venetian blinds pattern)
            log::info!(
                "Using progressive GPU renderer (zoom={zoom:.2e}, row_sets={})",
                self.config.gpu_progressive_row_sets
            );
            self.start_progressive_gpu_render(viewport, canvas);
        } else {
            log::info!("Using CPU renderer (zoom={zoom:.2e})");
            self.worker_pool.borrow_mut().start_perturbation_render(
                viewport.clone(),
                (width, height),
                tiles,
            );
        }
    }

    /// Start progressive GPU render using row-sets (venetian blinds pattern).
    ///
    /// Row-sets render alternating rows across the image, providing visual feedback
    /// that covers the entire canvas from the first row-set.
    fn start_progressive_gpu_render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        // Increment generation to invalidate any in-progress renders
        let gen = self.render_generation.get() + 1;
        self.render_generation.set(gen);

        let row_set_count = self.config.gpu_progressive_row_sets;

        // Initialize progress (row-sets instead of tiles)
        self.progress.set(RenderProgress::new(row_set_count));

        // Initialize full-image result buffer
        *self.gpu_result_buffer.borrow_mut() = vec![
            ComputeData::Mandelbrot(MandelbrotData {
                iterations: 0,
                max_iterations: 0,
                escaped: false,
                glitched: false,
                final_z_norm_sq: 0.0,
                final_z_re: 0.0,
                final_z_im: 0.0,
                final_derivative_re: 0.0,
                final_derivative_im: 0.0,
            });
            (width * height) as usize
        ];

        self.canvas_size.set((width, height));

        // Clone what we need for the callback chain
        let generation = Rc::clone(&self.render_generation);
        let progressive_gpu_renderer = Rc::clone(&self.progressive_gpu_renderer);
        let gpu_init_attempted = Rc::clone(&self.gpu_init_attempted);
        let gpu_in_use = Rc::clone(&self.gpu_in_use);
        let canvas_element = canvas.clone();
        let gpu_result_buffer = Rc::clone(&self.gpu_result_buffer);
        let tile_results = Rc::clone(&self.tile_results);
        let pipeline = Rc::clone(&self.pipeline);
        let progress = self.progress;
        let config = self.config;
        let viewport_clone = viewport.clone();

        // Set up callback for when orbit is ready
        self.worker_pool.borrow().set_orbit_complete_callback(
            move |orbit_data: OrbitCompleteData| {
                log::info!(
                    "Orbit ready: {} points, starting progressive GPU render ({} row-sets)",
                    orbit_data.orbit.len(),
                    row_set_count
                );

                let orbit_data = Rc::new(orbit_data);

                // Start GPU init then first row-set
                let generation = Rc::clone(&generation);
                let progressive_gpu_renderer = Rc::clone(&progressive_gpu_renderer);
                let gpu_init_attempted = Rc::clone(&gpu_init_attempted);
                let gpu_in_use = Rc::clone(&gpu_in_use);
                let canvas_element = canvas_element.clone();
                let gpu_result_buffer = Rc::clone(&gpu_result_buffer);
                let tile_results = Rc::clone(&tile_results);
                let pipeline = Rc::clone(&pipeline);
                let viewport = viewport_clone.clone();
                let orbit_data_clone = Rc::clone(&orbit_data);

                wasm_bindgen_futures::spawn_local(async move {
                    // GPU init
                    if !gpu_init_attempted.get() {
                        gpu_init_attempted.set(true);
                        match GpuContext::try_init().await {
                            GpuAvailability::Available(ctx) => {
                                log::info!("Progressive GPU renderer initialized");
                                *progressive_gpu_renderer.borrow_mut() =
                                    Some(ProgressiveGpuRenderer::new(ctx));
                            }
                            GpuAvailability::Unavailable(reason) => {
                                log::warn!("GPU unavailable: {reason}");
                                return;
                            }
                        }
                    }

                    // Schedule first row-set
                    let render_start_time = performance_now();
                    schedule_row_set(
                        0, // row_set_index
                        gen,
                        width,
                        height,
                        row_set_count,
                        config,
                        generation,
                        progressive_gpu_renderer,
                        gpu_in_use,
                        canvas_element,
                        gpu_result_buffer,
                        tile_results,
                        progress,
                        viewport,
                        orbit_data_clone,
                        render_start_time,
                        pipeline,
                    );
                });
            },
        );

        // Compute orbit for GPU rendering (triggers callback when ready)
        self.worker_pool
            .borrow_mut()
            .compute_orbit_for_gpu(viewport.clone(), (width, height));
    }

    pub fn switch_config(&mut self, config: &'static FractalConfig) -> Result<(), JsValue> {
        self.config = config;
        self.worker_pool.borrow_mut().switch_renderer(config.id);
        Ok(())
    }
}

/// Call requestAnimationFrame and invoke callback when it fires.
fn request_animation_frame_then<F: FnOnce() + 'static>(callback: F) {
    let closure = Closure::once(callback);
    web_sys::window()
        .unwrap()
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Schedule a progressive GPU row-set render with proper browser repaint between row-sets.
#[allow(clippy::too_many_arguments)]
fn schedule_row_set(
    row_set_index: u32,
    expected_gen: u32,
    width: u32,
    height: u32,
    row_set_count: u32,
    config: &'static FractalConfig,
    generation: Rc<Cell<u32>>,
    progressive_gpu_renderer: Rc<RefCell<Option<ProgressiveGpuRenderer>>>,
    gpu_in_use: Rc<Cell<bool>>,
    canvas_element: HtmlCanvasElement,
    gpu_result_buffer: Rc<RefCell<Vec<ComputeData>>>,
    tile_results: Rc<RefCell<Vec<TileResult>>>,
    progress: RwSignal<RenderProgress>,
    viewport: Viewport,
    orbit_data: Rc<OrbitCompleteData>,
    render_start_time: f64,
    pipeline: Rc<RefCell<ColorPipeline>>,
) {
    // Check generation - abort if stale
    if generation.get() != expected_gen {
        log::debug!("Render interrupted at row-set {}", row_set_index);
        return;
    }

    let is_final = row_set_index == row_set_count - 1;

    // Clone for spawn_local
    let generation_spawn = Rc::clone(&generation);
    let progressive_gpu_renderer_spawn = Rc::clone(&progressive_gpu_renderer);
    let gpu_in_use_spawn = Rc::clone(&gpu_in_use);
    let canvas_element_spawn = canvas_element.clone();
    let gpu_result_buffer_spawn = Rc::clone(&gpu_result_buffer);
    let tile_results_spawn = Rc::clone(&tile_results);
    let viewport_spawn = viewport.clone();
    let orbit_data_spawn = Rc::clone(&orbit_data);
    let pipeline_spawn = Rc::clone(&pipeline);

    wasm_bindgen_futures::spawn_local(async move {
        // Convert viewport to HDRFloat format for delta computation
        let vp_width = HDRFloat::from_bigfloat(&viewport_spawn.width);
        let vp_height = HDRFloat::from_bigfloat(&viewport_spawn.height);

        let half = HDRFloat::from_f64(0.5);
        let half_width = vp_width.mul(&half);
        let half_height = vp_height.mul(&half);
        let origin_re = half_width.neg();
        let origin_im = half_height.neg();

        // Use HDRFloat division to preserve extended exponent range at deep zoom
        let step_re = vp_width.div_f64(width as f64);
        let step_im = vp_height.div_f64(height as f64);

        let dc_origin = (
            (origin_re.head, origin_re.tail, origin_re.exp),
            (origin_im.head, origin_im.tail, origin_im.exp),
        );
        let dc_step = (
            (step_re.head, step_re.tail, step_re.exp),
            (step_im.head, step_im.tail, step_im.exp),
        );

        // Debug: log dc values for first row-set only
        if row_set_index == 0 {
            log::info!(
                "Progressive: vp_width: ({}, {}, {}), image: {}x{}, row_sets: {}",
                vp_width.head,
                vp_width.tail,
                vp_width.exp,
                width,
                height,
                row_set_count
            );
        }

        let tau_sq = config.tau_sq as f32;
        let reference_escaped =
            orbit_data_spawn.orbit.len() < orbit_data_spawn.max_iterations as usize;

        // Check if GPU is already in use
        if gpu_in_use_spawn.get() {
            log::debug!(
                "GPU busy, skipping row-set {} (gen {})",
                row_set_index,
                expected_gen
            );
            return;
        }

        // Take renderer
        let renderer_opt = progressive_gpu_renderer_spawn.borrow_mut().take();
        let Some(mut renderer) = renderer_opt else {
            log::debug!(
                "Progressive GPU renderer unavailable for row-set {}",
                row_set_index
            );
            return;
        };

        // Mark GPU in use
        gpu_in_use_spawn.set(true);

        let row_set_result = renderer
            .render_row_set(
                &orbit_data_spawn.orbit,
                &orbit_data_spawn.derivative,
                orbit_data_spawn.orbit_id,
                dc_origin,
                dc_step,
                width,
                height,
                row_set_index,
                row_set_count,
                orbit_data_spawn.max_iterations,
                config.gpu_iterations_per_dispatch,
                tau_sq,
                reference_escaped,
            )
            .await;

        // Return renderer
        match progressive_gpu_renderer_spawn.try_borrow_mut() {
            Ok(mut guard) => {
                *guard = Some(renderer);
            }
            Err(_) => {
                log::warn!("Could not return progressive GPU renderer - RefCell busy");
            }
        }
        gpu_in_use_spawn.set(false);

        // Check generation after await
        if generation_spawn.get() != expected_gen {
            return;
        }

        match row_set_result {
            Ok(result) => {
                log::debug!(
                    "Row-set {}/{}: {} pixels in {:.1}ms",
                    row_set_index + 1,
                    row_set_count,
                    result.data.len(),
                    result.compute_time_ms
                );

                // Copy row-set data into full-image buffer using venetian blind pattern
                // Row-set i contains rows: i, i+row_set_count, i+2*row_set_count, ...
                let rows_per_set = height.div_ceil(row_set_count);
                {
                    let mut buffer = gpu_result_buffer_spawn.borrow_mut();
                    let mut data_idx = 0;
                    for local_row in 0..rows_per_set {
                        let global_row = local_row * row_set_count + row_set_index;
                        if global_row >= height {
                            break;
                        }
                        for col in 0..width {
                            let image_idx = (global_row * width + col) as usize;
                            if data_idx < result.data.len() && image_idx < buffer.len() {
                                buffer[image_idx] = result.data[data_idx].clone();
                            }
                            data_idx += 1;
                        }
                    }
                }

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

                        let _ =
                            draw_pixels_to_canvas(&ctx, &row_pixels, width, 0.0, global_row as f64);
                        data_idx += width as usize;
                    }
                }

                // Update progress
                let elapsed_ms = performance_now() - render_start_time;
                progress.update(|p| {
                    p.completed_steps += 1;
                    p.elapsed_ms = elapsed_ms;
                    p.is_complete = is_final;
                });

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
                } else {
                    // Schedule next row-set via requestAnimationFrame
                    let next_index = row_set_index + 1;
                    request_animation_frame_then(move || {
                        schedule_row_set(
                            next_index,
                            expected_gen,
                            width,
                            height,
                            row_set_count,
                            config,
                            generation_spawn,
                            progressive_gpu_renderer_spawn,
                            gpu_in_use_spawn,
                            canvas_element_spawn,
                            gpu_result_buffer_spawn,
                            tile_results_spawn,
                            progress,
                            viewport_spawn,
                            orbit_data_spawn,
                            render_start_time,
                            pipeline_spawn,
                        );
                    });
                }
            }
            Err(e) => {
                log::error!("Progressive GPU row-set {} failed: {e}", row_set_index);
            }
        }
    });
}

/// Assemble tile results into a single full-image buffer.
/// Tiles may arrive out of order, so we place each tile's data at the correct position.
fn assemble_tiles_to_buffer(tiles: &[TileResult], width: usize, height: usize) -> Vec<ComputeData> {
    // Initialize with default (interior) pixels
    let mut buffer = vec![
        ComputeData::Mandelbrot(MandelbrotData {
            iterations: 0,
            max_iterations: 0,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 0.0,
            final_z_re: 0.0,
            final_z_im: 0.0,
            final_derivative_re: 0.0,
            final_derivative_im: 0.0,
        });
        width * height
    ];

    // Place each tile's data at the correct position
    for tile in tiles {
        let tile_x = tile.tile.x as usize;
        let tile_y = tile.tile.y as usize;
        let tile_width = tile.tile.width as usize;
        let tile_height = tile.tile.height as usize;

        for local_y in 0..tile_height {
            let global_y = tile_y + local_y;
            if global_y >= height {
                break;
            }
            for local_x in 0..tile_width {
                let global_x = tile_x + local_x;
                if global_x >= width {
                    break;
                }
                let local_idx = local_y * tile_width + local_x;
                let global_idx = global_y * width + global_x;
                if local_idx < tile.data.len() {
                    buffer[global_idx] = tile.data[local_idx].clone();
                }
            }
        }
    }

    buffer
}
