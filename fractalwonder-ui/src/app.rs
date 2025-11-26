// fractalwonder-ui/src/app.rs
use fractalwonder_core::{calculate_precision_bits, fit_viewport_to_canvas, Viewport};
use leptos::*;

use crate::components::{CircularProgress, InteractiveCanvas, UIPanel};
use crate::config::{default_config, get_config, FRACTAL_CONFIGS};
use crate::hooks::use_ui_visibility;
use crate::rendering::RenderProgress;

#[component]
pub fn App() -> impl IntoView {
    // Canvas size signal (updated by InteractiveCanvas on resize)
    let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

    // Render progress signal (updated by renderer)
    let (render_progress, set_render_progress) =
        create_signal(RwSignal::new(RenderProgress::default()));

    // Selected renderer (fractal type)
    let (selected_config_id, set_selected_config_id) = create_signal("mandelbrot".to_string());

    // Derive config from selected ID
    let config =
        create_memo(move |_| get_config(&selected_config_id.get()).unwrap_or_else(default_config));

    // Build renderer options from FRACTAL_CONFIGS
    let renderer_options = Signal::derive(move || {
        FRACTAL_CONFIGS
            .iter()
            .map(|c| (c.id.to_string(), c.display_name.to_string()))
            .collect::<Vec<_>>()
    });

    // Colorizer options (single default for now)
    let colorizer_options =
        Signal::derive(move || vec![("default".to_string(), "Default".to_string())]);
    let selected_colorizer_id = Signal::derive(move || "default".to_string());

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

    // Reset viewport when config changes
    create_effect(move |prev_id: Option<String>| {
        let current_id = selected_config_id.get();
        if let Some(prev) = prev_id {
            if prev != current_id {
                // Config changed - reset to default viewport
                if let Some(cfg) = get_config(&current_id) {
                    let size = canvas_size.get();
                    if size.0 > 0 && size.1 > 0 {
                        let precision = calculate_precision_bits(&cfg.default_viewport(128), size);
                        let natural_vp = cfg.default_viewport(precision);
                        let fitted_vp = fit_viewport_to_canvas(&natural_vp, size);
                        set_viewport.set(fitted_vp);
                    }
                }
            }
        }
        current_id
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

    let on_home_click = Callback::new(move |_: ()| {
        let size = canvas_size.get_untracked();
        let cfg = config.get_untracked();

        // Skip if invalid canvas size
        if size.0 == 0 || size.1 == 0 {
            return;
        }

        // Reset viewport to config default, fitted to current canvas
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
    });

    let on_progress_signal = Callback::new(move |progress_signal: RwSignal<RenderProgress>| {
        set_render_progress.set(progress_signal);
    });

    // UI visibility (autohide behavior)
    let ui_visibility = use_ui_visibility();

    // Cancel trigger - incremented to request render cancellation
    let (cancel_trigger, set_cancel_trigger) = create_signal(0u32);

    let on_cancel = Callback::new(move |_: ()| {
        set_cancel_trigger.update(|v| *v = v.wrapping_add(1));
    });

    view! {
        <InteractiveCanvas
            viewport=viewport.into()
            on_viewport_change=on_viewport_change
            config=config.into()
            on_resize=on_resize
            on_progress_signal=on_progress_signal
            cancel_trigger=cancel_trigger
        />
        <UIPanel
            canvas_size=canvas_size.into()
            viewport=viewport.into()
            config=config.into()
            precision_bits=precision_bits.into()
            on_home_click=on_home_click
            renderer_options=renderer_options
            selected_renderer_id=Signal::derive(move || selected_config_id.get())
            on_renderer_select=Callback::new(move |id: String| set_selected_config_id.set(id))
            colorizer_options=colorizer_options
            selected_colorizer_id=selected_colorizer_id
            on_colorizer_select=Callback::new(move |_: String| {
                // No-op for now - colorizer selection in Iteration 8
            })
            render_progress=render_progress.into()
            is_visible=ui_visibility.is_visible
            set_is_hovering=ui_visibility.set_is_hovering
            on_cancel=on_cancel
        />
        <CircularProgress
            progress=render_progress.into()
            is_ui_visible=ui_visibility.is_visible
        />
    }
}
