use crate::{MainToWorker, Renderer, WorkerToMain};
use fractalwonder_core::{AppData, BigFloat, PixelRect, Viewport};
use js_sys::Date;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

fn create_renderer(
    renderer_id: &str,
) -> Result<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>, JsValue> {
    crate::render_config::create_renderer(renderer_id)
        .ok_or_else(|| JsValue::from_str(&format!("Unknown renderer: {}", renderer_id)))
}

/// Message-based worker initialization
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();

    // No renderer created yet - wait for Initialize message
    let renderer: Rc<RefCell<Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>>>> =
        Rc::new(RefCell::new(None));

    // Set up message handler
    let renderer_clone = Rc::clone(&renderer);
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        if let Err(err) = handle_worker_message(&renderer_clone, e.data()) {
            web_sys::console::error_1(&JsValue::from_str(&format!("Worker error: {:?}", err)));
        }
    }) as Box<dyn FnMut(_)>);

    let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
        .dyn_into()
        .expect("Failed to get worker global scope");

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Signal ready - wait for Initialize message
    send_message(&WorkerToMain::Ready);
}

fn handle_worker_message(
    renderer: &Rc<RefCell<Option<Box<dyn Renderer<Scalar = BigFloat, Data = AppData>>>>>,
    data: JsValue,
) -> Result<(), JsValue> {
    let msg_str = data
        .as_string()
        .ok_or_else(|| JsValue::from_str("Message data is not a string"))?;

    let msg: MainToWorker = serde_json::from_str(&msg_str)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse message: {}", e)))?;

    match msg {
        MainToWorker::Initialize { renderer_id } => {
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "Initializing worker with renderer: {}",
                renderer_id
            )));

            let new_renderer = create_renderer(&renderer_id)?;
            *renderer.borrow_mut() = Some(new_renderer);

            // Now ready for work
            send_message(&WorkerToMain::RequestWork { render_id: None });
        }

        MainToWorker::RenderTile {
            render_id,
            viewport_json,
            tile,
            canvas_width,
            canvas_height,
        } => {
            let borrowed = renderer.borrow();
            let r = borrowed
                .as_ref()
                .ok_or_else(|| JsValue::from_str("Renderer not initialized"))?;

            handle_render_tile(
                r.as_ref(),
                render_id,
                viewport_json,
                tile,
                canvas_width,
                canvas_height,
            )?;
        }
        MainToWorker::NoWork => {
            // Render complete, go idle
        }
        MainToWorker::Terminate => {
            let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
                .dyn_into()
                .expect("Failed to get worker global scope");
            global.close();
        }
    }

    Ok(())
}

fn handle_render_tile(
    renderer: &dyn Renderer<Scalar = BigFloat, Data = AppData>,
    render_id: u32,
    viewport_json: String,
    tile: PixelRect,
    canvas_width: u32,
    canvas_height: u32,
) -> Result<(), JsValue> {
    let start_time = Date::now();

    // Parse viewport
    let viewport: Viewport<BigFloat> = serde_json::from_str(&viewport_json).map_err(|e| {
        let err_msg = format!("Failed to parse viewport: {}", e);
        send_error(Some(render_id), Some(tile), &err_msg);
        JsValue::from_str(&err_msg)
    })?;

    // Render tile
    let tile_data = renderer.render(&viewport, tile, (canvas_width, canvas_height));

    let compute_time_ms = Date::now() - start_time;

    // Send result
    send_tile_complete(render_id, tile, tile_data, compute_time_ms);

    // Request next work
    send_message(&WorkerToMain::RequestWork {
        render_id: Some(render_id),
    });

    Ok(())
}

fn send_message(msg: &WorkerToMain) {
    if let Ok(json) = serde_json::to_string(msg) {
        let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
            .dyn_into()
            .expect("Failed to get worker global scope");
        global.post_message(&JsValue::from_str(&json)).ok();
    }
}

fn send_tile_complete(render_id: u32, tile: PixelRect, data: Vec<AppData>, compute_time_ms: f64) {
    let msg_with_data = WorkerToMain::TileComplete {
        render_id,
        tile,
        data,
        compute_time_ms,
    };

    if let Ok(json_with_data) = serde_json::to_string(&msg_with_data) {
        let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
            .dyn_into()
            .expect("Failed to get worker global scope");
        global
            .post_message(&JsValue::from_str(&json_with_data))
            .ok();
    }
}

fn send_error(render_id: Option<u32>, tile: Option<PixelRect>, error: &str) {
    send_message(&WorkerToMain::Error {
        render_id,
        tile,
        error: error.to_string(),
    });
}
