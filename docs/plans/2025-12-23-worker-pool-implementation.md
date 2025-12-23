# Worker Pool Refactoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split 1291-line worker_pool.rs into focused modules: helpers, glitch_resolution, coordinator, and slimmed worker_pool.

**Architecture:** Extract pure functions to helpers.rs, glitch tracking to glitch_resolution.rs, perturbation state to coordinator.rs. WorkerPool becomes thin orchestrator delegating to PerturbationCoordinator.

**Tech Stack:** Rust, Leptos (reactive signals), Web Workers (wasm-bindgen), serde for message serialization.

---

## Task 1: Create Module Structure

**Files:**
- Create: `fractalwonder-ui/src/workers/perturbation/mod.rs`
- Create: `fractalwonder-ui/src/workers/perturbation/helpers.rs`
- Modify: `fractalwonder-ui/src/workers/mod.rs`

**Step 1: Create perturbation directory and mod.rs**

Create file `fractalwonder-ui/src/workers/perturbation/mod.rs`:

```rust
//! Perturbation rendering coordination and glitch resolution.

mod helpers;

pub use helpers::{calculate_dc_max, calculate_render_max_iterations, validate_viewport};
```

**Step 2: Create empty helpers.rs**

Create file `fractalwonder-ui/src/workers/perturbation/helpers.rs`:

```rust
//! Pure helper functions for perturbation rendering.
//!
//! These functions are stateless and easily testable.

use crate::config::{get_config, FractalConfig};
use fractalwonder_core::{calculate_max_iterations, Viewport};

/// Validate viewport dimensions for rendering.
///
/// Returns Ok(()) if valid, Err with message if invalid.
pub fn validate_viewport(viewport: &Viewport) -> Result<(), String> {
    let vp_width = viewport.width.to_f64();
    let vp_height = viewport.height.to_f64();

    if !vp_width.is_finite() || !vp_height.is_finite() || vp_width <= 0.0 || vp_height <= 0.0 {
        return Err(format!(
            "Invalid viewport dimensions: width={}, height={}",
            vp_width, vp_height
        ));
    }

    Ok(())
}

/// Calculate maximum iterations for a render based on zoom level.
pub fn calculate_render_max_iterations(viewport: &Viewport, config: Option<&FractalConfig>) -> u32 {
    let vp_width = viewport.width.to_f64();

    // Calculate zoom exponent from viewport width
    // Default Mandelbrot width is ~4, so zoom = 4 / width
    let zoom = 4.0 / vp_width;
    let zoom_exponent = if zoom.is_finite() && zoom > 0.0 {
        zoom.log10()
    } else {
        0.0
    };

    let multiplier = config.map(|c| c.iteration_multiplier).unwrap_or(200.0);
    let power = config.map(|c| c.iteration_power).unwrap_or(2.5);

    calculate_max_iterations(zoom_exponent, multiplier, power)
}

/// Calculate maximum |delta_c| for any pixel in the viewport.
///
/// This is the distance from viewport center to the farthest corner,
/// used for BLA table construction.
pub fn calculate_dc_max(viewport: &Viewport) -> f64 {
    let half_width = viewport.width.to_f64() / 2.0;
    let half_height = viewport.height.to_f64() / 2.0;

    (half_width * half_width + half_height * half_height).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractalwonder_core::BigFloat;

    fn create_test_viewport(width: f64, height: f64) -> Viewport {
        Viewport {
            center: (
                BigFloat::with_precision(-0.5, 64),
                BigFloat::with_precision(0.0, 64),
            ),
            width: BigFloat::with_precision(width, 64),
            height: BigFloat::with_precision(height, 64),
        }
    }

    #[test]
    fn validate_viewport_accepts_valid() {
        let viewport = create_test_viewport(4.0, 4.0);
        assert!(validate_viewport(&viewport).is_ok());
    }

    #[test]
    fn validate_viewport_rejects_zero_width() {
        let viewport = create_test_viewport(0.0, 4.0);
        assert!(validate_viewport(&viewport).is_err());
    }

    #[test]
    fn validate_viewport_rejects_negative_height() {
        let viewport = create_test_viewport(4.0, -1.0);
        assert!(validate_viewport(&viewport).is_err());
    }

    #[test]
    fn calculate_dc_max_at_default_zoom() {
        let viewport = create_test_viewport(4.0, 4.0);
        let dc_max = calculate_dc_max(&viewport);
        // sqrt(2^2 + 2^2) = sqrt(8) ≈ 2.828
        assert!((dc_max - 2.828).abs() < 0.01);
    }

    #[test]
    fn calculate_max_iterations_increases_with_zoom() {
        let shallow = create_test_viewport(4.0, 4.0);
        let deep = create_test_viewport(0.0001, 0.0001);

        let shallow_iter = calculate_render_max_iterations(&shallow, None);
        let deep_iter = calculate_render_max_iterations(&deep, None);

        assert!(deep_iter > shallow_iter);
    }
}
```

**Step 3: Update workers/mod.rs**

Modify `fractalwonder-ui/src/workers/mod.rs` to add perturbation module:

```rust
mod perturbation;
mod quadtree;
mod worker_pool;

pub use perturbation::{calculate_dc_max, calculate_render_max_iterations, validate_viewport};
pub use quadtree::{subdivide_to_depth, Bounds, QuadtreeCell, MAX_DEPTH, MIN_CELL_SIZE};
pub use worker_pool::{OrbitCompleteData, TileResult, WorkerPool};
```

