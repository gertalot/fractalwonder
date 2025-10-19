use crate::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};
use crate::rendering::{
    apply_pixel_transform_to_viewport, points::Point, render_with_viewport,
    renderer_info::{RendererInfo, RendererInfoData},
    renderer_trait::Renderer, viewport::Viewport,
};
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{window, AddEventListenerOptions};

/// Return value from InteractiveCanvas containing the view and control signals
pub struct CanvasWithInfo {
    pub view: View,
    pub info: ReadSignal<RendererInfoData>,
    pub reset_viewport: Box<dyn Fn()>,
}

/// Generic interactive canvas component with pan/zoom support
///
/// Manages canvas lifecycle, viewport state, and interaction handling.
/// Works with any Renderer implementation.
///
/// # Type Parameters
/// * `T` - Coordinate type for image space (f64, rug::Float, etc.)
/// * `R` - Renderer implementation
///
/// # Example
/// ```rust,ignore
/// let renderer = PixelRenderer::new(MyCompute::new());
/// view! { <InteractiveCanvas renderer=renderer /> }
/// ```
#[component]
pub fn InteractiveCanvas<T, R>(renderer: R) -> CanvasWithInfo
where
    T: Clone
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Div<Output = T>
        + std::ops::Div<f64, Output = T>
        + std::ops::Mul<f64, Output = T>
        + From<f64>
        + 'static,
    R: Renderer<Coord = T> + RendererInfo<Coord = T> + Clone + 'static,
{
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Initialize viewport from renderer's natural bounds
    let natural_bounds = renderer.natural_bounds();
    let center = Point::new(
        // Calculate center from bounds
        // For f64: (max + min) / 2, but we need generic approach
        // Use existing Viewport logic or make this configurable
        // For now, assume Point implements a center calculation
        natural_bounds.center().x().clone(),
        natural_bounds.center().y().clone(),
    );
    let viewport = create_rw_signal(Viewport::new(center, 1.0, natural_bounds));

    // Create info signal for UI display
    let info = create_rw_signal(renderer.info(&viewport.get()));

    // Reset viewport callback for Home button
    let renderer_for_reset = renderer.clone();
    let reset_viewport = move || {
        let bounds = renderer_for_reset.natural_bounds();
        viewport.set(Viewport::new(
            Point::new(bounds.center().x().clone(), bounds.center().y().clone()),
            1.0,
            bounds,
        ));
    };

    // Set up interaction hook with viewport update
    let handle = use_canvas_interaction(canvas_ref, move |result: TransformResult| {
        if let Some(canvas) = canvas_ref.get_untracked() {
            let width = canvas.width();
            let height = canvas.height();

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport.get_untracked(),
                &result,
                width,
                height,
            );

            viewport.set(new_viewport);
        }
    });

    // Initialize canvas on mount
    let renderer_for_init = renderer.clone();
    create_effect(move |_| {
        if let Some(canvas) = canvas_ref.get() {
            let window = web_sys::window().expect("should have window");
            canvas.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
            canvas.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

            // Initial render (without timing - happens once)
            render_with_viewport(&canvas, &renderer_for_init, &viewport.get());
        }
    });

    // Re-render whenever viewport changes (with performance timing)
    let renderer_for_updates = renderer.clone();
    create_effect(move |_| {
        let current_viewport = viewport.get();
        if let Some(canvas) = canvas_ref.get_untracked() {
            // Time the render
            let start = window()
                .and_then(|w| w.performance())
                .and_then(|p| Some(p.now()));

            render_with_viewport(&canvas, &renderer_for_updates, &current_viewport);

            let end = window()
                .and_then(|w| w.performance())
                .and_then(|p| Some(p.now()));

            // Update info with performance metrics
            if let (Some(start_time), Some(end_time)) = (start, end) {
                let mut info_data = renderer_for_updates.info(&current_viewport);
                info_data.render_time_ms = Some(end_time - start_time);
                info.set(info_data);
            }
        }
    });

    // Handle window resize
    create_effect({
        let on_canvas_resize = handle.on_canvas_resize.clone();
        let viewport_clone = viewport;
        move |_| {
            if let Some(canvas) = canvas_ref.get() {
                use wasm_bindgen::closure::Closure;

                let canvas_clone = canvas.clone();
                let on_canvas_resize = on_canvas_resize.clone();
                let viewport_for_resize = viewport_clone;

                let resize_handler = Closure::wrap(Box::new(move || {
                    let window = web_sys::window().expect("should have window");
                    let new_width = window.inner_width().unwrap().as_f64().unwrap() as u32;
                    let new_height = window.inner_height().unwrap().as_f64().unwrap() as u32;

                    let old_width = canvas_clone.width();
                    let old_height = canvas_clone.height();

                    if old_width != new_width || old_height != new_height {
                        (on_canvas_resize)(new_width, new_height);
                        canvas_clone.set_width(new_width);
                        canvas_clone.set_height(new_height);
                        viewport_for_resize.update(|v| {
                            *v = v.clone();
                        });
                    }
                }) as Box<dyn Fn() + 'static>);

                web_sys::window()
                    .expect("should have window")
                    .add_event_listener_with_callback(
                        "resize",
                        resize_handler.as_ref().unchecked_ref(),
                    )
                    .expect("should add resize listener");

                resize_handler.forget();
            }
        }
    });

    // Manually attach wheel event listener with passive: false
    create_effect({
        move |_| {
            if let Some(canvas) = canvas_ref.get() {
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
        </div>
    }
}
