use crate::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};
use crate::rendering::{
    coords::{ImageCoord, ImageRect, PixelCoord},
    renderer_trait::CanvasRenderer,
    transforms::pixel_to_image,
};
use leptos::*;
use wasm_bindgen::JsValue;

pub struct TestImageRenderer {
    checkerboard_size: f64,
    circle_radius_step: f64,
    circle_line_thickness: f64,
}

impl TestImageRenderer {
    pub fn new() -> Self {
        Self {
            checkerboard_size: 10.0,
            circle_radius_step: 10.0,
            circle_line_thickness: 0.5,
        }
    }

    fn compute_pixel_color(&self, x: f64, y: f64) -> (u8, u8, u8, u8) {
        // Check if on circle first (circles drawn on top)
        let distance = (x * x + y * y).sqrt();
        let nearest_ring = (distance / self.circle_radius_step).round();
        let ring_distance = (distance - nearest_ring * self.circle_radius_step).abs();

        if ring_distance < self.circle_line_thickness / 2.0 && nearest_ring > 0.0 {
            return (255, 0, 0, 255); // Red circle
        }

        // Checkerboard: (0,0) is corner of four squares
        let square_x = (x / self.checkerboard_size).floor() as i32;
        let square_y = (y / self.checkerboard_size).floor() as i32;
        let is_light = (square_x + square_y) % 2 == 0;

        if is_light {
            (255, 255, 255, 255) // White
        } else {
            (204, 204, 204, 255) // Light grey
        }
    }
}

impl CanvasRenderer for TestImageRenderer {
    type Coord = f64;

    fn natural_bounds(&self) -> ImageRect<f64> {
        ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0))
    }

    fn render(&self, target_rect: &ImageRect<f64>, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        for py in 0..height {
            for px in 0..width {
                // Map pixel to image coordinates using centralized transform
                let pixel = PixelCoord::new(px as f64, py as f64);
                let image_coord = pixel_to_image(pixel, target_rect, width, height);

                let color = self.compute_pixel_color(*image_coord.x(), *image_coord.y());

                let idx = ((py * width + px) * 4) as usize;
                pixels[idx] = color.0; // R
                pixels[idx + 1] = color.1; // G
                pixels[idx + 2] = color.2; // B
                pixels[idx + 3] = color.3; // A
            }
        }

        pixels
    }
}

#[component]
pub fn TestImageView() -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Set up interaction hook with console logging
    let handle = use_canvas_interaction(canvas_ref, move |result: TransformResult| {
        let msg = format!(
            "Interaction ended: offset=({:.2}, {:.2}), zoom={:.4}, matrix={:?}",
            result.offset_x, result.offset_y, result.zoom_factor, result.matrix
        );
        web_sys::console::log_1(&JsValue::from_str(&msg));
        // TODO: Trigger full re-render with transformation
    });

    // Initialize canvas on mount
    create_effect(move |_| {
        if let Some(canvas) = canvas_ref.get() {
            let window = web_sys::window().expect("should have window");
            canvas.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            // Initial render - draw test pattern
            render_test_pattern(&canvas);
        }
    });

    // Manually attach wheel event listener with passive: false
    // This is required to allow preventDefault() on wheel events
    create_effect({
        move |_| {
            if let Some(canvas) = canvas_ref.get() {
                use wasm_bindgen::JsCast;
                use web_sys::AddEventListenerOptions;

                let options = AddEventListenerOptions::new();
                options.set_passive(false);

                let on_wheel = handle.on_wheel.clone();
                let closure =
                    wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::WheelEvent| {
                        (on_wheel)(ev);
                    })
                        as Box<dyn Fn(web_sys::WheelEvent) + 'static>);

                canvas
                    .add_event_listener_with_callback_and_add_event_listener_options(
                        "wheel",
                        closure.as_ref().unchecked_ref(),
                        &options,
                    )
                    .expect("should add wheel listener");

                closure.forget();
            }
        }
    });

    view! {
        <div class="relative w-full h-full">
            <canvas
                node_ref=canvas_ref
                class="block w-full h-full"
                on:pointerdown=move |ev| (handle.on_pointer_down)(ev)
                on:pointermove=move |ev| (handle.on_pointer_move)(ev)
                on:pointerup=move |ev| (handle.on_pointer_up)(ev)
                style="touch-action: none; cursor: grab;"
            />
            <Show when=move || handle.is_interacting.get()>
                <div class="absolute top-4 left-4 bg-blue-600 text-white px-4 py-2 rounded-lg shadow-lg">
                    "Interacting..."
                </div>
            </Show>
        </div>
    }
}

