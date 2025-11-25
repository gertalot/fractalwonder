use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Yield to browser event loop via requestAnimationFrame.
///
/// Returns a Future that resolves on the next animation frame,
/// allowing the browser to handle events and paint between tiles.
pub async fn yield_to_browser() {
    let (sender, receiver) = futures::channel::oneshot::channel::<()>();

    let closure = Closure::once(move || {
        let _ = sender.send(());
    });

    web_sys::window()
        .expect("should have window")
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .expect("should register rAF");

    closure.forget();
    let _ = receiver.await;
}

/// Get the current time in milliseconds (for elapsed time tracking).
pub fn performance_now() -> f64 {
    web_sys::window()
        .expect("should have window")
        .performance()
        .expect("should have performance")
        .now()
}

/// Get 2D rendering context from canvas.
pub fn get_2d_context(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, JsValue> {
    Ok(canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("No 2d context"))?
        .dyn_into::<CanvasRenderingContext2d>()?)
}

/// Draw RGBA pixel data to canvas at specified position.
pub fn draw_pixels_to_canvas(
    ctx: &CanvasRenderingContext2d,
    pixels: &[u8],
    width: u32,
    x: f64,
    y: f64,
) -> Result<(), JsValue> {
    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
        Clamped(pixels),
        width,
        pixels.len() as u32 / width / 4,
    )?;
    ctx.put_image_data(&image_data, x, y)
}

#[cfg(test)]
mod tests {
    // Note: These are browser-only functions, so unit tests are limited.
    // Real testing happens in wasm-pack browser tests.
}
