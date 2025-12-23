//! Perturbation rendering coordinator.
//!
//! Manages perturbation-specific state and coordinates between
//! reference orbit computation, tile dispatch, and glitch resolution.

use super::glitch_resolution::GlitchResolver;
use super::helpers::{calculate_dc_max, calculate_render_max_iterations, validate_viewport};
use crate::config::get_config;
use fractalwonder_core::{BigFloat, MainToWorker, PixelRect, Viewport};
use std::collections::HashSet;

/// Request to compute a reference orbit.
#[derive(Clone)]
pub struct OrbitRequest {
    pub render_id: u32,
    pub orbit_id: u32,
    pub c_ref_json: String,
    pub max_iterations: u32,
}

/// Orbit data received from worker.
#[derive(Clone)]
pub struct OrbitData {
    pub c_ref: (f64, f64),
    pub orbit: Vec<(f64, f64)>,
    pub derivative: Vec<(f64, f64)>,
    pub escaped_at: Option<u32>,
}

/// Internal perturbation state.
struct PerturbationState {
    /// Current orbit ID being used
    orbit_id: u32,
    /// Workers that have confirmed storing the orbit
    workers_with_orbit: HashSet<usize>,
    /// Maximum iterations for perturbation tiles
    max_iterations: u32,
    /// Delta step per pixel in fractal space
    delta_step: (BigFloat, BigFloat),
    /// Glitch detection threshold squared
    tau_sq: f64,
    /// Maximum |delta_c| for BLA table construction
    dc_max: f64,
    /// Enable BLA for iteration skipping
    bla_enabled: bool,
}

impl Default for PerturbationState {
    fn default() -> Self {
        Self {
            orbit_id: 0,
            workers_with_orbit: HashSet::new(),
            max_iterations: 0,
            delta_step: (BigFloat::zero(64), BigFloat::zero(64)),
            tau_sq: 1e-6,
            dc_max: 0.0,
            bla_enabled: true,
        }
    }
}

/// Coordinates perturbation rendering across workers.
pub struct PerturbationCoordinator {
    state: PerturbationState,
    glitch_resolver: GlitchResolver,
    /// Current viewport for delta calculations
    current_viewport: Option<Viewport>,
    /// Canvas dimensions
    canvas_size: (u32, u32),
    /// Renderer ID for config lookup
    renderer_id: String,
}

impl PerturbationCoordinator {
    pub fn new(renderer_id: &str) -> Self {
        Self {
            state: PerturbationState::default(),
            glitch_resolver: GlitchResolver::new(),
            current_viewport: None,
            canvas_size: (0, 0),
            renderer_id: renderer_id.to_string(),
        }
    }

    /// Update renderer ID when switching fractals.
    pub fn set_renderer_id(&mut self, renderer_id: &str) {
        self.renderer_id = renderer_id.to_string();
    }

    /// Get current orbit ID.
    pub fn orbit_id(&self) -> u32 {
        self.state.orbit_id
    }

    /// Get max iterations for current render.
    pub fn max_iterations(&self) -> u32 {
        self.state.max_iterations
    }

    /// Get dc_max for BLA.
    pub fn dc_max(&self) -> f64 {
        self.state.dc_max
    }

    /// Get bla_enabled flag.
    pub fn bla_enabled(&self) -> bool {
        self.state.bla_enabled
    }

    /// Access glitch resolver.
    pub fn glitch_resolver(&self) -> &GlitchResolver {
        &self.glitch_resolver
    }

    /// Access glitch resolver mutably.
    pub fn glitch_resolver_mut(&mut self) -> &mut GlitchResolver {
        &mut self.glitch_resolver
    }

    /// Check if a worker is ready for tile dispatch.
    pub fn worker_ready_for_tiles(&self, worker_id: usize) -> bool {
        self.state.workers_with_orbit.contains(&worker_id)
    }

