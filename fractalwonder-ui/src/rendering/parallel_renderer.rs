use crate::config::{FractalConfig, RendererType};
use crate::rendering::canvas_utils::{
    draw_full_frame, draw_pixels_to_canvas, get_2d_context, performance_now,
};
use crate::rendering::colorizers::{
    colorize, colorize_with_palette, presets, ColorizerKind, ColorSchemePreset, Palette,
};
use crate::rendering::tiles::{calculate_tile_size, generate_tiles};
use crate::rendering::RenderProgress;
use crate::workers::{OrbitCompleteData, TileResult, WorkerPool};
use fractalwonder_core::{PixelRect, Viewport};
use fractalwonder_gpu::{Adam7Accumulator, Adam7Pass, GpuAvailability, GpuContext, GpuRenderer};
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
    xray_enabled: Rc<Cell<bool>>,
    /// Stored tile results for re-colorizing without recompute
    tile_results: Rc<RefCell<Vec<TileResult>>>,
    /// GPU renderer, lazily initialized when gpu_enabled
    gpu_renderer: Rc<RefCell<Option<GpuRenderer>>>,
    /// Whether GPU initialization has been attempted
    gpu_init_attempted: Rc<Cell<bool>>,
    /// Whether GPU is currently executing a render pass (temporarily taken from RefCell)
    gpu_in_use: Rc<Cell<bool>>,
    /// Canvas dimensions for GPU rendering
    canvas_size: Rc<Cell<(u32, u32)>>,
    /// Render generation counter for interruption handling
    render_generation: Rc<Cell<u32>>,
    /// Adam7 accumulator for progressive rendering
    adam7_accumulator: Rc<RefCell<Option<Adam7Accumulator>>>,
    /// Current palette for colorization
    palette: Rc<RefCell<Palette>>,
    /// Current colorizer algorithm
    colorizer: Rc<RefCell<ColorizerKind>>,
}

