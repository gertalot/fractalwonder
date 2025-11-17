use fractalwonder_compute::{SharedBufferLayout, WorkerRequest};
use fractalwonder_core::Viewport;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, Worker};

pub struct WorkerPool {
    workers: Vec<Worker>,
    shared_buffer: Option<js_sys::SharedArrayBuffer>,
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

            web_sys::console::log_1(&JsValue::from_str(&format!("Worker {} created", i)));

            // Set up message handler for worker responses
            let worker_id = i;
            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Some(msg) = e.data().as_string() {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Worker {} message: {}",
                        worker_id, msg
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

    pub fn start_render(
        &mut self,
        viewport: &Viewport<f64>,
        canvas_width: u32,
        canvas_height: u32,
        tile_size: u32,
    ) -> Result<u32, JsValue> {
        // Increment render ID
        let render_id = self.current_render_id.fetch_add(1, Ordering::SeqCst) + 1;

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Starting render {} ({}x{}, tile_size={})",
            render_id, canvas_width, canvas_height, tile_size
        )));

        // Create SharedArrayBuffer layout
        let layout = SharedBufferLayout::new(canvas_width, canvas_height);
        let buffer_size = layout.buffer_size();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Creating SharedArrayBuffer of {} bytes",
            buffer_size
        )));

        // Create SharedArrayBuffer
        let shared_buffer = js_sys::SharedArrayBuffer::new(buffer_size as u32);
        self.shared_buffer = Some(shared_buffer.clone());

        // Initialize atomic counters in buffer
        let int32_array = js_sys::Int32Array::new(&shared_buffer);
        int32_array.set_index(0, 0); // tile_index counter = 0
        int32_array.set_index(1, render_id as i32); // render_id

        // Zero out pixel data
        let view = js_sys::Uint8Array::new(&shared_buffer);
        for i in 8..buffer_size {
            view.set_index(i as u32, 0);
        }

        // Serialize viewport to JSON
        let viewport_json = serde_json::to_string(viewport)
            .map_err(|e| JsValue::from_str(&format!("Serialize viewport error: {}", e)))?;

        // Create render request
        let request = WorkerRequest::Render {
            viewport_json,
            canvas_width,
            canvas_height,
            render_id,
            tile_size,
        };

        let message = serde_json::to_string(&request)
            .map_err(|e| JsValue::from_str(&format!("Serialize request error: {}", e)))?;

        // Send to all workers WITH SharedArrayBuffer
        for (i, worker) in self.workers.iter().enumerate() {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Sending render request to worker {}",
                i
            )));

            // Create message object with both the JSON request and the buffer
            let msg_obj = js_sys::Object::new();
            js_sys::Reflect::set(&msg_obj, &JsValue::from_str("request"), &JsValue::from_str(&message))?;
            js_sys::Reflect::set(&msg_obj, &JsValue::from_str("buffer"), &shared_buffer)?;

            worker.post_message(&msg_obj)?;
        }

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Render {} started on {} workers",
            render_id,
            self.workers.len()
        )));

        Ok(render_id)
    }

    pub fn get_shared_buffer(&self) -> Option<&js_sys::SharedArrayBuffer> {
        self.shared_buffer.as_ref()
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
