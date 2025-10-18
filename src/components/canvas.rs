
use leptos::html::Canvas;
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::rendering::{
    renderer_trait::CanvasRenderer, transforms::calculate_visible_bounds, viewport::Viewport,
};

#[component]
pub fn Canvas<R>(renderer: R, viewport: ReadSignal<Viewport<R::Coord>>) -> impl IntoView
where
    R: CanvasRenderer + 'static,
    R::Coord: Clone
        + std::ops::Sub<Output = R::Coord>
        + std::ops::Div<f64, Output = R::Coord>
        + std::ops::Mul<f64, Output = R::Coord>
        + std::ops::Add<Output = R::Coord>,
{
    let canvas_ref = NodeRef::<Canvas>::new();

    // Main render function
    let render = move || {
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

        // Calculate what image-space rectangle is visible
        let visible_bounds = calculate_visible_bounds(&viewport.get(), width, height);

        // Ask renderer for pixel data
        let pixel_data = renderer.render(&visible_bounds, width, height);

        // Put pixels on canvas
        let image_data =
            ImageData::new_with_u8_clamped_array_and_sh(wasm_bindgen::Clamped(&pixel_data), width, height)
                .expect("should create ImageData");

        context.put_image_data(&image_data, 0.0, 0.0).expect("should put image data");
    };

    // Initialize canvas on mount
    create_effect(move |_| {
        if canvas_ref.get().is_some() {
            let canvas = canvas_ref.get().expect("canvas should exist");
            let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();

            let window = web_sys::window().expect("should have window");
            canvas_element.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas_element.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            render();
        }
    });

    // Re-render when viewport changes
    create_effect(move |_| {
        viewport.track(); // Track viewport signal
        if canvas_ref.get().is_some() {
            render();
        }
    });

    // Handle window resize
    let handle_resize = move || {
        if let Some(canvas) = canvas_ref.get() {
            let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();
            let window = web_sys::window().expect("should have window");

            canvas_element.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas_element.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            render();
        }
    };

    let _ = leptos_use::use_event_listener(leptos_use::use_window(), leptos::ev::resize, move |_| {
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
