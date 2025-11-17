use fractalwonder_compute::WorkerRequest;
use js_sys::ArrayBuffer;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::Worker;

pub struct WorkerPool {
    workers: Vec<Worker>,
    shared_buffer: Option<ArrayBuffer>,
    current_render_id: Arc<AtomicU32>,
}

impl WorkerPool {
    pub fn new() -> Result<Self, JsValue> {
        // Get hardware concurrency (CPU core count)
        let worker_count = web_sys::window()
            .map(|w| w.navigator().hardware_concurrency() as usize)
            .unwrap_or(4);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating WorkerPool with {} workers",
            worker_count
        )));

        let workers = Vec::new();

        Ok(Self {
            workers,
            shared_buffer: None,
            current_render_id: Arc::new(AtomicU32::new(0)),
        })
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn current_render_id(&self) -> u32 {
        self.current_render_id.load(Ordering::SeqCst)
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        // Terminate all workers on cleanup
        for worker in &self.workers {
            let request = WorkerRequest::Terminate;
            if let Ok(message) = serde_json::to_string(&request) {
                worker.post_message(&JsValue::from_str(&message)).ok();
            }
        }
    }
}