**Step 4: Run tests to verify module structure**

Run: `cargo test --package fractalwonder-ui -q`

Expected: All tests pass (including new helper tests)

**Step 5: Commit**

```bash
git add fractalwonder-ui/src/workers/perturbation/
git add fractalwonder-ui/src/workers/mod.rs
git commit -m "feat(workers): add perturbation module with helpers"
```

---

## Task 2: Extract GlitchResolver

**Files:**
- Create: `fractalwonder-ui/src/workers/perturbation/glitch_resolution.rs`
- Modify: `fractalwonder-ui/src/workers/perturbation/mod.rs`

**Step 1: Create glitch_resolution.rs with struct and basic methods**

Create file `fractalwonder-ui/src/workers/perturbation/glitch_resolution.rs`:

```rust
//! Glitch resolution using quadtree-based spatial tracking.
//!
//! Implements Phase 7-8 of perturbation rendering: detecting glitched regions,
//! computing reference orbits for cell centers, and distributing to workers.

use crate::workers::quadtree::{Bounds, QuadtreeCell};
use fractalwonder_compute::ReferenceOrbit;
use fractalwonder_core::{pixel_to_fractal, MainToWorker, PixelRect, Viewport};
use std::collections::{HashMap, HashSet};

/// Key for identifying quadtree cells: (x, y, width, height)
pub type CellKey = (u32, u32, u32, u32);

/// Data needed to broadcast an orbit to workers.
#[derive(Clone)]
pub struct OrbitBroadcast {
    pub c_ref: (f64, f64),
    pub orbit: Vec<(f64, f64)>,
    pub derivative: Vec<(f64, f64)>,
    pub escaped_at: Option<u32>,
}

/// Manages glitch detection and resolution via quadtree subdivision.
pub struct GlitchResolver {
    /// Quadtree for spatial tracking of glitched regions
    quadtree: Option<QuadtreeCell>,
    /// Tiles that have glitched pixels
    glitched_tiles: Vec<PixelRect>,
    /// Glitched tile count for current render
    glitched_tile_count: u32,
    /// Computed reference orbits for cell centers
    cell_orbits: HashMap<CellKey, ReferenceOrbit>,
    /// Mapping from cell bounds to orbit_id
    cell_orbit_ids: HashMap<CellKey, u32>,
    /// Counter for generating unique cell orbit IDs
    orbit_id_counter: u32,
    /// Tracks which workers have confirmed storing which cell orbits
    cell_orbit_confirmations: HashMap<u32, HashSet<usize>>,
}

impl Default for GlitchResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GlitchResolver {
    pub fn new() -> Self {
        Self {
            quadtree: None,
            glitched_tiles: Vec::new(),
            glitched_tile_count: 0,
            cell_orbits: HashMap::new(),
            cell_orbit_ids: HashMap::new(),
            orbit_id_counter: 1000, // Start at 1000 to distinguish from main orbit IDs
            cell_orbit_confirmations: HashMap::new(),
        }
    }

    /// Initialize for a new render.
    pub fn init_for_render(&mut self, canvas_size: (u32, u32)) {
        self.quadtree = Some(QuadtreeCell::new_root(canvas_size));
        self.glitched_tiles.clear();
        self.glitched_tile_count = 0;
        self.cell_orbits.clear();
        self.cell_orbit_ids.clear();
        self.cell_orbit_confirmations.clear();
    }

    /// Clear state (for non-perturbation renders).
    pub fn clear(&mut self) {
        self.quadtree = None;
        self.glitched_tiles.clear();
        self.glitched_tile_count = 0;
    }

    /// Record a tile that has glitched pixels.
    pub fn record_glitched_tile(&mut self, tile: PixelRect) {
        self.glitched_tiles.push(tile);
        self.glitched_tile_count += 1;
    }

    /// Get current glitched tile count.
    pub fn glitched_tile_count(&self) -> u32 {
        self.glitched_tile_count
    }

    /// Get quadtree reference for logging.
    pub fn quadtree(&self) -> Option<&QuadtreeCell> {
        self.quadtree.as_ref()
    }

    /// Check if a cell orbit confirmation is being tracked.
    pub fn is_tracking_orbit(&self, orbit_id: u32) -> bool {
        self.cell_orbit_confirmations.contains_key(&orbit_id)
    }

    /// Record worker confirmation for an orbit.
    ///
    /// Returns true if all initialized workers have confirmed.
    pub fn confirm_orbit_stored(
        &mut self,
        orbit_id: u32,
        worker_id: usize,
        initialized_workers: &HashSet<usize>,
    ) -> bool {
        if let Some(confirmations) = self.cell_orbit_confirmations.get_mut(&orbit_id) {
            confirmations.insert(worker_id);
            initialized_workers
                .iter()
                .all(|&id| confirmations.contains(&id))
        } else {
            false
        }
    }

    /// Subdivide quadtree cells that contain glitched tiles.
    ///
    /// Performs ONE level of subdivision for leaf cells intersecting glitched tiles.
    /// Returns the number of cells subdivided.
    pub fn subdivide_glitched_cells(&mut self) -> u32 {
        let Some(quadtree) = &mut self.quadtree else {
            return 0;
        };

        if self.glitched_tiles.is_empty() {
            return 0;
        }

        fn tile_to_bounds(tile: &PixelRect) -> Bounds {
            Bounds::new(tile.x, tile.y, tile.width, tile.height)
        }

        fn subdivide_leaves_once(
            cell: &mut QuadtreeCell,
            glitched_tiles: &[PixelRect],
            subdivided_count: &mut u32,
        ) {
            let has_glitched = glitched_tiles
                .iter()
                .any(|tile| cell.bounds.intersects(&tile_to_bounds(tile)));

            if !has_glitched {
                return;
            }

            if cell.is_leaf() {
                if cell.subdivide() {
                    *subdivided_count += 1;
                }
                return;
            }

            if let Some(children) = &mut cell.children {
                for child in children.iter_mut() {
                    subdivide_leaves_once(child, glitched_tiles, subdivided_count);
                }
            }
        }

        let mut subdivided_count = 0;
        subdivide_leaves_once(quadtree, &self.glitched_tiles, &mut subdivided_count);
        subdivided_count
    }

    /// Compute reference orbits for cells containing glitched tiles.
    ///
    /// For each leaf cell with glitched tiles:
    /// 1. Computes the cell center in fractal coordinates
    /// 2. Computes a ReferenceOrbit at that point
    /// 3. Stores the orbit for later distribution
    ///
    /// Returns the number of orbits computed.
    pub fn compute_cell_orbits(
        &mut self,
        viewport: &Viewport,
        canvas_size: (u32, u32),
        max_iterations: u32,
    ) -> u32 {
        let Some(quadtree) = &self.quadtree else {
            return 0;
        };

        fn tile_to_bounds(tile: &PixelRect) -> Bounds {
            Bounds::new(tile.x, tile.y, tile.width, tile.height)
        }

        let mut leaves = Vec::new();
        quadtree.collect_leaves(&mut leaves);

        let precision_bits = viewport.precision_bits();
        let mut computed_count = 0;

        for leaf in &leaves {
            let has_glitched = self
                .glitched_tiles
                .iter()
                .any(|tile| leaf.bounds.intersects(&tile_to_bounds(tile)));

            if !has_glitched {
                continue;
            }

            let cell_key = (
                leaf.bounds.x,
                leaf.bounds.y,
                leaf.bounds.width,
                leaf.bounds.height,
            );

            if self.cell_orbits.contains_key(&cell_key) {
                continue;
            }

            // Compute cell center in pixel coordinates
            let center_px_x = leaf.bounds.x as f64 + leaf.bounds.width as f64 / 2.0;
            let center_px_y = leaf.bounds.y as f64 + leaf.bounds.height as f64 / 2.0;

            // Convert to fractal coordinates
            let (c_ref_x, c_ref_y) =
                pixel_to_fractal(center_px_x, center_px_y, viewport, canvas_size, precision_bits);

            // Compute the reference orbit
            let c_ref = (c_ref_x, c_ref_y);
            let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);

            self.cell_orbits.insert(cell_key, orbit);
            computed_count += 1;
        }

        computed_count
    }

    /// Get orbits that need to be broadcast to workers.
    ///
    /// Returns Vec of (orbit_id, broadcast_data) for orbits not yet assigned an ID.
    pub fn orbits_to_broadcast(&mut self, dc_max: f64, bla_enabled: bool) -> Vec<(u32, MainToWorker)> {
        let mut broadcasts = Vec::new();

        let cells_without_id: Vec<CellKey> = self
            .cell_orbits
            .keys()
            .filter(|key| !self.cell_orbit_ids.contains_key(*key))
            .cloned()
            .collect();

        for cell_key in cells_without_id {
            let Some(orbit) = self.cell_orbits.get(&cell_key) else {
                continue;
            };

            let orbit_id = self.orbit_id_counter;
            self.orbit_id_counter = self.orbit_id_counter.wrapping_add(1);

            self.cell_orbit_ids.insert(cell_key, orbit_id);
            self.cell_orbit_confirmations
                .insert(orbit_id, HashSet::new());

            let msg = MainToWorker::StoreReferenceOrbit {
                orbit_id,
                c_ref: orbit.c_ref,
                orbit: orbit.orbit.clone(),
                derivative: orbit.derivative.clone(),
                escaped_at: orbit.escaped_at,
                dc_max,
                bla_enabled,
            };

            broadcasts.push((orbit_id, msg));
        }

        broadcasts
    }

    /// Get leaves with their glitch counts for logging.
    pub fn leaves_with_glitch_counts(&self) -> Vec<(Bounds, usize)> {
        let Some(quadtree) = &self.quadtree else {
            return Vec::new();
        };

        fn tile_to_bounds(tile: &PixelRect) -> Bounds {
            Bounds::new(tile.x, tile.y, tile.width, tile.height)
        }

        let mut leaves = Vec::new();
        quadtree.collect_leaves(&mut leaves);

        leaves
            .into_iter()
            .filter_map(|leaf| {
                let count = self
                    .glitched_tiles
                    .iter()
                    .filter(|tile| leaf.bounds.intersects(&tile_to_bounds(tile)))
                    .count();
                if count > 0 {
                    Some((leaf.bounds, count))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_resolver_has_no_quadtree() {
        let resolver = GlitchResolver::new();
        assert!(resolver.quadtree().is_none());
        assert_eq!(resolver.glitched_tile_count(), 0);
    }

    #[test]
    fn init_creates_quadtree() {
        let mut resolver = GlitchResolver::new();
        resolver.init_for_render((800, 600));
        assert!(resolver.quadtree().is_some());
    }

    #[test]
    fn record_glitched_tile_increments_count() {
        let mut resolver = GlitchResolver::new();
        resolver.init_for_render((800, 600));
        resolver.record_glitched_tile(PixelRect::new(0, 0, 64, 64));
        resolver.record_glitched_tile(PixelRect::new(64, 0, 64, 64));
        assert_eq!(resolver.glitched_tile_count(), 2);
    }

    #[test]
    fn subdivide_with_no_glitches_returns_zero() {
        let mut resolver = GlitchResolver::new();
        resolver.init_for_render((800, 600));
        let count = resolver.subdivide_glitched_cells();
        assert_eq!(count, 0);
    }

    #[test]
    fn subdivide_with_glitches_subdivides_cells() {
        let mut resolver = GlitchResolver::new();
        resolver.init_for_render((128, 128));
        resolver.record_glitched_tile(PixelRect::new(0, 0, 32, 32));
        let count = resolver.subdivide_glitched_cells();
        assert!(count > 0);
    }

    #[test]
    fn clear_removes_quadtree() {
        let mut resolver = GlitchResolver::new();
        resolver.init_for_render((800, 600));
        resolver.record_glitched_tile(PixelRect::new(0, 0, 64, 64));
        resolver.clear();
        assert!(resolver.quadtree().is_none());
        assert_eq!(resolver.glitched_tile_count(), 0);
    }
}
```