// Helper function to render the test pattern on canvas
fn render_test_pattern(canvas: &web_sys::HtmlCanvasElement) {
    use wasm_bindgen::JsCast;
    use web_sys::CanvasRenderingContext2d;

    let context = canvas
        .get_context("2d")
        .expect("should get 2d context")
        .expect("context should not be null")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("should cast to CanvasRenderingContext2d");

    let width = canvas.width();
    let height = canvas.height();

    // Create a simple test pattern
    let renderer = TestImageRenderer::new();
    let bounds = renderer.natural_bounds();

    // Calculate visible bounds (centered at origin, zoom 1.0)
    use crate::rendering::{transforms::calculate_visible_bounds, viewport::Viewport};

    let viewport = Viewport {
        center: ImageCoord::new(0.0, 0.0),
        zoom: 1.0,
        natural_bounds: bounds,
    };

    let visible_bounds = calculate_visible_bounds(&viewport, width, height);

    // Render the pattern
    let pixel_data = renderer.render(&visible_bounds, width, height);

    // Put pixels on canvas
    let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&pixel_data),
        width,
        height,
    )
    .expect("should create ImageData");

    context
        .put_image_data(&image_data, 0.0, 0.0)
        .expect("should put image data");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_natural_bounds() {
        let renderer = TestImageRenderer::new();
        let bounds = renderer.natural_bounds();
        assert_eq!(*bounds.min.x(), -50.0);
        assert_eq!(*bounds.max.x(), 50.0);
    }

    #[test]
    fn test_renderer_produces_correct_pixel_count() {
        let renderer = TestImageRenderer::new();
        let bounds = ImageRect::new(ImageCoord::new(-10.0, -10.0), ImageCoord::new(10.0, 10.0));
        let pixels = renderer.render(&bounds, 100, 100);
        assert_eq!(pixels.len(), 100 * 100 * 4);
    }

    #[test]
    fn test_checkerboard_pattern_at_origin() {
        let renderer = TestImageRenderer::new();

        // Point at (-5, -5) should be in one square
        let color1 = renderer.compute_pixel_color(-5.0, -5.0);
        // Point at (5, 5) should be in same color (both negative square indices sum to even)
        let color2 = renderer.compute_pixel_color(5.0, 5.0);
        // Point at (5, -5) should be opposite color
        let color3 = renderer.compute_pixel_color(5.0, -5.0);

        assert_eq!(color1, color2);
        assert_ne!(color1, color3);
    }

    #[test]
    fn test_circle_at_radius_10() {
        let renderer = TestImageRenderer::new();

        // Point exactly on circle (radius 10)
        let color_on = renderer.compute_pixel_color(10.0, 0.0);
        assert_eq!(color_on, (255, 0, 0, 255)); // Red

        // Point between circles
        let color_off = renderer.compute_pixel_color(15.0, 0.0);
        assert_ne!(color_off, (255, 0, 0, 255)); // Not red
    }

    #[test]
    fn test_origin_is_corner_of_four_squares() {
        let renderer = TestImageRenderer::new();

        // (0,0) is corner, so nearby points in different quadrants have different colors
        let q1 = renderer.compute_pixel_color(1.0, 1.0);
        let q2 = renderer.compute_pixel_color(-1.0, 1.0);
        let q3 = renderer.compute_pixel_color(-1.0, -1.0);
        let q4 = renderer.compute_pixel_color(1.0, -1.0);

        // Opposite quadrants should have same color
        assert_eq!(q1, q3);
        assert_eq!(q2, q4);
        assert_ne!(q1, q2);
    }
}
