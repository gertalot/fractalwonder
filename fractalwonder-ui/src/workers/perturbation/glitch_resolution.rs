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

    /// Get quadtree reference for testing.
    #[allow(dead_code)]
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
            let (c_ref_x, c_ref_y) = pixel_to_fractal(
                center_px_x,
                center_px_y,
                viewport,
                canvas_size,
                precision_bits,
            );

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
    pub fn orbits_to_broadcast(
        &mut self,
        dc_max: f64,
        bla_enabled: bool,
    ) -> Vec<(u32, MainToWorker)> {
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
