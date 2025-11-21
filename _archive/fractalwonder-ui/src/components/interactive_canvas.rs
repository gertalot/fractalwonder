use crate::hooks::use_canvas_interaction::use_canvas_interaction;
use crate::rendering::{AppData, CanvasRenderer};
use fractalwonder_core::{Rect, Viewport};
use leptos::*;
use wasm_bindgen::JsCast;

#[component]
pub fn InteractiveCanvas<CR: 'static + CanvasRenderer<Scalar = f64, Data = AppData> + Clone>(
    canvas_renderer: RwSignal<CR>,
    viewport: Signal<Viewport<f64>>,
    set_viewport: impl Fn(Viewport<f64>) + 'static + Copy,
    set_render_time_ms: WriteSignal<Option<f64>>,
    natural_bounds: Signal<Rect<f64>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Canvas interaction hook - callback updates viewport
    // This is what makes this an *interactive* canvas.
    let interaction = use_canvas_interaction(canvas_ref, move |transform_result| {
        if let Some(canvas_el) = canvas_ref.get_untracked() {
            let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();
            let width = canvas.width();
            let height = canvas.height();

            let current_vp = viewport.get_untracked();
            let new_vp = crate::rendering::apply_pixel_transform_to_viewport(
                &current_vp,
                &natural_bounds.get_untracked(),
                &transform_result,
                width,
                height,
            );
            set_viewport(new_vp);
        }
    });

    // Cancel any in-progress render when user starts interacting
    create_effect(move |_| {
        if interaction.is_interacting.get() {
            canvas_renderer.with(|cr| cr.cancel_render());
        }
    });

    // Initialize canvas dimensions on mount
    create_effect(move |_| {
        if let Some(canvas_el) = canvas_ref.get() {
            let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();
            let window = web_sys::window().expect("should have window");
            let new_width = window.inner_width().unwrap().as_f64().unwrap() as u32;
            let new_height = window.inner_height().unwrap().as_f64().unwrap() as u32;
            canvas.set_width(new_width);
            canvas.set_height(new_height);
        }
    });

    // Effect: Render when canvas_renderer OR viewport changes
    create_effect(move |_| {
        let vp = viewport.get();
        canvas_renderer.track();

        if let Some(canvas_el) = canvas_ref.get() {
            let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();

            let start = web_sys::window().unwrap().performance().unwrap().now();

            canvas_renderer.with(|cr| cr.render(&vp, canvas));

            let elapsed = web_sys::window().unwrap().performance().unwrap().now() - start;
            set_render_time_ms.set(Some(elapsed));
        }
    });

    view! {
        <canvas node_ref=canvas_ref class="w-full h-full" />
    }
}
