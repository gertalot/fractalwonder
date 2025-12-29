//! Glitch resolution methods for WorkerPool.

use super::worker_pool::WorkerPool;
use super::worker_pool_types::performance_now;

impl WorkerPool {
    /// Subdivide cells that contain glitched tiles for re-rendering.
    pub fn subdivide_glitched_cells(&mut self) {
        let subdivided = self
            .perturbation
            .glitch_resolver_mut()
            .subdivide_glitched_cells();
        if subdivided == 0 {
            web_sys::console::log_1(&"[WorkerPool] No cells subdivided".into());
            return;
        }

        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Subdivided {} cells with glitched tiles",
                subdivided
            )
            .into(),
        );
        for (bounds, count) in self
            .perturbation
            .glitch_resolver()
            .leaves_with_glitch_counts()
        {
            web_sys::console::log_1(
                &format!(
                    "[WorkerPool] Cell ({},{})-({},{}): {} glitched tiles",
                    bounds.x,
                    bounds.y,
                    bounds.x + bounds.width,
                    bounds.y + bounds.height,
                    count
                )
                .into(),
            );
        }
        self.compute_orbits_for_glitched_cells();
    }

    pub(super) fn compute_orbits_for_glitched_cells(&mut self) {
        let Some(viewport) = self.current_viewport.clone() else {
            web_sys::console::log_1(
                &"[WorkerPool] No viewport available, cannot compute cell center orbits".into(),
            );
            return;
        };

        let max_iterations = self.perturbation.max_iterations();
        let start_time = performance_now();
        let computed = self.perturbation.glitch_resolver_mut().compute_cell_orbits(
            &viewport,
            self.canvas_size,
            max_iterations,
        );
        let elapsed = performance_now() - start_time;
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Phase 7: Computed {} reference orbits in {:.1}ms",
                computed, elapsed
            )
            .into(),
        );
        self.broadcast_cell_orbits_to_workers();
    }

    pub(super) fn broadcast_cell_orbits_to_workers(&mut self) {
        let dc_max = self.perturbation.dc_max();
        let bla_enabled = self.perturbation.bla_enabled();
        let broadcasts = self
            .perturbation
            .glitch_resolver_mut()
            .orbits_to_broadcast(dc_max, bla_enabled);

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
                    orbit_id,
                    self.workers.len()
                )
                .into(),
            );
        }
        let elapsed = performance_now() - start_time;
        web_sys::console::log_1(
            &format!(
                "[WorkerPool] Phase 8: Broadcast {} cell orbits in {:.1}ms",
                broadcasts.len(),
                elapsed
            )
            .into(),
        );
    }
}
