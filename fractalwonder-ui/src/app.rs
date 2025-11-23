// fractalwonder-ui/src/app.rs
use fractalwonder_core::{calculate_precision_bits, fit_viewport_to_canvas, Viewport};
use leptos::*;

use crate::components::{InteractiveCanvas, UIPanel};
use crate::config::default_config;

#[component]
pub fn App() -> impl IntoView {
    // Canvas size signal (updated by InteractiveCanvas on resize)
    let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

    // Current fractal configuration
    let (config, _set_config) = create_signal(default_config());

    // Viewport signal - now writable for interaction updates
    let (viewport, set_viewport) = create_signal(Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 64));

    // Initialize and adjust viewport when canvas size changes
    create_effect(move |prev_size: Option<(u32, u32)>| {
        let size = canvas_size.get();
        let cfg = config.get();

        // Skip if invalid size
        if size.0 == 0 || size.1 == 0 {
            return size;
        }

        let was_valid = prev_size.map(|(w, h)| w > 0 && h > 0).unwrap_or(false);

        if !was_valid {
            // First time we have a valid size: initialize viewport from config
            let natural = cfg.default_viewport(64);
            let fitted = fit_viewport_to_canvas(&natural, size);
            let required_bits = calculate_precision_bits(&fitted, size);

            let final_viewport = if required_bits > fitted.precision_bits() {
                let natural_high_prec = cfg.default_viewport(required_bits);
                fit_viewport_to_canvas(&natural_high_prec, size)
            } else {
                fitted
            };

            set_viewport.set(final_viewport);
        } else if prev_size != Some(size) {
            // Canvas resized: adjust viewport to maintain aspect ratio
            // Keep the same center and vertical extent, adjust horizontal to match new aspect
            let current_vp = viewport.get_untracked();
            let fitted = fit_viewport_to_canvas(&current_vp, size);
            set_viewport.set(fitted);
        }

        size
    });

    // Precision bits - derived from viewport and canvas
    let precision_bits = create_memo(move |_| {
        let vp = viewport.get();
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            64
        } else {
            calculate_precision_bits(&vp, size)
        }
    });

    let on_resize = Callback::new(move |size: (u32, u32)| {
        set_canvas_size.set(size);
    });

    let on_viewport_change = Callback::new(move |new_vp: Viewport| {
        set_viewport.set(new_vp);
    });

    view! {
        <InteractiveCanvas
            viewport=viewport.into()
            on_viewport_change=on_viewport_change
            on_resize=on_resize
        />
        <UIPanel
            canvas_size=canvas_size.into()
            viewport=viewport.into()
            config=config.into()
            precision_bits=precision_bits.into()
        />
    }
}
