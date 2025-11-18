use fractalwonder_core::{AppData, PixelRect};
use serde::{Deserialize, Serialize};

/// Messages sent from worker to main thread
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WorkerToMain {
    /// Worker is initialized and ready for commands
    Ready,

    /// Worker requests work assignment
    RequestWork {
        /// None = worker just started, will accept any work
        /// Some(id) = finished work for this render, wants more from same render
        render_id: Option<u32>,
    },

    /// Worker completed a tile
    TileComplete {
        render_id: u32,
        tile: PixelRect,
        data: Vec<AppData>,
        compute_time_ms: f64,
    },

    /// Worker encountered an error
    Error {
        render_id: Option<u32>,
        tile: Option<PixelRect>,
        error: String,
    },
}

/// Messages sent from main thread to worker
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MainToWorker {
    /// Assign tile to render
    RenderTile {
        render_id: u32,
        viewport_json: String,
        tile: PixelRect,
        canvas_width: u32,
        canvas_height: u32,
    },

    /// No work available
    NoWork,

    /// Terminate worker
    Terminate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ready_message_serialization() {
        let msg = WorkerToMain::Ready;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Ready\""));
    }

    #[test]
    fn test_ready_message_deserialization() {
        let json = r#"{"type":"Ready"}"#;
        let msg: WorkerToMain = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, WorkerToMain::Ready));
    }
}