    /// Record that a worker has stored the orbit.
    pub fn record_worker_has_orbit(&mut self, worker_id: usize) {
        self.state.workers_with_orbit.insert(worker_id);
    }

    /// Check if all initialized workers have the orbit.
    pub fn all_workers_have_orbit(&self, initialized_workers: &HashSet<usize>) -> bool {
        initialized_workers
            .iter()
            .all(|&id| self.state.workers_with_orbit.contains(&id))
    }

    /// Get count of workers with orbit.
    pub fn workers_with_orbit_count(&self) -> usize {
        self.state.workers_with_orbit.len()
    }

    /// Prepare for a new perturbation render.
    ///
    /// Returns Ok(OrbitRequest) if valid, Err(message) if viewport invalid.
    pub fn start_render(
        &mut self,
        render_id: u32,
        viewport: &Viewport,
        canvas_size: (u32, u32),
    ) -> Result<OrbitRequest, String> {
        // Validate viewport
        validate_viewport(viewport)?;

        // Reset state
        self.state.orbit_id = self.state.orbit_id.wrapping_add(1);
        self.state.workers_with_orbit.clear();
        self.current_viewport = Some(viewport.clone());
        self.canvas_size = canvas_size;

        // Initialize glitch resolver
        self.glitch_resolver.init_for_render(canvas_size);

        // Get config
        let config = get_config(&self.renderer_id);

        // Calculate render parameters
        self.state.max_iterations = calculate_render_max_iterations(viewport, config);
        self.state.tau_sq = config.map(|c| c.tau_sq).unwrap_or(1e-6);
        self.state.dc_max = calculate_dc_max(viewport);
        self.state.bla_enabled = config.map(|c| c.bla_enabled).unwrap_or(true);

        // Calculate delta step per pixel
        let precision = viewport.width.precision_bits();
        let canvas_width_bf = BigFloat::with_precision(canvas_size.0 as f64, precision);
        let canvas_height_bf = BigFloat::with_precision(canvas_size.1 as f64, precision);
        self.state.delta_step = (
            viewport.width.div(&canvas_width_bf),
            viewport.height.div(&canvas_height_bf),
        );

        // Prepare orbit request
        let c_ref_json = serde_json::to_string(&viewport.center).unwrap_or_default();

        Ok(OrbitRequest {
            render_id,
            orbit_id: self.state.orbit_id,
            c_ref_json,
            max_iterations: self.state.max_iterations,
        })
    }

    /// Prepare for GPU-only orbit computation (no tiles).
    pub fn start_gpu_render(
        &mut self,
        render_id: u32,
        viewport: &Viewport,
        canvas_size: (u32, u32),
    ) -> Result<OrbitRequest, String> {
        // Validate viewport
        validate_viewport(viewport)?;

        // Reset state (simplified for GPU - no glitch tracking)
        self.state.orbit_id = self.state.orbit_id.wrapping_add(1);
        self.state.workers_with_orbit.clear();
        self.current_viewport = Some(viewport.clone());
        self.canvas_size = canvas_size;

        // Get config
        let config = get_config(&self.renderer_id);

        // Calculate render parameters
        self.state.max_iterations = calculate_render_max_iterations(viewport, config);
        self.state.tau_sq = config.map(|c| c.tau_sq).unwrap_or(1e-6);

        // Prepare orbit request
        let c_ref_json = serde_json::to_string(&viewport.center).unwrap_or_default();

        Ok(OrbitRequest {
            render_id,
            orbit_id: self.state.orbit_id,
            c_ref_json,
            max_iterations: self.state.max_iterations,
        })
    }

    /// Build StoreReferenceOrbit messages for all workers.
    pub fn build_orbit_broadcast(&self, orbit_data: &OrbitData) -> MainToWorker {
        MainToWorker::StoreReferenceOrbit {
            orbit_id: self.state.orbit_id,
            c_ref: orbit_data.c_ref,
            orbit: orbit_data.orbit.clone(),
            derivative: orbit_data.derivative.clone(),
            escaped_at: orbit_data.escaped_at,
            dc_max: self.state.dc_max,
            bla_enabled: self.state.bla_enabled,
        }
    }

