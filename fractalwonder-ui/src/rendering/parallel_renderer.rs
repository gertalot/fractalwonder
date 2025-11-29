use crate::config::{FractalConfig, RendererType};
use crate::rendering::canvas_utils::{draw_pixels_to_canvas, get_2d_context};
use crate::rendering::colorizers::colorize;
use crate::rendering::tiles::{calculate_tile_size, generate_tiles};
use crate::rendering::RenderProgress;
use crate::workers::{TileResult, WorkerPool};
use fractalwonder_core::Viewport;
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

    pub fn switch_config(&mut self, config: &'static FractalConfig) -> Result<(), JsValue> {
        self.config = config;
        self.worker_pool.borrow_mut().switch_renderer(config.id);
        Ok(())
    }
}