impl ParallelRenderer {
    pub fn new(config: &'static FractalConfig) -> Result<Self, JsValue> {
        let progress = create_rw_signal(RenderProgress::default());
        let canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>> = Rc::new(RefCell::new(None));
        let xray_enabled: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let tile_results: Rc<RefCell<Vec<TileResult>>> = Rc::new(RefCell::new(Vec::new()));
        let gpu_renderer: Rc<RefCell<Option<GpuRenderer>>> = Rc::new(RefCell::new(None));
        let gpu_init_attempted: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let gpu_in_use: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let canvas_size: Rc<Cell<(u32, u32)>> = Rc::new(Cell::new((0, 0)));
        let render_generation: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let adam7_accumulator: Rc<RefCell<Option<Adam7Accumulator>>> = Rc::new(RefCell::new(None));

        // Initialize with default color scheme (Classic preset)
        let default_preset = &presets()[0];
        let palette: Rc<RefCell<Palette>> = Rc::new(RefCell::new(default_preset.palette.clone()));
        let colorizer: Rc<RefCell<ColorizerKind>> =
            Rc::new(RefCell::new(default_preset.colorizer.clone()));

        let ctx_clone = Rc::clone(&canvas_ctx);
        let xray_clone = Rc::clone(&xray_enabled);
        let results_clone = Rc::clone(&tile_results);
        let palette_clone = Rc::clone(&palette);
        let colorizer_clone = Rc::clone(&colorizer);
        let on_tile_complete = move |result: TileResult| {
            if let Some(ctx) = ctx_clone.borrow().as_ref() {
                // Colorize with current palette, colorizer, and xray state
                let xray = xray_clone.get();
                let pal = palette_clone.borrow();
                let col = colorizer_clone.borrow();
                let pixels: Vec<u8> = result
                    .data
                    .iter()
                    .flat_map(|d| colorize_with_palette(d, &pal, &col, xray))
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

        Ok(Self {
            config,
            worker_pool,
            progress,
            canvas_ctx,
            xray_enabled,
            tile_results,
            gpu_renderer,
            gpu_init_attempted,
            gpu_in_use,
            canvas_size,
            render_generation,
            adam7_accumulator,
            palette,
            colorizer,
        })
    }

    /// Set x-ray mode enabled state.
    pub fn set_xray_enabled(&self, enabled: bool) {
        self.xray_enabled.set(enabled);
    }

    /// Re-colorize all stored tiles with current xray state (no recompute).
    pub fn recolorize(&self) {
        let xray = self.xray_enabled.get();
        let palette = self.palette.borrow();
        let colorizer = self.colorizer.borrow();
        let ctx_ref = self.canvas_ctx.borrow();
        let Some(ctx) = ctx_ref.as_ref() else {
            return;
        };

        for result in self.tile_results.borrow().iter() {
            let pixels: Vec<u8> = result
                .data
                .iter()
                .flat_map(|d| colorize_with_palette(d, &palette, &colorizer, xray))
                .collect();
            let _ = draw_pixels_to_canvas(
                ctx,
                &pixels,
                result.tile.width,
                result.tile.x as f64,
                result.tile.y as f64,
            );
        }
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

    /// Set the color scheme (palette and colorizer).
    pub fn set_color_scheme(&self, preset: &ColorSchemePreset) {
        *self.palette.borrow_mut() = preset.palette.clone();
        *self.colorizer.borrow_mut() = preset.colorizer.clone();
    }

    /// Get available color scheme presets.
    pub fn color_scheme_presets(&self) -> Vec<ColorSchemePreset> {
        presets()
    }

    pub fn render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            return;
        }

        // Clear stored tile results from previous render
        self.tile_results.borrow_mut().clear();

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

        // Start render with appropriate method based on renderer type
        match self.config.renderer_type {
            RendererType::Simple => {
                self.worker_pool.borrow_mut().start_render(
                    viewport.clone(),
                    (width, height),
                    tiles,
                );
            }
            RendererType::Perturbation => {
                if self.config.gpu_enabled {
                    self.start_gpu_render(viewport, canvas);
                } else {
                    self.worker_pool.borrow_mut().start_perturbation_render(
                        viewport.clone(),
                        (width, height),
                        tiles,
                    );
                }
            }
        }
    }

    /// Start GPU-accelerated perturbation render with progressive passes.
    ///
    /// Uses callback-based setTimeout to create macrotask boundaries between passes,
    /// allowing the browser to repaint between each progressive resolution level.
    fn start_gpu_render(&self, viewport: &Viewport, canvas: &HtmlCanvasElement) {
        let width = canvas.width();
        let height = canvas.height();

        // Increment generation to invalidate any in-progress renders
        let gen = self.render_generation.get() + 1;
        self.render_generation.set(gen);

        // Initialize progress for GPU passes (7 Adam7 passes)
        let total_passes = Adam7Pass::all().len() as u32;
        self.progress.set(RenderProgress::new(total_passes));

        // Initialize accumulator for this render
        *self.adam7_accumulator.borrow_mut() = Some(Adam7Accumulator::new(width, height));

        self.canvas_size.set((width, height));

        // Clone what we need for the callback chain
        let generation = Rc::clone(&self.render_generation);
        let gpu_renderer = Rc::clone(&self.gpu_renderer);
        let gpu_init_attempted = Rc::clone(&self.gpu_init_attempted);
        let gpu_in_use = Rc::clone(&self.gpu_in_use);
        let canvas_element = canvas.clone();
        let xray_enabled = Rc::clone(&self.xray_enabled);
        let tile_results = Rc::clone(&self.tile_results);
        let worker_pool = Rc::clone(&self.worker_pool);
        let adam7_accumulator = Rc::clone(&self.adam7_accumulator);
        let progress = self.progress;
        let config = self.config;
        let viewport_clone = viewport.clone();
        let tiles = generate_tiles(width, height, calculate_tile_size(1.0));

        // Set up callback for when orbit is ready
        self.worker_pool.borrow().set_orbit_complete_callback(
            move |orbit_data: OrbitCompleteData| {
                log::info!(
                    "Orbit ready: {} points, starting progressive GPU render",
                    orbit_data.orbit.len()
                );

                // Wrap orbit data in Rc for sharing across pass callbacks
                let orbit_data = Rc::new(orbit_data);

                // Clone for GPU init
                let generation = Rc::clone(&generation);
                let gpu_renderer = Rc::clone(&gpu_renderer);
                let gpu_init_attempted = Rc::clone(&gpu_init_attempted);
                let gpu_in_use = Rc::clone(&gpu_in_use);
                let canvas_element = canvas_element.clone();
                let xray_enabled = Rc::clone(&xray_enabled);
                let tile_results = Rc::clone(&tile_results);
                let worker_pool = Rc::clone(&worker_pool);
                let adam7_accumulator = Rc::clone(&adam7_accumulator);
                let viewport = viewport_clone.clone();
                let tiles = tiles.clone();
                let orbit_data_init = Rc::clone(&orbit_data);

                // First spawn_local: GPU init, then schedule first pass via macrotask
                wasm_bindgen_futures::spawn_local(async move {
                    // Try GPU init if not attempted
                    if !gpu_init_attempted.get() {
                        gpu_init_attempted.set(true);
                        match GpuContext::try_init().await {
                            GpuAvailability::Available(ctx) => {
                                log::info!("GPU renderer initialized");
                                *gpu_renderer.borrow_mut() = Some(GpuRenderer::new(ctx));
                            }
                            GpuAvailability::Unavailable(reason) => {
                                log::warn!("GPU unavailable: {reason}");
                            }
                        }
                    }

                    // Check if we have GPU available
                    // If GPU is temporarily in use by a stale render, wait for it
                    let gpu_available = gpu_renderer.borrow().is_some();
                    let gpu_busy = gpu_in_use.get();

                    if !gpu_available && !gpu_busy {
                        // GPU truly unavailable (init failed or not supported)
                        log::info!("No GPU available, using CPU");
                        worker_pool.borrow_mut().start_perturbation_render(
                            viewport,
                            (width, height),
                            tiles,
                        );
                        return;
                    }

                    if !gpu_available && gpu_busy {
                        // GPU is temporarily taken out by a stale render pass
                        // The stale pass will abort via generation check and return it shortly
                        // Schedule a retry after a short delay
                        log::info!("GPU temporarily busy, waiting...");
                        let gpu_renderer = Rc::clone(&gpu_renderer);
                        let gpu_in_use = Rc::clone(&gpu_in_use);
                        let generation = Rc::clone(&generation);
                        let canvas_element = canvas_element.clone();
                        let xray_enabled = Rc::clone(&xray_enabled);
                        let tile_results = Rc::clone(&tile_results);
                        let worker_pool = Rc::clone(&worker_pool);
                        let adam7_accumulator = Rc::clone(&adam7_accumulator);
                        let orbit_data = Rc::clone(&orbit_data_init);

                        // Use requestAnimationFrame to retry
                        request_animation_frame_then(move || {
                            // Check generation - might have been superseded
                            if generation.get() != gen {
                                return;
                            }
                            // Check if GPU is now available
                            if gpu_renderer.borrow().is_some() {
                                let render_start_time = performance_now();
                                schedule_adam7_pass(
                                    Adam7Pass::all()[0],
                                    0,
                                    gen,
                                    width,
                                    height,
                                    config,
                                    generation,
                                    gpu_renderer,
                                    canvas_element,
                                    xray_enabled,
                                    tile_results,
                                    worker_pool,
                                    progress,
                                    viewport,
                                    tiles,
                                    orbit_data,
                                    render_start_time,
                                    gpu_in_use,
                                    adam7_accumulator,
                                );
                            } else {
                                // Still not available, fall back to CPU
                                log::warn!("GPU still unavailable after wait, using CPU");
                                worker_pool.borrow_mut().start_perturbation_render(
                                    viewport,
                                    (width, height),
                                    tiles,
                                );
                            }
                        });
                        return;
                    }

                    // Schedule first pass via setTimeout (macrotask boundary)
                    // This ends the current spawn_local, allowing browser to repaint
                    let passes = Adam7Pass::all();
                    let render_start_time = performance_now();
                    schedule_adam7_pass(
                        passes[0],
                        0,
                        gen,
                        width,
                        height,
                        config,
                        Rc::clone(&generation),
                        Rc::clone(&gpu_renderer),
                        canvas_element.clone(),
                        Rc::clone(&xray_enabled),
                        Rc::clone(&tile_results),
                        Rc::clone(&worker_pool),
                        progress,
                        viewport.clone(),
                        tiles.clone(),
                        Rc::clone(&orbit_data_init),
                        render_start_time,
                        Rc::clone(&gpu_in_use),
                        Rc::clone(&adam7_accumulator),
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

/// Schedule an Adam7 pass with proper browser repaint between passes.
#[allow(clippy::too_many_arguments)]
fn schedule_adam7_pass(
    pass: Adam7Pass,
    pass_index: usize,
    expected_gen: u32,
    width: u32,
    height: u32,
    config: &'static FractalConfig,
    generation: Rc<Cell<u32>>,
    gpu_renderer: Rc<RefCell<Option<GpuRenderer>>>,
    canvas_element: HtmlCanvasElement,
    xray_enabled: Rc<Cell<bool>>,
    tile_results: Rc<RefCell<Vec<TileResult>>>,
    worker_pool: Rc<RefCell<WorkerPool>>,
    progress: RwSignal<RenderProgress>,
    viewport: Viewport,
    tiles: Vec<PixelRect>,
    orbit_data: Rc<OrbitCompleteData>,
    render_start_time: f64,
    gpu_in_use: Rc<Cell<bool>>,
    adam7_accumulator: Rc<RefCell<Option<Adam7Accumulator>>>,
) {
    log::info!("Scheduling Adam7 pass {}", pass.step());

    // Check generation - abort if stale
    if generation.get() != expected_gen {
        log::debug!("Render interrupted at Adam7 pass {}", pass.step());
        return;
    }

    // Clone for spawn_local
    let generation_spawn = Rc::clone(&generation);
    let gpu_renderer_spawn = Rc::clone(&gpu_renderer);
    let gpu_in_use_spawn = Rc::clone(&gpu_in_use);
    let canvas_element_spawn = canvas_element.clone();
    let xray_enabled_spawn = Rc::clone(&xray_enabled);
    let tile_results_spawn = Rc::clone(&tile_results);
    let worker_pool_spawn = Rc::clone(&worker_pool);
    let viewport_spawn = viewport.clone();
    let tiles_spawn = tiles.clone();
    let orbit_data_spawn = Rc::clone(&orbit_data);
    let adam7_accumulator_spawn = Rc::clone(&adam7_accumulator);

    wasm_bindgen_futures::spawn_local(async move {
        let vp_width = viewport_spawn.width.to_f64() as f32;
        let vp_height = viewport_spawn.height.to_f64() as f32;
        let dc_origin = (-vp_width / 2.0, -vp_height / 2.0);
        let dc_step = (vp_width / width as f32, vp_height / height as f32);
        let tau_sq = config.tau_sq as f32;

        // Mark GPU as in use
        gpu_in_use_spawn.set(true);

        // Take renderer temporarily
        let mut renderer = gpu_renderer_spawn.borrow_mut().take().unwrap();
        let pass_result = renderer
            .render(
                &orbit_data_spawn.orbit,
                orbit_data_spawn.orbit_id,
                dc_origin,
                dc_step,
                width,
                height,
                orbit_data_spawn.max_iterations,
                tau_sq,
                pass,
            )
            .await;

        // Put renderer back
        *gpu_renderer_spawn.borrow_mut() = Some(renderer);
        gpu_in_use_spawn.set(false);

        match pass_result {
            Ok(result) => {
                log::info!(
                    "Adam7 pass {}: {:.1}ms",
                    pass.step(),
                    result.compute_time_ms
                );

                // Merge into accumulator
                if let Some(ref mut acc) = *adam7_accumulator_spawn.borrow_mut() {
                    acc.merge(&result.data);

                    // Get display buffer (with gaps filled)
                    let display_data = if pass.is_final() {
                        acc.to_final_buffer()
                    } else {
                        acc.to_display_buffer()
                    };

                    // Store for recolorize (update with latest)
                    tile_results_spawn.borrow_mut().clear();
                    tile_results_spawn.borrow_mut().push(TileResult {
                        tile: PixelRect {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        },
                        data: display_data.clone(),
                        compute_time_ms: result.compute_time_ms,
                    });

                    // Colorize and draw
                    let xray = xray_enabled_spawn.get();
                    let pixels: Vec<u8> = display_data
                        .iter()
                        .flat_map(|d| colorize(d, xray))
                        .collect();

                    if let Ok(ctx) = get_2d_context(&canvas_element_spawn) {
                        match draw_full_frame(&ctx, &pixels, width, height) {
                            Ok(()) => {
                                log::info!("Drew Adam7 pass {} to canvas", pass.step())
                            }
                            Err(e) => {
                                log::error!("Draw failed for Adam7 pass {}: {:?}", pass.step(), e)
                            }
                        }
                    }
                }

                // Update progress
                let elapsed_ms = performance_now() - render_start_time;
                progress.update(|p| {
                    p.completed_steps += 1;
                    p.elapsed_ms = elapsed_ms;
                    p.is_complete = pass.is_final();
                });

                if !pass.is_final() {
                    // Schedule next pass via double rAF
                    let passes = Adam7Pass::all();
                    let next_index = pass_index + 1;
                    if next_index < passes.len() {
                        request_animation_frame_then(move || {
                            request_animation_frame_then(move || {
                                schedule_adam7_pass(
                                    passes[next_index],
                                    next_index,
                                    expected_gen,
                                    width,
                                    height,
                                    config,
                                    generation_spawn,
                                    gpu_renderer_spawn,
                                    canvas_element_spawn,
                                    xray_enabled_spawn,
                                    tile_results_spawn,
                                    worker_pool_spawn,
                                    progress,
                                    viewport_spawn,
                                    tiles_spawn,
                                    orbit_data_spawn,
                                    render_start_time,
                                    gpu_in_use_spawn,
                                    adam7_accumulator_spawn,
                                );
                            });
                        });
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "GPU Adam7 pass {} failed: {e}, falling back to CPU",
                    pass.step()
                );
                worker_pool_spawn.borrow_mut().start_perturbation_render(
                    viewport_spawn,
                    (width, height),
                    tiles_spawn,
                );
            }
        }
    });
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
