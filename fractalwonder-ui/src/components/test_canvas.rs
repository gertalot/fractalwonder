use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, ImageData};

#[component]
pub fn TestCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Render test pattern on mount and when canvas resizes
    create_effect(move |_| {
        if let Some(canvas_el) = canvas_ref.get() {
            // Set canvas to fill viewport
            let window = web_sys::window().unwrap();
            let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height = window.inner_height().unwrap().as_f64().unwrap() as u32;

            canvas_el.set_width(width);
            canvas_el.set_height(height);

            // Render test pattern: blue-orange gradient
            render_test_pattern(&canvas_el, width, height);
        }
    });

    view! {
        <canvas
            node_ref=canvas_ref
            style="display: block; width: 100vw; height: 100vh;"
        />
    }
}

fn render_test_pattern(
    canvas: &leptos::HtmlElement<leptos::html::Canvas>,
    width: u32,
    height: u32,
) {
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d").unwrap().unwrap().unchecked_into();

    // Create pixel buffer
    let pixel_count = (width * height * 4) as usize;
    let mut pixels = vec![0u8; pixel_count];

    // Generate gradient: blue (top-left) to orange (bottom-right)
    for y in 0..height {
        for x in 0..width {
            let t_x = x as f64 / width as f64;
            let t_y = y as f64 / height as f64;

            let r = (t_x * 255.0) as u8;
            let g = 128;
            let b = (t_y * 255.0) as u8;

            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = r;
            pixels[idx + 1] = g;
            pixels[idx + 2] = b;
            pixels[idx + 3] = 255; // Alpha
        }
    }

    // Put pixels on canvas
    let image_data =
        ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&pixels), width, height)
            .unwrap();

    ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
}
