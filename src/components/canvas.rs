
use leptos::html::Canvas;
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

const SQUARE_SIZE: u32 = 32;
const COLOR_LIGHT: &str = "#ffffff";
const COLOR_DARK: &str = "#e0e0e0";

#[component]
pub fn Canvas() -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();

    // Draw checkerboard pattern
    let draw_checkerboard = move || {
        let canvas = canvas_ref.get().expect("canvas element should be mounted");
        let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();

        let context = canvas_element
            .get_context("2d")
            .expect("should get 2d context")
            .expect("context should not be null")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("should cast to CanvasRenderingContext2d");

        let width = canvas_element.width();
        let height = canvas_element.height();

        // Clear canvas
        context.clear_rect(0.0, 0.0, width as f64, height as f64);

        // Draw checkerboard
        let cols = width.div_ceil(SQUARE_SIZE);
        let rows = height.div_ceil(SQUARE_SIZE);

        for row in 0..rows {
            for col in 0..cols {
                let is_light = (row + col) % 2 == 0;
                let color = if is_light { COLOR_LIGHT } else { COLOR_DARK };

                context.set_fill_style_str(color);
                context.fill_rect(
                    (col * SQUARE_SIZE) as f64,
                    (row * SQUARE_SIZE) as f64,
                    SQUARE_SIZE as f64,
                    SQUARE_SIZE as f64,
                );
            }
        }
    };

    // Initialize canvas on mount
    create_effect(move |_| {
        if canvas_ref.get().is_some() {
            let canvas = canvas_ref.get().expect("canvas should exist");
            let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();

            let window = web_sys::window().expect("should have window");
            canvas_element.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas_element.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            draw_checkerboard();
        }
    });

    // Handle window resize
    let handle_resize = move || {
        if let Some(canvas) = canvas_ref.get() {
            let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();
            let window = web_sys::window().expect("should have window");

            canvas_element.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas_element.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            draw_checkerboard();
        }
    };

    let _ =
        leptos_use::use_event_listener(leptos_use::use_window(), leptos::ev::resize, move |_| {
            handle_resize();
        });

    view! {
      <canvas
        node_ref=canvas_ref
        class="block w-full h-full"
        style="touch-action: none; cursor: grab;"
      />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_square_size() {
        assert_eq!(SQUARE_SIZE, 32, "Square size should be 32x32 pixels");
    }

    #[test]
    fn test_checkerboard_colors() {
        assert_eq!(COLOR_LIGHT, "#ffffff", "Light color should be white");
        assert_eq!(COLOR_DARK, "#e0e0e0", "Dark color should be light gray");
    }

    #[test]
    fn test_checkerboard_pattern() {
        // Test that alternating pattern is correct
        for row in 0..10 {
            for col in 0..10 {
                let is_light = (row + col) % 2 == 0;
                let opposite_is_dark = (row + col + 1) % 2 == 0;
                assert_ne!(
                    is_light, opposite_is_dark,
                    "Adjacent squares should alternate"
                );
            }
        }
    }
}
