use fractalwonder_compute::WorkerRequest;
use js_sys::ArrayBuffer;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker};

pub struct WorkerPool {
    workers: Vec<Worker>,
    #[allow(dead_code)]
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

        let mut workers = Vec::new();

        for i in 0..worker_count {
            // Worker script path (Trunk generates this in dist/)
            // Note: Not using WorkerType::Module because worker uses no-modules target
            let worker = Worker::new("./fractalwonder-compute.js")?;

            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Worker {} created",
                i
            )));

            // Set up message handler for worker responses
            let worker_id = i;
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Some(msg) = e.data().as_string() {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Worker {} message: {}",
                        worker_id,
                        msg
                    )));
                }
            }) as Box<dyn FnMut(_)>);

            worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget(); // Keep closure alive

            // Set up error handler
            let error_handler = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "Worker {} error: {:?}",
                    worker_id,
                    e.message()
                )));
            }) as Box<dyn FnMut(_)>);

            worker.set_onerror(Some(error_handler.as_ref().unchecked_ref()));
            error_handler.forget(); // Keep closure alive

            workers.push(worker);
        }

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
