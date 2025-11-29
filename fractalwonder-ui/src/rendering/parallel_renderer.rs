use crate::config::{FractalConfig, RendererType};
use crate::rendering::canvas_utils::{draw_full_frame, draw_pixels_to_canvas, get_2d_context};
use crate::rendering::colorizers::colorize;
use crate::rendering::tiles::{calculate_tile_size, generate_tiles};
use crate::rendering::RenderProgress;
use crate::workers::{TileResult, WorkerPool};
use fractalwonder_core::{PixelRect, Viewport};
use fractalwonder_gpu::{GpuAvailability, GpuContext, GpuRenderer};
use leptos::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::JsValue;
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
    /// Canvas dimensions for GPU rendering
    canvas_size: Rc<Cell<(u32, u32)>>,
}

impl ParallelRenderer {
    pub fn new(config: &'static FractalConfig) -> Result<Self, JsValue> {
        let progress = create_rw_signal(RenderProgress::default());
        let canvas_ctx: Rc<RefCell<Option<CanvasRenderingContext2d>>> = Rc::new(RefCell::new(None));
        let xray_enabled: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let tile_results: Rc<RefCell<Vec<TileResult>>> = Rc::new(RefCell::new(Vec::new()));
        let gpu_renderer: Rc<RefCell<Option<GpuRenderer>>> = Rc::new(RefCell::new(None));
        let gpu_init_attempted: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let canvas_size: Rc<Cell<(u32, u32)>> = Rc::new(Cell::new((0, 0)));

        let ctx_clone = Rc::clone(&canvas_ctx);
        let xray_clone = Rc::clone(&xray_enabled);
        let results_clone = Rc::clone(&tile_results);
        let on_tile_complete = move |result: TileResult| {
            if let Some(ctx) = ctx_clone.borrow().as_ref() {
                // Colorize with current xray state
                let xray = xray_clone.get();
                let pixels: Vec<u8> = result.data.iter().flat_map(|d| colorize(d, xray)).collect();

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
            canvas_size,
        })
    }

    /// Set x-ray mode enabled state.
    pub fn set_xray_enabled(&self, enabled: bool) {
        self.xray_enabled.set(enabled);
    }

    /// Re-colorize all stored tiles with current xray state (no recompute).
    pub fn recolorize(&self) {
        let xray = self.xray_enabled.get();
        let ctx_ref = self.canvas_ctx.borrow();
        let Some(ctx) = ctx_ref.as_ref() else {
            return;
        };

        for result in self.tile_results.borrow().iter() {
            let pixels: Vec<u8> = result.data.iter().flat_map(|d| colorize(d, xray)).collect();
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
                self.worker_pool.borrow_mut().start_perturbation_render(
                    viewport.clone(),
                    (width, height),
                    tiles,
                );
            }
        }
    }

    /// Start GPU-accelerated render if available.
    /// Falls back to CPU tile rendering if GPU fails.
    pub fn render_with_gpu(
        &self,
        viewport: &Viewport,
        canvas: &HtmlCanvasElement,
        orbit: Vec<(f64, f64)>,
        orbit_id: u32,
        max_iterations: u32,
    ) {
        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            return;
        }

        // Store canvas context
        if let Ok(ctx) = get_2d_context(canvas) {
            *self.canvas_ctx.borrow_mut() = Some(ctx);
        }

        self.canvas_size.set((width, height));

        // Set up progress for GPU (1 "tile" = whole frame)
        self.progress.set(RenderProgress::new(1));

        // Clone what we need for the async block
        let gpu_renderer = Rc::clone(&self.gpu_renderer);
        let gpu_init_attempted = Rc::clone(&self.gpu_init_attempted);
        let canvas_ctx = Rc::clone(&self.canvas_ctx);
        let xray_enabled = Rc::clone(&self.xray_enabled);
        let tile_results = Rc::clone(&self.tile_results);
        let progress = self.progress;
        let config = self.config;
        let worker_pool = Rc::clone(&self.worker_pool);
        let viewport_clone = viewport.clone();
        let tiles = generate_tiles(width, height, calculate_tile_size(1.0));

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

            // Try GPU render
            let gpu_result = {
                let mut gpu_ref = gpu_renderer.borrow_mut();
                if let Some(gpu) = gpu_ref.as_mut() {
                    let vp_width = viewport_clone.width.to_f64() as f32;
                    let vp_height = viewport_clone.height.to_f64() as f32;
                    let dc_origin = (-vp_width / 2.0, -vp_height / 2.0);
                    let dc_step = (vp_width / width as f32, vp_height / height as f32);
                    let tau_sq = config.tau_sq as f32;

                    Some(
                        gpu.render(
                            &orbit,
                            orbit_id,
                            dc_origin,
                            dc_step,
                            width,
                            height,
                            max_iterations,
                            tau_sq,
                        )
                        .await,
                    )
                } else {
                    None
                }
            };

            match gpu_result {
                Some(Ok(result)) => {
                    log::info!(
                        "GPU render: {}x{} in {:.1}ms",
                        width,
                        height,
                        result.compute_time_ms
                    );

                    let xray = xray_enabled.get();
                    let pixels: Vec<u8> = result
                        .data
                        .iter()
                        .flat_map(|d| colorize(d, xray))
                        .collect();

                    if let Some(ctx) = canvas_ctx.borrow().as_ref() {
                        let _ = draw_full_frame(ctx, &pixels, width, height);
                    }

                    // Store for recolorize
                    tile_results.borrow_mut().clear();
                    tile_results.borrow_mut().push(TileResult {
                        tile: PixelRect {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        },
                        data: result.data,
                        compute_time_ms: result.compute_time_ms,
                    });

                    progress.update(|p| {
                        p.completed_tiles = 1;
                        p.is_complete = true;
                    });
                }
                Some(Err(e)) => {
                    log::warn!("GPU render failed: {e}, falling back to CPU");
                    worker_pool
                        .borrow_mut()
                        .start_perturbation_render(viewport_clone, (width, height), tiles);
                }
                None => {
                    log::info!("No GPU available, using CPU");
                    worker_pool
                        .borrow_mut()
                        .start_perturbation_render(viewport_clone, (width, height), tiles);
                }
            }
        });
    }

    pub fn switch_config(&mut self, config: &'static FractalConfig) -> Result<(), JsValue> {
        self.config = config;
        self.worker_pool.borrow_mut().switch_renderer(config.id);
        Ok(())
    }
}
