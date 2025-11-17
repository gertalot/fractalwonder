use crate::{
    atomics::{atomic_fetch_add_u32, atomic_load_u32},
    MandelbrotComputer, PixelRenderer, Renderer, SharedBufferLayout, WorkerRequest, WorkerResponse,
};
use fractalwonder_core::{MandelbrotData, PixelRect, Viewport};
use js_sys::{SharedArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;

/// Handle message from main thread
#[wasm_bindgen]
pub fn handle_message(event_data: JsValue) -> Result<(), JsValue> {
    // Parse the message object
    let request_str = js_sys::Reflect::get(&event_data, &JsValue::from_str("request"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("No request field"))?;

    let buffer = js_sys::Reflect::get(&event_data, &JsValue::from_str("buffer"))?
        .dyn_into::<js_sys::SharedArrayBuffer>()?;

    // Call existing process_render_request with the buffer
    process_render_request(request_str, buffer)
}

/// Worker initialization - called when worker starts
#[wasm_bindgen]
pub fn init_worker() {
    console_error_panic_hook::set_once();

    // Set up message handler
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        if let Err(err) = handle_message(e.data()) {
            web_sys::console::error_1(&err);
        }
    }) as Box<dyn FnMut(_)>);

    let global = js_sys::global()
        .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
        .unwrap();
    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Send ready message
    let response = WorkerResponse::Ready;
    if let Ok(message) = serde_json::to_string(&response) {
        global.post_message(&JsValue::from_str(&message)).ok();
    }
}

/// Process render request from main thread
#[wasm_bindgen]
pub fn process_render_request(
    message_json: String,
    shared_buffer: SharedArrayBuffer,
) -> Result<(), JsValue> {
    let request: WorkerRequest = serde_json::from_str(&message_json)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    match request {
        WorkerRequest::Render {
            viewport_json,
            canvas_width,
            canvas_height,
            render_id,
            tile_size,
        } => {
            let viewport: Viewport<f64> = serde_json::from_str(&viewport_json)
                .map_err(|e| JsValue::from_str(&format!("Parse viewport: {}", e)))?;

            compute_tiles(
                viewport,
                canvas_width,
                canvas_height,
                tile_size,
                render_id,
                shared_buffer,
            )
        }
        WorkerRequest::Terminate => {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
                    .dyn_into()
                    .expect("Failed to get worker global scope");
                global.close();
            }
            Ok(())
        }
    }
}

/// Compute all tiles using work-stealing pattern
fn compute_tiles(
    viewport: Viewport<f64>,
    width: u32,
    height: u32,
    tile_size: u32,
    render_id: u32,
    shared_buffer: SharedArrayBuffer,
) -> Result<(), JsValue> {
    let layout = SharedBufferLayout::new(width, height);
    let view = Uint8Array::new(&shared_buffer);

    // Generate all tiles
    let tiles = generate_tiles(width, height, tile_size);
    let total_tiles = tiles.len() as u32;

    // Create renderer
    let computer = MandelbrotComputer::<f64>::default();
    let renderer = PixelRenderer::new(computer);

    // Work-stealing loop
    loop {
        // Atomically get next tile index
        let tile_index = atomic_fetch_add_u32(&shared_buffer, layout.tile_index_offset() as u32, 1);

        if tile_index >= total_tiles {
            break; // All work done
        }

        // Check if render was cancelled
        let current_render_id = atomic_load_u32(&shared_buffer, layout.render_id_offset() as u32);
        if current_render_id != render_id {
            break; // Cancelled
        }

        let tile = &tiles[tile_index as usize];

        // Render tile
        let tile_data = renderer.render(&viewport, *tile, (width, height));

        // Write to shared buffer
        write_tile_to_buffer(&view, &layout, tile, &tile_data, width);

        // Increment completed tiles counter AFTER writing
        atomic_fetch_add_u32(&shared_buffer, layout.completed_tiles_offset() as u32, 1);

        // Notify main thread (optional - main polls buffer)
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            let response = WorkerResponse::TileComplete { tile_index };
            if let Ok(msg) = serde_json::to_string(&response) {
                let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global()
                    .dyn_into()
                    .expect("Failed to get worker global scope");

                global.post_message(&JsValue::from_str(&msg)).ok();
            }
        }
    }

    Ok(())
}

/// Generate tiles in spiral order (center-out)
fn generate_tiles(width: u32, height: u32, tile_size: u32) -> Vec<PixelRect> {
    let mut tiles = Vec::new();

    for y_start in (0..height).step_by(tile_size as usize) {
        for x_start in (0..width).step_by(tile_size as usize) {
            let x = x_start;
            let y = y_start;
            let w = tile_size.min(width - x_start);
            let h = tile_size.min(height - y_start);

            tiles.push(PixelRect::new(x, y, w, h));
        }
    }

    // Sort by distance from center (closest first)
    let canvas_center_x = width as f64 / 2.0;
    let canvas_center_y = height as f64 / 2.0;

    tiles.sort_by(|a, b| {
        let a_center_x = a.x as f64 + a.width as f64 / 2.0;
        let a_center_y = a.y as f64 + a.height as f64 / 2.0;
        let a_dist_sq =
            (a_center_x - canvas_center_x).powi(2) + (a_center_y - canvas_center_y).powi(2);

        let b_center_x = b.x as f64 + b.width as f64 / 2.0;
        let b_center_y = b.y as f64 + b.height as f64 / 2.0;
        let b_dist_sq =
            (b_center_x - canvas_center_x).powi(2) + (b_center_y - canvas_center_y).powi(2);

        a_dist_sq
            .partial_cmp(&b_dist_sq)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    tiles
}

/// Write tile data to shared buffer
fn write_tile_to_buffer(
    view: &Uint8Array,
    layout: &SharedBufferLayout,
    tile: &PixelRect,
    tile_data: &[MandelbrotData],
    canvas_width: u32,
) {
    for local_y in 0..tile.height {
        let canvas_y = tile.y + local_y;
        for local_x in 0..tile.width {
            let canvas_x = tile.x + local_x;
            let pixel_index = (canvas_y * canvas_width + canvas_x) as usize;
            let tile_data_index = (local_y * tile.width + local_x) as usize;

            // Encode pixel data
            let pixel = &tile_data[tile_data_index];
            let encoded = SharedBufferLayout::encode_pixel(pixel);

            // Write to buffer
            let offset = layout.pixel_offset(pixel_index);
            for (i, byte) in encoded.iter().enumerate() {
                view.set_index((offset + i) as u32, *byte);
            }
        }
    }
}