    /// Build RenderTilePerturbation message for a tile.
    pub fn build_tile_message(&self, render_id: u32, tile: PixelRect) -> Option<MainToWorker> {
        let viewport = self.current_viewport.as_ref()?;
        let precision = viewport.width.precision_bits();

        // Calculate delta_c_origin for this tile's top-left pixel
        let norm_x = tile.x as f64 / self.canvas_size.0 as f64 - 0.5;
        let norm_y = tile.y as f64 / self.canvas_size.1 as f64 - 0.5;

        let norm_x_bf = BigFloat::with_precision(norm_x, precision);
        let norm_y_bf = BigFloat::with_precision(norm_y, precision);
        let delta_c_origin = (
            norm_x_bf.mul(&viewport.width),
            norm_y_bf.mul(&viewport.height),
        );

        let delta_c_origin_json = serde_json::to_string(&delta_c_origin).ok()?;
        let delta_c_step_json = serde_json::to_string(&self.state.delta_step).ok()?;

        let bigfloat_threshold_bits = get_config(&self.renderer_id)
            .map(|c| c.bigfloat_threshold_bits)
            .unwrap_or(1024);

        Some(MainToWorker::RenderTilePerturbation {
            render_id,
            tile,
            orbit_id: self.state.orbit_id,
            delta_c_origin_json,
            delta_c_step_json,
            max_iterations: self.state.max_iterations,
            tau_sq: self.state.tau_sq,
            bigfloat_threshold_bits,
            bla_enabled: self.state.bla_enabled,
        })
    }

    /// Reset state for cancel or non-perturbation render.
    pub fn reset(&mut self) {
        self.state.workers_with_orbit.clear();
        self.glitch_resolver.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_viewport() -> Viewport {
        Viewport {
            center: (
                BigFloat::with_precision(-0.5, 64),
                BigFloat::with_precision(0.0, 64),
            ),
            width: BigFloat::with_precision(4.0, 64),
            height: BigFloat::with_precision(4.0, 64),
        }
    }

    #[test]
    fn new_coordinator_has_zero_orbit_id() {
        let coord = PerturbationCoordinator::new("mandelbrot");
        assert_eq!(coord.orbit_id(), 0);
    }

    #[test]
    fn start_render_increments_orbit_id() {
        let mut coord = PerturbationCoordinator::new("mandelbrot");
        let viewport = create_test_viewport();
        let _ = coord.start_render(1, &viewport, (800, 600));
        assert_eq!(coord.orbit_id(), 1);
        let _ = coord.start_render(2, &viewport, (800, 600));
        assert_eq!(coord.orbit_id(), 2);
    }

    #[test]
    fn start_render_returns_orbit_request() {
        let mut coord = PerturbationCoordinator::new("mandelbrot");
        let viewport = create_test_viewport();
        let result = coord.start_render(42, &viewport, (800, 600));
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.render_id, 42);
        assert_eq!(request.orbit_id, 1);
        assert!(request.max_iterations > 0);
    }

    #[test]
    fn worker_ready_for_tiles_false_initially() {
        let coord = PerturbationCoordinator::new("mandelbrot");
        assert!(!coord.worker_ready_for_tiles(0));
    }

    #[test]
    fn record_worker_has_orbit_makes_ready() {
        let mut coord = PerturbationCoordinator::new("mandelbrot");
        coord.record_worker_has_orbit(0);
        assert!(coord.worker_ready_for_tiles(0));
    }

    #[test]
    fn reset_clears_workers_with_orbit() {
        let mut coord = PerturbationCoordinator::new("mandelbrot");
        coord.record_worker_has_orbit(0);
        coord.reset();
        assert!(!coord.worker_ready_for_tiles(0));
    }
}