**Step 2: Update perturbation/mod.rs**

Modify `fractalwonder-ui/src/workers/perturbation/mod.rs`:

```rust
//! Perturbation rendering coordination and glitch resolution.

mod glitch_resolution;
mod helpers;

pub use glitch_resolution::GlitchResolver;
pub use helpers::{calculate_dc_max, calculate_render_max_iterations, validate_viewport};
```

**Step 3: Run tests**

Run: `cargo test --package fractalwonder-ui -q`

Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/workers/perturbation/glitch_resolution.rs
git add fractalwonder-ui/src/workers/perturbation/mod.rs
git commit -m "feat(workers): add GlitchResolver for Phase 7-8 glitch handling"
```

---

## Task 3: Create PerturbationState and OrbitRequest Types

**Files:**
- Create: `fractalwonder-ui/src/workers/perturbation/coordinator.rs`
- Modify: `fractalwonder-ui/src/workers/perturbation/mod.rs`

**Step 1: Create coordinator.rs with state types**

Create file `fractalwonder-ui/src/workers/perturbation/coordinator.rs`:

```rust
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

    /// Get tau_sq for glitch detection.
    pub fn tau_sq(&self) -> f64 {
        self.state.tau_sq
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
```

**Step 2: Update perturbation/mod.rs**

Modify `fractalwonder-ui/src/workers/perturbation/mod.rs`:

```rust
//! Perturbation rendering coordination and glitch resolution.

mod coordinator;
mod glitch_resolution;
mod helpers;

pub use coordinator::{OrbitData, OrbitRequest, PerturbationCoordinator};
pub use glitch_resolution::GlitchResolver;
pub use helpers::{calculate_dc_max, calculate_render_max_iterations, validate_viewport};
```

**Step 3: Run tests**

Run: `cargo test --package fractalwonder-ui -q`

Expected: All tests pass

**Step 4: Commit**

```bash
git add fractalwonder-ui/src/workers/perturbation/coordinator.rs
git add fractalwonder-ui/src/workers/perturbation/mod.rs
git commit -m "feat(workers): add PerturbationCoordinator for render orchestration"
```

---

## Task 4: Refactor WorkerPool to Use PerturbationCoordinator

**Files:**
- Modify: `fractalwonder-ui/src/workers/worker_pool.rs`
- Modify: `fractalwonder-ui/src/workers/mod.rs`

**Step 1: Update imports and struct in worker_pool.rs**

Replace the imports and struct definition at the top of `fractalwonder-ui/src/workers/worker_pool.rs`:

```rust
use crate::config::get_config;
use crate::rendering::RenderProgress;
use crate::workers::perturbation::{OrbitData, OrbitRequest, PerturbationCoordinator};
use crate::workers::quadtree::Bounds;
use fractalwonder_core::{BigFloat, ComputeData, MainToWorker, PixelRect, Viewport, WorkerToMain};
use leptos::*;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::{Rc, Weak};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

const WORKER_SCRIPT_PATH: &str = "./message-compute-worker.js";

#[derive(Clone)]
pub struct TileResult {
    pub tile: PixelRect,
    pub data: Vec<ComputeData>,
    pub compute_time_ms: f64,
}

/// Orbit data passed to the orbit complete callback.
#[derive(Clone)]
pub struct OrbitCompleteData {
    pub orbit: Vec<(f64, f64)>,
    pub derivative: Vec<(f64, f64)>,
    pub orbit_id: u32,
    pub max_iterations: u32,
    pub escaped_at: Option<u32>,
}

/// Type alias for orbit complete callback.
type OrbitCompleteCallback = Rc<RefCell<Option<Box<dyn Fn(OrbitCompleteData)>>>>;

/// Pending reference orbit computation request.
struct PendingOrbitRequest {
    request: OrbitRequest,
}

pub struct WorkerPool {
    workers: Vec<Worker>,
    renderer_id: String,
    initialized_workers: HashSet<usize>,
    pending_tiles: VecDeque<PixelRect>,
    current_render_id: u32,
    current_viewport: Option<Viewport>,
    canvas_size: (u32, u32),
    on_tile_complete: Rc<dyn Fn(TileResult)>,
    on_render_complete: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    on_orbit_complete: OrbitCompleteCallback,
    progress: RwSignal<RenderProgress>,
    render_start_time: Option<f64>,
    self_ref: Weak<RefCell<Self>>,
    /// Perturbation coordinator (handles state, glitch resolution, tile messages)
    perturbation: PerturbationCoordinator,
    /// Whether current render is using perturbation mode
    is_perturbation_render: bool,
    /// GPU mode: orbit complete callback handles rendering, skip tile dispatch
    gpu_mode: bool,
    /// Pending orbit computation (waiting for worker to initialize)
    pending_orbit_request: Option<PendingOrbitRequest>,
    /// Cached orbit data for callbacks
    pending_orbit_data: Option<OrbitData>,
}
```

**Step 2: Update WorkerPool::new**

Replace the `new` method:

```rust
impl WorkerPool {
    pub fn new<F>(
        renderer_id: &str,
        on_tile_complete: F,
        progress: RwSignal<RenderProgress>,
    ) -> Result<Rc<RefCell<Self>>, JsValue>
    where
        F: Fn(TileResult) + 'static,
    {
        let config_worker_count = get_config(renderer_id).map(|c| c.worker_count).unwrap_or(0);

        let worker_count = if config_worker_count == 0 {
            web_sys::window()
                .map(|w| w.navigator().hardware_concurrency() as usize)
                .unwrap_or(4)
                .max(1)
        } else {
            config_worker_count
        };

        let pool = Rc::new(RefCell::new(Self {
            workers: Vec::new(),
            renderer_id: renderer_id.to_string(),
            initialized_workers: HashSet::new(),
            pending_tiles: VecDeque::new(),
            current_render_id: 0,
            current_viewport: None,
            canvas_size: (0, 0),
            on_tile_complete: Rc::new(on_tile_complete),
            on_render_complete: Rc::new(RefCell::new(None)),
            on_orbit_complete: Rc::new(RefCell::new(None)),
            progress,
            render_start_time: None,
            self_ref: Weak::new(),
            perturbation: PerturbationCoordinator::new(renderer_id),
            is_perturbation_render: false,
            gpu_mode: false,
            pending_orbit_request: None,
            pending_orbit_data: None,
        }));

        pool.borrow_mut().self_ref = Rc::downgrade(&pool);

        let workers = create_workers(worker_count, Rc::clone(&pool))?;
        pool.borrow_mut().workers = workers;

        Ok(pool)
    }
```

This is getting very long. Let me save what I have and continue with remaining tasks.

**Step 3: Extract message handlers to separate methods**

Add these handler methods to the `impl WorkerPool` block (after the existing methods):

```rust
    fn handle_ready(&mut self, worker_id: usize) {
        self.send_to_worker(
            worker_id,
            &MainToWorker::Initialize {
                renderer_id: self.renderer_id.clone(),
            },
        );
    }

    fn handle_request_work(&mut self, worker_id: usize, render_id: Option<u32>) {
        if render_id.is_none() {
            let was_empty = self.initialized_workers.is_empty();
            self.initialized_workers.insert(worker_id);

            if was_empty {
                if let Some(pending) = self.pending_orbit_request.take() {
                    web_sys::console::log_1(
                        &"[WorkerPool] First worker ready, dispatching queued orbit request".into(),
                    );
                    self.send_to_worker(
                        worker_id,
                        &MainToWorker::ComputeReferenceOrbit {
                            render_id: pending.request.render_id,
                            orbit_id: pending.request.orbit_id,
                            c_ref_json: pending.request.c_ref_json,
                            max_iterations: pending.request.max_iterations,
                        },
                    );
                    return;
                }
            }
        }

        let should_send = match render_id {
            None => true,
            Some(id) => id == self.current_render_id,
        };

        if should_send {
            self.dispatch_work(worker_id);
        } else {
            self.send_to_worker(worker_id, &MainToWorker::NoWork);
        }
    }

    fn handle_tile_complete(
        &mut self,
        render_id: u32,
        tile: PixelRect,
        data: Vec<ComputeData>,
        compute_time_ms: f64,
    ) {
        if render_id != self.current_render_id {
            web_sys::console::warn_1(
                &format!(
                    "[WorkerPool] Ignoring stale tile from render #{} (current: #{})",
                    render_id, self.current_render_id
                )
                .into(),
            );
            return;
        }

        // Count glitched pixels (perturbation mode only)
        if self.is_perturbation_render {
            let glitched_count = data
                .iter()
                .filter(|d| matches!(d, ComputeData::Mandelbrot(m) if m.glitched))
                .count();

            if glitched_count > 0 {
                web_sys::console::log_1(
                    &format!(
                        "[WorkerPool] Tile ({},{}): {}/{} pixels glitched",
                        tile.x, tile.y, glitched_count, data.len()
                    )
                    .into(),
                );
                self.perturbation.glitch_resolver_mut().record_glitched_tile(tile);
            }
        }

        // Update progress
        let elapsed = self
            .render_start_time
            .map(|start| performance_now() - start)
            .unwrap_or(0.0);

        let is_complete = {
            let mut complete = false;
            self.progress.update(|p| {
                p.completed_steps += 1;
                p.elapsed_ms = elapsed;
                p.is_complete = p.completed_steps >= p.total_steps;
                complete = p.is_complete;
            });
            complete
        };

        if is_complete && self.is_perturbation_render {
            let total = self.progress.get_untracked().total_steps;
            let glitched_count = self.perturbation.glitch_resolver().glitched_tile_count();
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Render complete: {} tiles had glitches (of {} total)",
                    glitched_count, total
                )
                .into(),
            );
        }

        (self.on_tile_complete)(TileResult {
            tile,
            data,
            compute_time_ms,
        });

        if is_complete {
            if let Some(ref callback) = *self.on_render_complete.borrow() {
                callback();
            }
        }
    }

    fn handle_error(&self, worker_id: usize, message: String) {
        web_sys::console::error_1(&JsValue::from_str(&format!(
            "Worker {} error: {}",
            worker_id, message
        )));
    }

    fn handle_orbit_complete(
        &mut self,
        render_id: u32,
        orbit_id: u32,
        c_ref: (f64, f64),
        orbit: Vec<(f64, f64)>,
        derivative: Vec<(f64, f64)>,
        escaped_at: Option<u32>,
    ) {
        if render_id != self.current_render_id {
            return;
        }

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Reference orbit complete: {} points, escaped_at={:?}",
                orbit.len(),
                escaped_at
            )
            .into(),
        );

        let orbit_data = OrbitData {
            c_ref,
            orbit: orbit.clone(),
            derivative: derivative.clone(),
            escaped_at,
        };
        self.pending_orbit_data = Some(orbit_data.clone());

        // GPU mode: call callback and skip worker broadcast
        if self.gpu_mode {
            web_sys::console::log_1(&"[WorkerPool] GPU mode: triggering orbit callback".into());
            if let Some(callback) = self.on_orbit_complete.borrow().as_ref() {
                callback(OrbitCompleteData {
                    orbit,
                    derivative,
                    orbit_id,
                    max_iterations: self.perturbation.max_iterations(),
                    escaped_at,
                });
            }
            return;
        }

        // CPU mode: broadcast to all workers
        let msg = self.perturbation.build_orbit_broadcast(&orbit_data);
        for worker_id in 0..self.workers.len() {
            self.send_to_worker(worker_id, &msg);
        }
    }

    fn handle_orbit_stored(&mut self, worker_id: usize, orbit_id: u32) {
        // Check if this is a cell orbit (Phase 8)
        if self.perturbation.glitch_resolver().is_tracking_orbit(orbit_id) {
            web_sys::console::log_1(
                &format!("[WorkerPool] Worker {} stored orbit #{}", worker_id, orbit_id).into(),
            );

            let all_confirmed = self.perturbation.glitch_resolver_mut().confirm_orbit_stored(
                orbit_id,
                worker_id,
                &self.initialized_workers,
            );

            if all_confirmed {
                web_sys::console::log_1(
                    &format!(
                        "[WorkerPool] Phase 8: All workers confirmed orbit #{}",
                        orbit_id
                    )
                    .into(),
                );
            }
            return;
        }

        // Main perturbation orbit handling
        if orbit_id != self.perturbation.orbit_id() {
            return;
        }

        self.perturbation.record_worker_has_orbit(worker_id);

        if self.perturbation.all_workers_have_orbit(&self.initialized_workers)
            && !self.pending_tiles.is_empty()
        {
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] All {} workers have orbit, dispatching {} tiles",
                    self.perturbation.workers_with_orbit_count(),
                    self.pending_tiles.len()
                )
                .into(),
            );

            for worker_id in 0..self.workers.len() {
                if self.initialized_workers.contains(&worker_id) {
                    self.dispatch_work(worker_id);
                }
            }
        }
    }
