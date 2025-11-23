// fractalwonder-ui/src/components/interactive_canvas.rs
use fractalwonder_core::{apply_pixel_transform_to_viewport, pixel_to_fractal, BigFloat, Viewport};
use leptos::*;
use leptos_use::use_window_size;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use crate::hooks::use_canvas_interaction;
use crate::rendering::test_pattern_color_normalized;

#[component]
pub fn InteractiveCanvas(
    /// Current viewport in fractal space (read-only)
    viewport: Signal<Viewport>,
    /// Callback fired when user interaction ends with a new viewport
    on_viewport_change: Callback<Viewport>,
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
    let _interaction = use_canvas_interaction(canvas_ref, move |transform| {
        let current_vp = viewport.get_untracked();
        let size = canvas_size.get_untracked();

        if size.0 > 0 && size.1 > 0 {
            let precision = current_vp.precision_bits();
            let new_vp =
                apply_pixel_transform_to_viewport(&current_vp, &transform, size, precision);

            on_viewport_change.call(new_vp);
        }
    });

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

    // Render effect - redraws when viewport changes
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

        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .unchecked_into::<CanvasRenderingContext2d>();

        let (width, height) = size;
        let precision = vp.precision_bits();

        // Pre-compute origin offset in normalized viewport coordinates.
        // CRITICAL: Normalize both x and y by the SAME dimension (height) to preserve
        // aspect ratio. This ensures equal normalized distances represent equal fractal
        // distances, so grid cells appear square on screen.
        let zero = BigFloat::zero(precision);
        let origin_norm_x = zero.sub(&vp.center.0).div(&vp.height).to_f64();
        let origin_norm_y = zero.sub(&vp.center.1).div(&vp.height).to_f64();

        // Create pixel buffer
        let mut data = vec![0u8; (width * height * 4) as usize];

        for py in 0..height {
            for px in 0..width {
                // CRITICAL: Use pixel_to_fractal to test the BigFloat coordinate pipeline!
                // This is the same transformation the Mandelbrot renderer will use.
                let (fx, fy) = pixel_to_fractal(px as f64, py as f64, &vp, size, precision);

                // Convert to viewport-center-relative normalized coordinates.
                // CRITICAL: Subtract center first, then normalize by HEIGHT.
                // This gives coordinates in [-0.5, 0.5] range where 0 = viewport center.
                let norm_x = fx.sub(&vp.center.0).div(&vp.height).to_f64();
                let norm_y = fy.sub(&vp.center.1).div(&vp.height).to_f64();

                // Compute color using normalized coordinates
                let color =
                    test_pattern_color_normalized(norm_x, norm_y, origin_norm_x, origin_norm_y);

                let idx = ((py * width + px) * 4) as usize;
                data[idx] = color[0];
                data[idx + 1] = color[1];
                data[idx + 2] = color[2];
                data[idx + 3] = color[3];
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
