use crate::{ComputeData, PixelRect};
use serde::{Deserialize, Serialize};

/// Messages sent from main thread to worker.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MainToWorker {
    /// Initialize worker with specified renderer type.
    Initialize { renderer_id: String },

    /// Render a tile. viewport_json is JSON-serialized Viewport to preserve BigFloat precision.
    RenderTile {
        render_id: u32,
        viewport_json: String,
        tile: PixelRect,
    },

    /// No work available - worker should idle.
    NoWork,

    /// Terminate worker.
    Terminate,

    /// Compute a reference orbit at high precision.
    ComputeReferenceOrbit {
        render_id: u32,
        orbit_id: u32,
        c_ref_json: String,
        max_iterations: u32,
    },

    /// Store a reference orbit for use in tile rendering.
    StoreReferenceOrbit {
        orbit_id: u32,
        c_ref: (f64, f64),
        orbit: Vec<(f64, f64)>,
        escaped_at: Option<u32>,
    },

    /// Render a tile using perturbation.
    RenderTilePerturbation {
        render_id: u32,
        tile: PixelRect,
        orbit_id: u32,
        delta_c_origin: (f64, f64),
        delta_c_step: (f64, f64),
        max_iterations: u32,
    },

    /// Discard a cached orbit.
    DiscardOrbit { orbit_id: u32 },
}

/// Messages sent from worker to main thread.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WorkerToMain {
    /// Worker is loaded and ready for initialization.
    Ready,

    /// Worker requests work assignment.
    /// render_id is None after Initialize, Some(id) after completing work for that render.
    RequestWork { render_id: Option<u32> },

    /// Worker completed a tile.
    TileComplete {
        render_id: u32,
        tile: PixelRect,
        data: Vec<ComputeData>,
        compute_time_ms: f64,
    },

    /// Worker encountered an error.
    Error { message: String },

    /// Reference orbit computation complete.
    ReferenceOrbitComplete {
        render_id: u32,
        orbit_id: u32,
        c_ref: (f64, f64),
        orbit: Vec<(f64, f64)>,
        escaped_at: Option<u32>,
    },

    /// Orbit stored and ready.
    OrbitStored { orbit_id: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_to_worker_initialize_roundtrip() {
        let msg = MainToWorker::Initialize {
            renderer_id: "mandelbrot".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Initialize""#));
        assert!(json.contains(r#""renderer_id":"mandelbrot""#));

        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::Initialize { renderer_id } => assert_eq!(renderer_id, "mandelbrot"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn main_to_worker_render_tile_roundtrip() {
        let msg = MainToWorker::RenderTile {
            render_id: 42,
            viewport_json: r#"{"center":...}"#.to_string(),
            tile: PixelRect::new(10, 20, 64, 64),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::RenderTile {
                render_id, tile, ..
            } => {
                assert_eq!(render_id, 42);
                assert_eq!(tile.x, 10);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn worker_to_main_ready_roundtrip() {
        let msg = WorkerToMain::Ready;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Ready""#));

        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, WorkerToMain::Ready));
    }

    #[test]
    fn worker_to_main_tile_complete_roundtrip() {
        use crate::MandelbrotData;

        let msg = WorkerToMain::TileComplete {
            render_id: 1,
            tile: PixelRect::new(0, 0, 64, 64),
            data: vec![ComputeData::Mandelbrot(MandelbrotData {
                iterations: 100,
                max_iterations: 1000,
                escaped: true,
            })],
            compute_time_ms: 12.5,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        match parsed {
            WorkerToMain::TileComplete {
                render_id, data, ..
            } => {
                assert_eq!(render_id, 1);
                assert_eq!(data.len(), 1);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn compute_reference_orbit_roundtrip() {
        let msg = MainToWorker::ComputeReferenceOrbit {
            render_id: 1,
            orbit_id: 42,
            c_ref_json: r#"{"x":"-0.5","y":"0.0"}"#.to_string(),
            max_iterations: 10000,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::ComputeReferenceOrbit {
                orbit_id,
                max_iterations,
                ..
            } => {
                assert_eq!(orbit_id, 42);
                assert_eq!(max_iterations, 10000);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn store_reference_orbit_roundtrip() {
        let msg = MainToWorker::StoreReferenceOrbit {
            orbit_id: 1,
            c_ref: (-0.5, 0.0),
            orbit: vec![(0.0, 0.0), (-0.5, 0.0), (-0.25, 0.0)],
            escaped_at: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::StoreReferenceOrbit {
                orbit_id, orbit, ..
            } => {
                assert_eq!(orbit_id, 1);
                assert_eq!(orbit.len(), 3);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn render_tile_perturbation_roundtrip() {
        let msg = MainToWorker::RenderTilePerturbation {
            render_id: 1,
            tile: PixelRect::new(0, 0, 64, 64),
            orbit_id: 42,
            delta_c_origin: (0.001, -0.002),
            delta_c_step: (0.0001, 0.0001),
            max_iterations: 10000,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::RenderTilePerturbation {
                orbit_id,
                delta_c_origin,
                ..
            } => {
                assert_eq!(orbit_id, 42);
                assert!((delta_c_origin.0 - 0.001).abs() < 1e-10);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn reference_orbit_complete_roundtrip() {
        let msg = WorkerToMain::ReferenceOrbitComplete {
            render_id: 1,
            orbit_id: 42,
            c_ref: (-0.5, 0.0),
            orbit: vec![(0.0, 0.0), (-0.5, 0.0)],
            escaped_at: Some(1000),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        match parsed {
            WorkerToMain::ReferenceOrbitComplete {
                orbit_id,
                escaped_at,
                ..
            } => {
                assert_eq!(orbit_id, 42);
                assert_eq!(escaped_at, Some(1000));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn orbit_stored_roundtrip() {
        let msg = WorkerToMain::OrbitStored { orbit_id: 42 };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        match parsed {
            WorkerToMain::OrbitStored { orbit_id } => assert_eq!(orbit_id, 42),
            _ => panic!("Wrong variant"),
        }
    }
}
