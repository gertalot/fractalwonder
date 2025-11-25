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
}
