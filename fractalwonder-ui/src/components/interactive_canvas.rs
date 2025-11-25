// fractalwonder-ui/src/components/interactive_canvas.rs
use crate::config::FractalConfig;
use crate::hooks::use_canvas_interaction;
use crate::rendering::AsyncProgressiveRenderer;
use fractalwonder_core::{apply_pixel_transform_to_viewport, Viewport};
use leptos::*;
use leptos_use::use_window_size;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[component]
pub fn InteractiveCanvas(
    /// Current viewport in fractal space (read-only)
    viewport: Signal<Viewport>,
    /// Callback fired when user interaction ends with a new viewport
    on_viewport_change: Callback<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Callback fired when canvas dimensions change
    #[prop(optional)]
    on_resize: Option<Callback<(u32, u32)>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Reactive window size - automatically updates on resize
    let window_size = use_window_size();

    // Store canvas size for use in callbacks
    let canvas_size = create_rw_signal((0u32, 0u32));

    // Create renderer - store value and track config changes separately
    let renderer = store_value(AsyncProgressiveRenderer::new(config.get_untracked()));

    // Recreate renderer when config changes
    create_effect(move |_| {
        let cfg = config.get();
        renderer.set_value(AsyncProgressiveRenderer::new(cfg));
    });

    // Wire up interaction hook with cancel on start
    let _interaction = use_canvas_interaction(
        canvas_ref,
        move || {
            renderer.with_value(|r| r.cancel());
        },
        move |transform| {
            let current_vp = viewport.get_untracked();
            let size = canvas_size.get_untracked();

            if size.0 > 0 && size.1 > 0 {
                let new_vp = apply_pixel_transform_to_viewport(&current_vp, &transform, size);
                on_viewport_change.call(new_vp);
            }
        },
    );

    // Effect to handle resize
    create_effect(move |_| {
        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        let width = window_size.width.get() as u32;
        let height = window_size.height.get() as u32;

        if width == 0 || height == 0 {
            return;
        }

        // Update canvas dimensions
        canvas.set_width(width);
        canvas.set_height(height);

        // Store for interaction callback
        canvas_size.set((width, height));

        // Notify parent of dimensions
        if let Some(callback) = on_resize {
            callback.call((width, height));
        }
    });

    // Render effect - triggers async render on viewport change
    create_effect(move |_| {
        let vp = viewport.get();
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            return;
        }

        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        // Start async render (previous render is auto-cancelled)
        renderer.with_value(|r| r.render(&vp, canvas));
    });

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}
