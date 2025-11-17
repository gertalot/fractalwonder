use serde::{Deserialize, Serialize};

/// Message from main thread to worker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkerRequest {
    /// Render a viewport
    Render {
        viewport_json: String,  // Serialized Viewport<f64>
        canvas_width: u32,
        canvas_height: u32,
        render_id: u32,
        tile_size: u32,
    },

    /// Terminate worker
    Terminate,
}

/// Message from worker to main thread
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkerResponse {
    /// Worker initialized and ready
    Ready,

    /// Single tile completed
    TileComplete {
        tile_index: u32,
    },

    /// All tiles completed
    RenderComplete,

    /// Error occurred
    Error {
        message: String,
    },
}
