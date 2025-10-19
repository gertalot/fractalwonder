use leptos::html::Canvas;
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use crate::rendering::{render_with_viewport, renderer_trait::CanvasRenderer, viewport::Viewport};

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

    // Create a signal to trigger renders
    let (render_trigger, set_render_trigger) = create_signal(());

    // Main render effect
    create_effect(move |_| {
        render_trigger.track(); // Track render trigger

        if let Some(canvas) = canvas_ref.get() {
            let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();
            render_with_viewport(&canvas_element, &renderer, &viewport.get());
        }
    });

    // Initialize canvas on mount
    create_effect(move |_| {
        if canvas_ref.get().is_some() {
            let canvas = canvas_ref.get().expect("canvas should exist");
            let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();

            let window = web_sys::window().expect("should have window");
            canvas_element.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas_element.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            set_render_trigger.set(());
        }
    });

    // Re-render when viewport changes
    create_effect(move |_| {
        viewport.track(); // Track viewport signal
        if canvas_ref.get().is_some() {
            set_render_trigger.set(());
        }
    });

    // Handle window resize
    let handle_resize = move || {
        if let Some(canvas) = canvas_ref.get() {
            let canvas_element: HtmlCanvasElement = (*canvas).clone().unchecked_into();
            let window = web_sys::window().expect("should have window");

            canvas_element.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas_element.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            set_render_trigger.set(());
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
