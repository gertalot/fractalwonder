//! Types for the worker pool.

use crate::workers::perturbation::OrbitRequest;
use fractalwonder_core::{ComputeData, PixelRect};
use std::cell::RefCell;
use std::rc::Rc;

/// Result of a tile computation.
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
pub type OrbitCompleteCallback = Rc<RefCell<Option<Box<dyn Fn(OrbitCompleteData)>>>>;

/// Type alias for render complete callback.
pub type RenderCompleteCallback = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// Pending reference orbit computation request.
pub struct PendingOrbitRequest {
    pub request: OrbitRequest,
}

/// Get current performance timestamp in milliseconds.
pub fn performance_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}