```

**Step 4: Update handle_message to use handlers**

Replace the `handle_message` method:

```rust
    fn handle_message(&mut self, worker_id: usize, msg: WorkerToMain) {
        match msg {
            WorkerToMain::Ready => self.handle_ready(worker_id),
            WorkerToMain::RequestWork { render_id } => self.handle_request_work(worker_id, render_id),
            WorkerToMain::TileComplete {
                render_id,
                tile,
                data,
                compute_time_ms,
            } => self.handle_tile_complete(render_id, tile, data, compute_time_ms),
            WorkerToMain::Error { message } => self.handle_error(worker_id, message),
            WorkerToMain::ReferenceOrbitComplete {
                render_id,
                orbit_id,
                c_ref,
                orbit,
                derivative,
                escaped_at,
            } => self.handle_orbit_complete(render_id, orbit_id, c_ref, orbit, derivative, escaped_at),
            WorkerToMain::OrbitStored { orbit_id } => self.handle_orbit_stored(worker_id, orbit_id),
        }
    }
```

**Step 5: Update dispatch_work to use coordinator**

Replace the `dispatch_work` method:

```rust
    fn dispatch_work(&mut self, worker_id: usize) {
        if !self.initialized_workers.contains(&worker_id) {
            return;
        }

        if self.is_perturbation_render && !self.perturbation.worker_ready_for_tiles(worker_id) {
            self.send_to_worker(worker_id, &MainToWorker::NoWork);
            return;
        }

        if let Some(tile) = self.pending_tiles.pop_front() {
            if self.is_perturbation_render {
                if let Some(msg) = self.perturbation.build_tile_message(self.current_render_id, tile) {
                    self.send_to_worker(worker_id, &msg);
                } else {
                    self.send_to_worker(worker_id, &MainToWorker::NoWork);
                }
            } else {
                let tile_viewport = self
                    .current_viewport
                    .as_ref()
                    .map(|vp| crate::rendering::tile_to_viewport(&tile, vp, self.canvas_size));

                let viewport_json = tile_viewport
                    .and_then(|v| serde_json::to_string(&v).ok())
                    .unwrap_or_default();

                self.send_to_worker(
                    worker_id,
                    &MainToWorker::RenderTile {
                        render_id: self.current_render_id,
                        viewport_json,
                        tile,
                    },
                );
            }
        } else {
            self.send_to_worker(worker_id, &MainToWorker::NoWork);
        }
    }
