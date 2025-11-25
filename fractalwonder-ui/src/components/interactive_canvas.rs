// fractalwonder-ui/src/components/interactive_canvas.rs
use fractalwonder_compute::{MandelbrotRenderer, Renderer, TestImageRenderer};
use fractalwonder_core::{apply_pixel_transform_to_viewport, calculate_max_iterations, Viewport};
use leptos::*;
use leptos_use::use_window_size;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::config::FractalConfig;
use crate::hooks::use_canvas_interaction;
use crate::rendering::{colorize_mandelbrot, colorize_test_image};

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

    // Wire up interaction hook
    let _interaction = use_canvas_interaction(
        canvas_ref,
        move || {
            // Cancel render on interaction start (placeholder for now)
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Interaction started - would cancel render"));
        },
        move |transform| {
            let current_vp = viewport.get_untracked();
            let size = canvas_size.get_untracked();

            if size.0 > 0 && size.1 > 0 {
                // Precision is calculated internally by apply_pixel_transform_to_viewport
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

    // Render effect - redraws when viewport or config changes
    create_effect(move |_| {
        let vp = viewport.get();
        let cfg = config.get();
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            return;
        }

        let Some(canvas_el) = canvas_ref.get() else {
            return;
        };
        let canvas = canvas_el.unchecked_ref::<HtmlCanvasElement>();

        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .unchecked_into::<CanvasRenderingContext2d>();

        let (width, height) = size;

        // Dispatch to renderer based on config.id
        let mut data = vec![0u8; (width * height * 4) as usize];

        match cfg.id {
            "test_image" => {
                let renderer = TestImageRenderer;
                let computed = renderer.render(&vp, size);
                for (i, pixel_data) in computed.iter().enumerate() {
                    let color = colorize_test_image(pixel_data);
                    let idx = i * 4;
                    data[idx] = color[0];
                    data[idx + 1] = color[1];
                    data[idx + 2] = color[2];
                    data[idx + 3] = color[3];
                }
            }
            "mandelbrot" => {
                let reference_width = cfg.default_viewport(vp.precision_bits()).width;
                let max_iters = calculate_max_iterations(&vp.width, &reference_width);
                let renderer = MandelbrotRenderer::new(max_iters);
                let computed = renderer.render(&vp, size);
                for (i, pixel_data) in computed.iter().enumerate() {
                    let color = colorize_mandelbrot(pixel_data);
                    let idx = i * 4;
                    data[idx] = color[0];
                    data[idx + 1] = color[1];
                    data[idx + 2] = color[2];
                    data[idx + 3] = color[3];
                }
            }
            _ => {
                // Unknown renderer - fill with magenta for visibility
                for i in 0..(width * height) as usize {
                    let idx = i * 4;
                    data[idx] = 255;
                    data[idx + 1] = 0;
                    data[idx + 2] = 255;
                    data[idx + 3] = 255;
                }
            }
        }

        // Draw to canvas
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&data), width, height)
            .expect("should create ImageData");
        ctx.put_image_data(&image_data, 0.0, 0.0)
            .expect("should put image data");
    });

    view! {
        <canvas node_ref=canvas_ref class="block" />
    }
}
