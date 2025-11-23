use leptos::*;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Calculate gradient color for a pixel position.
/// R increases left-to-right, G increases top-to-bottom, B constant at 128.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn gradient_color(x: u32, y: u32, width: u32, height: u32) -> [u8; 4] {
    let r = ((x as f64 / width as f64) * 255.0) as u8;
    let g = ((y as f64 / height as f64) * 255.0) as u8;
    let b = 128u8;
    let a = 255u8;
    [r, g, b, a]
}

#[component]
pub fn InteractiveCanvas() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    create_effect(move |_| {
        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        // Set canvas dimensions to fill viewport
        let window = web_sys::window().expect("should have window");
        let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
        let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
        canvas.set_width(width);
        canvas.set_height(height);

        // Get 2D rendering context
        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .unchecked_into::<CanvasRenderingContext2d>();

        // Create pixel buffer and fill with gradient
        let mut data = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let [r, g, b, a] = gradient_color(x, y, width, height);
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = a;
            }
        }

        // Create ImageData and draw to canvas
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&data), width, height)
            .expect("should create ImageData");
        ctx.put_image_data(&image_data, 0.0, 0.0)
            .expect("should put image data");
    });

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_top_left_is_green_blue() {
        let [r, g, b, a] = gradient_color(0, 0, 100, 100);
        assert_eq!(r, 0, "top-left red should be 0");
        assert_eq!(g, 0, "top-left green should be 0");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }

    #[test]
    fn gradient_bottom_right_is_red_green_blue() {
        let [r, g, b, a] = gradient_color(99, 99, 100, 100);
        // 99/100 * 255 = 252.45 -> 252
        assert_eq!(r, 252, "bottom-right red should be ~252");
        assert_eq!(g, 252, "bottom-right green should be ~252");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }

    #[test]
    fn gradient_center_is_half_intensity() {
        let [r, g, b, a] = gradient_color(50, 50, 100, 100);
        // 50/100 * 255 = 127.5 -> 127
        assert_eq!(r, 127, "center red should be ~127");
        assert_eq!(g, 127, "center green should be ~127");
        assert_eq!(b, 128, "blue should be constant 128");
        assert_eq!(a, 255, "alpha should be 255");
    }
}