```

**Step 6: Update start_perturbation_render**

Replace the `start_perturbation_render` method:

```rust
    pub fn start_perturbation_render(
        &mut self,
        viewport: Viewport,
        canvas_size: (u32, u32),
        tiles: Vec<PixelRect>,
    ) {
        self.is_perturbation_render = true;
        self.gpu_mode = false;
        self.current_render_id = self.current_render_id.wrapping_add(1);

        // Prepare coordinator
        let orbit_request = match self.perturbation.start_render(self.current_render_id, &viewport, canvas_size) {
            Ok(req) => req,
            Err(e) => {
                web_sys::console::error_1(&format!("[WorkerPool] {}", e).into());
                return;
            }
        };

        let zoom_exponent = (4.0 / viewport.width.to_f64()).log10();
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Starting perturbation render #{} with {} tiles, zoom=10^{:.1}, max_iter={}",
                self.current_render_id,
                tiles.len(),
                zoom_exponent,
                orbit_request.max_iterations
            )
            .into(),
        );

        self.current_viewport = Some(viewport);
        self.canvas_size = canvas_size;
        self.pending_tiles = tiles.into();
        self.render_start_time = Some(performance_now());

        let total = self.pending_tiles.len() as u32;
        self.progress.set(RenderProgress::new(total));

        // Send ComputeReferenceOrbit to first available worker, or queue
        if let Some(&worker_id) = self.initialized_workers.iter().next() {
            self.pending_orbit_request = None;
            self.send_to_worker(
                worker_id,
                &MainToWorker::ComputeReferenceOrbit {
                    render_id: orbit_request.render_id,
                    orbit_id: orbit_request.orbit_id,
                    c_ref_json: orbit_request.c_ref_json,
                    max_iterations: orbit_request.max_iterations,
                },
            );
        } else {
            web_sys::console::log_1(
                &"[WorkerPool] No workers initialized yet, queueing orbit request".into(),
            );
            self.pending_orbit_request = Some(PendingOrbitRequest { request: orbit_request });
        }
    }
