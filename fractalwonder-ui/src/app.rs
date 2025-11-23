// fractalwonder-ui/src/app.rs
use fractalwonder_core::{calculate_precision_bits, fit_viewport_to_canvas};
use leptos::*;

use crate::components::{InteractiveCanvas, UIPanel};
use crate::config::default_config;

#[component]
pub fn App() -> impl IntoView {
    // Canvas size signal (updated by InteractiveCanvas on resize)
    let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

    // Current fractal configuration
    let (config, _set_config) = create_signal(default_config());

    // Viewport signal - computed from config and canvas size
    let viewport = create_memo(move |_| {
        let cfg = config.get();
        let size = canvas_size.get();

        // Handle zero-sized canvas (not yet measured)
        if size.0 == 0 || size.1 == 0 {
            return cfg.default_viewport(64);
        }

        // Create natural viewport at initial precision (64 bits = f64 equivalent, sufficient up to ~10^14 zoom)
        let natural = cfg.default_viewport(64);

        // Fit to canvas aspect ratio
        let fitted = fit_viewport_to_canvas(&natural, size);

        // Calculate required precision
        let required_bits = calculate_precision_bits(&fitted, size);

        // If we need more precision, recreate with correct precision
        if required_bits > fitted.precision_bits() {
            let natural_high_prec = cfg.default_viewport(required_bits);
            fit_viewport_to_canvas(&natural_high_prec, size)
        } else {
            fitted
        }
    });

    // Precision bits - derived from viewport and canvas
    let precision_bits = create_memo(move |_| {
        let vp = viewport.get();
        let size = canvas_size.get();

        if size.0 == 0 || size.1 == 0 {
            64 // Default (f64 equivalent)
        } else {
            calculate_precision_bits(&vp, size)
        }
    });

    let on_resize = Callback::new(move |size: (u32, u32)| {
        set_canvas_size.set(size);
    });

    view! {
        <InteractiveCanvas on_resize=on_resize />
        <UIPanel
            canvas_size=canvas_size.into()
            viewport=viewport.into()
            config=config.into()
            precision_bits=precision_bits.into()
        />
    }
}
