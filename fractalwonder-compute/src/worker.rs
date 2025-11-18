use crate::{AdaptiveMandelbrotRenderer, MainToWorker, Renderer, WorkerToMain};
use fractalwonder_core::{AppData, BigFloat, PixelRect, Viewport};
use js_sys::Date;
use wasm_bindgen::prelude::*;

/// Message-based worker initialization
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();

    // Create adaptive renderer once at startup
    let renderer = AdaptiveMandelbrotRenderer::new(1e10);

    // Set up message handler
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        if let Err(err) = handle_worker_message(&renderer, e.data()) {
            web_sys::console::error_1(&JsValue::from_str(&format!("Worker error: {:?}", err)));
        }
    }) as Box<dyn FnMut(_)>);

    let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
        .dyn_into()
        .expect("Failed to get worker global scope");

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Request work immediately (no "Ready" message)
    send_message(&WorkerToMain::RequestWork { render_id: None });
}

fn handle_worker_message(
    renderer: &AdaptiveMandelbrotRenderer,
    data: JsValue,
) -> Result<(), JsValue> {
    let msg_str = data
        .as_string()
        .ok_or_else(|| JsValue::from_str("Message data is not a string"))?;

    let msg: MainToWorker = serde_json::from_str(&msg_str)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse message: {}", e)))?;

    match msg {
        MainToWorker::Initialize { renderer_id: _ } => {
            // TODO: Create renderer based on renderer_id (Task 6)
            // For now, just log to maintain compilation
            web_sys::console::log_1(&JsValue::from_str(
                "Initialize message received (not yet implemented)",
            ));
        }

        MainToWorker::RenderTile {
            render_id,
            viewport_json,
            tile,
            canvas_width,
            canvas_height,
        } => {
            handle_render_tile(
                renderer,
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
    renderer: &AdaptiveMandelbrotRenderer,
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