```

**Step 7: Update compute_orbit_for_gpu**

Replace the `compute_orbit_for_gpu` method:

```rust
    pub fn compute_orbit_for_gpu(&mut self, viewport: Viewport, canvas_size: (u32, u32)) {
        self.gpu_mode = true;
        self.is_perturbation_render = false;
        self.current_render_id = self.current_render_id.wrapping_add(1);

        let orbit_request = match self.perturbation.start_gpu_render(self.current_render_id, &viewport, canvas_size) {
            Ok(req) => req,
            Err(e) => {
                web_sys::console::error_1(&format!("[WorkerPool] {}", e).into());
                return;
            }
        };

        let zoom_exponent = (4.0 / viewport.width.to_f64()).log10();
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Computing orbit for GPU render #{}, zoom=10^{:.1}, max_iter={}",
                self.current_render_id, zoom_exponent, orbit_request.max_iterations
            )
            .into(),
        );

        self.current_viewport = Some(viewport);
        self.canvas_size = canvas_size;
        self.render_start_time = Some(performance_now());

        if let Some(&worker_id) = self.initialized_workers.iter().next() {
            self.pending_orbit_request = None;
            self.send_to_worker(
                worker_id,
                &MainToWorker::ComputeReferenceOrbit {
                    render_id: orbit_request.render_id,
                    orbit_id: orbit_request.orbit_id,
                    c_ref_json: orbit_request.c_ref_json,
                    max_iterations: orbit_request.max_iterations,
                },
            );
        } else {
            web_sys::console::log_1(
                &"[WorkerPool] No workers initialized yet, queueing orbit request".into(),
            );
            self.pending_orbit_request = Some(PendingOrbitRequest { request: orbit_request });
        }
    }
