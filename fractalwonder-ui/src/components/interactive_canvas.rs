// fractalwonder-ui/src/components/interactive_canvas.rs
use crate::config::FractalConfig;
use crate::hooks::use_canvas_interaction;
use crate::rendering::ParallelRenderer;
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
    /// Callback fired with progress signal when renderer is created
    #[prop(optional)]
    on_progress_signal: Option<Callback<RwSignal<crate::rendering::RenderProgress>>>,
    /// Signal that triggers render cancellation when incremented
    #[prop(optional)]
    cancel_trigger: Option<ReadSignal<u32>>,
    /// Signal that triggers quadtree subdivision when incremented
    #[prop(optional)]
    subdivide_trigger: Option<ReadSignal<u32>>,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Reactive window size - automatically updates on resize
    let window_size = use_window_size();

    // Store canvas size for use in callbacks
    let canvas_size = create_rw_signal((0u32, 0u32));

    // Create renderer - handle Result since ParallelRenderer::new can fail
    let renderer = match ParallelRenderer::new(config.get_untracked()) {
        Ok(r) => store_value(r),
        Err(e) => {
            web_sys::console::error_1(&e);
            return view! { <div class="text-red-500">"Failed to initialize renderer"</div> }
                .into_view();
        }
    };

    // Notify parent of progress signal
    if let Some(callback) = on_progress_signal {
        renderer.with_value(|r| callback.call(r.progress()));
    }

    // Switch renderer when config changes
    create_effect(move |_| {
        let cfg = config.get();
        renderer.update_value(|r| {
            if let Err(e) = r.switch_config(cfg) {
                web_sys::console::error_1(&e);
            }
        });

        // Notify parent of new progress signal
        if let Some(callback) = on_progress_signal {
            renderer.with_value(|r| callback.call(r.progress()));
        }
    });

    // Watch for external cancel requests
    if let Some(trigger) = cancel_trigger {
        create_effect(move |prev: Option<u32>| {
            let current = trigger.get();
            // Only cancel if value changed (not on initial mount)
            if prev.is_some() && prev != Some(current) {
                renderer.with_value(|r| r.cancel());
            }
            current
        });
    }

    // Watch for subdivision requests
    if let Some(trigger) = subdivide_trigger {
        create_effect(move |prev: Option<u32>| {
            let current = trigger.get();
            // Only subdivide if value changed (not on initial mount)
            if prev.is_some() && prev != Some(current) {
                renderer.with_value(|r| r.subdivide_glitched_cells());
            }
            current
        });
    }

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
    .into_view()
}
