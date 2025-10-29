use crate::hooks::use_canvas_interaction::use_canvas_interaction;
use crate::rendering::{points::Rect, viewport::Viewport, Renderer, TilingCanvasRenderer};
use leptos::*;
use wasm_bindgen::JsCast;

#[component]
pub fn InteractiveCanvas<R>(
    canvas_renderer: RwSignal<TilingCanvasRenderer<R>>,
    viewport: ReadSignal<Viewport<R::Coord>>,
    set_viewport: WriteSignal<Viewport<R::Coord>>,
    set_render_time_ms: WriteSignal<Option<f64>>,
    natural_bounds: Rect<R::Coord>,
) -> impl IntoView
where
    R: Renderer + Clone + 'static,
    R::Coord: Clone + PartialEq + 'static,
    R::Coord: std::ops::Sub<Output = R::Coord>
        + std::ops::Add<Output = R::Coord>
        + std::ops::Mul<f64, Output = R::Coord>
        + std::ops::Div<f64, Output = R::Coord>
        + From<f64>,
{
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

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

    // Canvas interaction hook - callback updates viewport
    let interaction = use_canvas_interaction(canvas_ref, move |transform_result| {
        if let Some(canvas_el) = canvas_ref.get() {
            let canvas = canvas_el.unchecked_ref::<web_sys::HtmlCanvasElement>();
            let width = canvas.width();
            let height = canvas.height();

            set_viewport.update(|vp| {
                *vp = crate::rendering::apply_pixel_transform_to_viewport(
                    vp,
                    &natural_bounds,
                    &transform_result,
                    width,
                    height,
                );
            });
        }
    });

    view! {
        <canvas
            node_ref=canvas_ref
            class="w-full h-full"
            width="800"
            height="600"
            on:wheel=move |e| (interaction.on_wheel)(e)
            on:pointerdown=move |e| (interaction.on_pointer_down)(e)
            on:pointermove=move |e| (interaction.on_pointer_move)(e)
            on:pointerup=move |e| (interaction.on_pointer_up)(e)
        />
    }
}