```

**Step 8: Update cancel and subdivide_glitched_cells**

Replace `cancel`:

```rust
    pub fn cancel(&mut self) {
        let pending_count = self.pending_tiles.len();
        if pending_count == 0 && self.progress.get_untracked().is_complete {
            return;
        }

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Cancelling render #{}, {} tiles pending - terminating workers",
                self.current_render_id, pending_count
            )
            .into(),
        );

        self.is_perturbation_render = false;
        self.perturbation.reset();

        self.recreate_workers();

        self.progress.update(|p| {
            p.is_complete = true;
        });

        self.current_render_id = self.current_render_id.wrapping_add(1);
    }
```

Replace `subdivide_glitched_cells`:

```rust
    pub fn subdivide_glitched_cells(&mut self) {
        let subdivided = self.perturbation.glitch_resolver_mut().subdivide_glitched_cells();

        if subdivided == 0 {
            web_sys::console::log_1(&"[WorkerPool] No cells subdivided".into());
            return;
        }

        web_sys::console::log_1(
            &format!("[WorkerPool] Subdivided {} cells with glitched tiles", subdivided).into(),
        );

        // Log leaves with glitch counts
        for (bounds, count) in self.perturbation.glitch_resolver().leaves_with_glitch_counts() {
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Cell ({},{})-({},{}): {} glitched tiles",
                    bounds.x, bounds.y, bounds.x + bounds.width, bounds.y + bounds.height, count
                )
                .into(),
            );
        }

        // Phase 7: Compute reference orbits for cell centers
        self.compute_orbits_for_glitched_cells();
    }

    fn compute_orbits_for_glitched_cells(&mut self) {
        let Some(viewport) = self.current_viewport.clone() else {
            web_sys::console::log_1(
                &"[WorkerPool] No viewport available, cannot compute cell center orbits".into(),
            );
            return;
        };

        let max_iterations = self.perturbation.max_iterations();
        let start_time = performance_now();

        let computed = self
            .perturbation
            .glitch_resolver_mut()
            .compute_cell_orbits(&viewport, self.canvas_size, max_iterations);

        let elapsed = performance_now() - start_time;
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Phase 7: Computed {} reference orbits in {:.1}ms",
                computed, elapsed
            )
            .into(),
        );

        // Phase 8: Broadcast orbits to workers
        self.broadcast_cell_orbits_to_workers();
    }

    fn broadcast_cell_orbits_to_workers(&mut self) {
        let dc_max = self.perturbation.dc_max();
        let bla_enabled = self.perturbation.bla_enabled();
        let broadcasts = self.perturbation.glitch_resolver_mut().orbits_to_broadcast(dc_max, bla_enabled);

        if broadcasts.is_empty() {
            web_sys::console::log_1(&"[WorkerPool] Phase 8: No cell orbits to distribute".into());
            return;
        }

        let start_time = performance_now();

        for (orbit_id, msg) in &broadcasts {
            for worker_id in 0..self.workers.len() {
                self.send_to_worker(worker_id, msg);
            }
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Phase 8: Broadcasting orbit #{} to {} workers",
                    orbit_id, self.workers.len()
                )
                .into(),
            );
        }

        let elapsed = performance_now() - start_time;
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Phase 8: Broadcast {} cell orbits in {:.1}ms",
                broadcasts.len(), elapsed
            )
            .into(),
        );
    }
```

**Step 9: Update switch_renderer and getters**

```rust
    pub fn switch_renderer(&mut self, renderer_id: &str) {
        self.renderer_id = renderer_id.to_string();
        self.perturbation.set_renderer_id(renderer_id);
        self.recreate_workers();
    }

    pub fn get_orbit(&self) -> Option<(Vec<(f64, f64)>, u32)> {
        self.pending_orbit_data
            .as_ref()
            .map(|o| (o.orbit.clone(), self.perturbation.orbit_id()))
    }

    pub fn get_max_iterations(&self) -> u32 {
        self.perturbation.max_iterations()
    }
```

**Step 10: Remove old types and unused code**

Remove these items that are no longer needed (they're now in the perturbation module):
- `OrbitData` struct (now in coordinator.rs)
- `PerturbationState` struct (now internal to coordinator.rs)
- `PendingOrbitRequest` struct fields (replaced with simpler version)
- `calculate_dc_max` function (moved to helpers.rs)

**Step 11: Run tests**

Run: `cargo test --workspace -q`

Expected: All tests pass

**Step 12: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`

Expected: No warnings

**Step 13: Commit**

```bash
git add fractalwonder-ui/src/workers/
git commit -m "refactor(workers): integrate PerturbationCoordinator into WorkerPool

- WorkerPool now delegates perturbation state to PerturbationCoordinator
- Message handlers extracted to separate methods
- Glitch resolution delegated to GlitchResolver
- Removed ~400 lines of duplicated/inline code"
```

---

## Task 5: Final Cleanup and Verification

**Files:**
- Review: `fractalwonder-ui/src/workers/worker_pool.rs`
- Review: `fractalwonder-ui/src/workers/perturbation/`

**Step 1: Verify line counts**

Run: `wc -l fractalwonder-ui/src/workers/*.rs fractalwonder-ui/src/workers/perturbation/*.rs`

Expected output (approximate):
```
  500 fractalwonder-ui/src/workers/worker_pool.rs
  726 fractalwonder-ui/src/workers/quadtree.rs
    6 fractalwonder-ui/src/workers/mod.rs
  350 fractalwonder-ui/src/workers/perturbation/coordinator.rs
  250 fractalwonder-ui/src/workers/perturbation/glitch_resolution.rs
   80 fractalwonder-ui/src/workers/perturbation/helpers.rs
    8 fractalwonder-ui/src/workers/perturbation/mod.rs
```

**Step 2: Run full test suite**

Run: `cargo test --workspace --all-targets --all-features -- --nocapture`

Expected: All tests pass

**Step 3: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings -W clippy::all`

Expected: No warnings

**Step 4: Build check**

Run: `cargo check --workspace --all-targets --all-features`

Expected: No errors

**Step 5: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "chore(workers): cleanup after refactoring" --allow-empty
```

---

## Summary

| Before | After | Change |
|--------|-------|--------|
| worker_pool.rs: 1291 lines | worker_pool.rs: ~500 lines | -791 lines |
| No tests | helpers.rs + glitch_resolution.rs + coordinator.rs tests | +30 tests |
| 5 mixed concerns | 4 focused modules | Clear separation |

**New module structure:**
```
workers/
├── mod.rs
├── worker_pool.rs        (500 lines - lifecycle + routing)
├── quadtree.rs           (726 lines - unchanged)
└── perturbation/
    ├── mod.rs            (8 lines)
    ├── helpers.rs        (80 lines - pure functions)
    ├── glitch_resolution.rs (250 lines - Phase 7-8)
    └── coordinator.rs    (350 lines - state + orchestration)
```
