// fractalwonder-ui/src/app.rs
use fractalwonder_core::{calculate_precision_bits, fit_viewport_to_canvas, Viewport};
use leptos::*;
use wasm_bindgen::prelude::Closure;

use crate::components::PaletteEditorState;
use crate::components::{CircularProgress, InteractiveCanvas, PaletteEditor, Toast, UIPanel};
use crate::config::{default_config, get_config};
use crate::hooks::{
    apply_palette_order, load_palette_order, load_state, save_palette_order, save_state,
    use_hashchange_listener, use_ui_visibility, PersistedState,
};
use crate::rendering::colorizers::Palette;
use crate::rendering::RenderProgress;

#[component]
pub fn App() -> impl IntoView {
    // Load persisted state from localStorage (if any)
    let persisted = load_state();

    // Extract persisted values before moving into closures
    let initial_config_id = persisted
        .as_ref()
        .map(|s| s.config_id.clone())
        .unwrap_or_else(|| "mandelbrot".to_string());
    let initial_palette_id = persisted
        .as_ref()
        .map(|s| s.palette_name.clone())
        .unwrap_or_else(|| "Classic".to_string());
    let initial_render_settings = persisted
        .as_ref()
        .map(|s| s.render_settings.clone())
        .unwrap_or_default();
    let persisted_viewport = persisted.map(|s| s.viewport);

    // Store persisted viewport for use in effect (consumed on first use)
    let stored_viewport = store_value(persisted_viewport);

    // Canvas size signal (updated by InteractiveCanvas on resize)
    let (canvas_size, set_canvas_size) = create_signal((0u32, 0u32));

    // Render progress signal (updated by renderer)
    let (render_progress, set_render_progress) =
        create_signal(RwSignal::new(RenderProgress::default()));

    // Selected renderer (fractal type) - use persisted value if available
    let (selected_config_id, _set_selected_config_id) = create_signal(initial_config_id);

    // Derive config from selected ID
    let config =
        create_memo(move |_| get_config(&selected_config_id.get()).unwrap_or_else(default_config));

    // Palette and render settings state
    let palette = create_rw_signal(Palette::default());
    let (render_settings, set_render_settings) = create_signal(initial_render_settings);
    let (palette_id, set_palette_id) = create_signal(initial_palette_id.clone());

    // Load initial palette asynchronously
    create_effect(move |_| {
        let id = palette_id.get();
        spawn_local(async move {
            Palette::factory_defaults().await; // ensure loaded
            if let Some(pal) = Palette::get(&id).await {
                palette.set(pal);
            }
        });
    });

    // Derive cycle_count for UI display
    let cycle_count = create_memo(move |_| render_settings.get().cycle_count);

    // Palette options for dropdown (load asynchronously)
    let (palette_list, set_palette_list) = create_signal(Vec::<Palette>::new());
    create_effect(move |_| {
        spawn_local(async move {
            let palettes = Palette::factory_defaults().await;
            set_palette_list.set(palettes);
        });
    });

    // Palette order (persisted to localStorage)
    let initial_order = load_palette_order().unwrap_or_default();
    let (palette_order, set_palette_order) = create_signal(initial_order);

    // Derive ordered palette options
    let palette_options = Signal::derive(move || {
        let available: Vec<(String, String)> = palette_list
            .get()
            .iter()
            .map(|p| (p.name.clone(), p.name.clone()))
            .collect();
        let order = palette_order.get();

        if order.is_empty() {
            available
        } else {
            apply_palette_order(&available, &order)
        }
    });

    // Handle palette reorder (from_id dropped onto to_id)
    let on_palette_reorder = Callback::new(move |(from_id, to_id): (String, String)| {
        let current_options = palette_options.get_untracked();
        let current_order: Vec<String> = current_options.iter().map(|(id, _)| id.clone()).collect();

        // Find positions
        let from_idx = current_order.iter().position(|id| id == &from_id);
        let to_idx = current_order.iter().position(|id| id == &to_id);

        if let (Some(from), Some(to)) = (from_idx, to_idx) {
            let mut new_order = current_order;
            let item = new_order.remove(from);
            // Insert before the target position
            new_order.insert(to, item);

            // Persist and update signal
            save_palette_order(&new_order);
            set_palette_order.set(new_order);
        }
    });

    // Palette editor state (None = closed)
    let editor_state = create_rw_signal(None::<PaletteEditorState>);

    // Factory palette names
    let factory_names = Signal::derive(move || {
        palette_list
            .get()
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
    });

    // All palette names (for now, just factory names)
    let all_palette_names = factory_names;

    // Render palette: use working_palette when editor is open, else active palette
    let render_palette = Signal::derive(move || {
        if let Some(state) = editor_state.get() {
            state.working_palette.clone()
        } else {
            palette.get()
        }
    });

    // Toast message signal
    let (toast_message, set_toast_message) = create_signal::<Option<String>>(None);

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
            // First time we have a valid size: check for persisted viewport
            let final_viewport = if let Some(persisted_vp) = stored_viewport.get_value() {
                // Clear stored viewport so it's only used once
                stored_viewport.set_value(None);
                // Fit persisted viewport to current canvas
                fit_viewport_to_canvas(&persisted_vp, size)
            } else {
                // No persisted state - initialize from config default
                let natural = cfg.default_viewport(64);
                let fitted = fit_viewport_to_canvas(&natural, size);
                let required_bits = calculate_precision_bits(&fitted, size);

                if required_bits > fitted.precision_bits() {
                    let natural_high_prec = cfg.default_viewport(required_bits);
                    fit_viewport_to_canvas(&natural_high_prec, size)
                } else {
                    fitted
                }
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

    // Persist state to localStorage when viewport, palette, or render settings change
    create_effect(move |_| {
        let vp = viewport.get();
        let config_id = selected_config_id.get();
        let pal_id = palette_id.get();
        let settings = render_settings.get();

        // Skip saving if viewport hasn't been initialized yet
        if vp.width.to_f64() == 4.0 && vp.height.to_f64() == 3.0 {
            return;
        }

        let state = PersistedState::new(vp, config_id, pal_id, settings);
        save_state(&state);
    });

    // Listen for hashchange events (e.g., when user clicks a bookmark)
    // This enables navigating to a saved position via URL hash
    use_hashchange_listener(move |state| {
        let size = canvas_size.get_untracked();
        if size.0 > 0 && size.1 > 0 {
            // Fit the persisted viewport to the current canvas size
            let fitted = fit_viewport_to_canvas(&state.viewport, size);
            set_viewport.set(fitted);
            set_palette_id.set(state.palette_name.clone());
            set_render_settings.set(state.render_settings.clone());
            log::info!("Restored viewport, palette, and render settings from URL hash change");
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

    // X-ray mode toggle for visualizing glitched regions
    let (xray_enabled, set_xray_enabled) = create_signal(false);

    // Trigger for quadtree subdivision (incremented by "d" key when x-ray enabled)
    let (subdivide_trigger, set_subdivide_trigger) = create_signal(0u32);

    // Global keyboard handler for shortcuts
    // Store handler in a StoredValue so it lives for the component lifetime
    // and can be properly cleaned up
    let keyboard_handler = store_value::<Option<Closure<dyn FnMut(web_sys::KeyboardEvent)>>>(None);

    create_effect(move |_| {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        let handler = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            // Skip if typing in an input field
            if let Some(target) = e.target() {
                if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                    let tag = element.tag_name().to_lowercase();
                    if tag == "input" || tag == "textarea" {
                        return;
                    }
                }
            }

            match e.key().as_str() {
                "x" | "X" => {
                    // Toggle x-ray mode
                    set_xray_enabled.update(|v| {
                        *v = !*v;
                        let msg = if *v { "X-ray: On" } else { "X-ray: Off" };
                        set_toast_message.set(Some(msg.to_string()));
                    });
                }
                "d" | "D" => {
                    // Subdivide quadtree (only when x-ray enabled)
                    if xray_enabled.get_untracked() {
                        set_subdivide_trigger.update(|v| *v = v.wrapping_add(1));
                    }
                }
                "ArrowLeft" => {
                    // Previous palette (using ordered list)
                    let ordered = palette_options.get_untracked();
                    let current_name = palette_id.get_untracked();
                    let current_idx = ordered
                        .iter()
                        .position(|(id, _)| id == &current_name)
                        .unwrap_or(0);
                    let new_idx = if current_idx == 0 {
                        ordered.len() - 1
                    } else {
                        current_idx - 1
                    };
                    if let Some((new_id, _)) = ordered.get(new_idx) {
                        set_palette_id.set(new_id.clone());
                        set_toast_message.set(Some(format!("Palette: {}", new_id)));
                    }
                }
                "ArrowRight" => {
                    // Next palette (using ordered list)
                    let ordered = palette_options.get_untracked();
                    let current_name = palette_id.get_untracked();
                    let current_idx = ordered
                        .iter()
                        .position(|(id, _)| id == &current_name)
                        .unwrap_or(0);
                    let new_idx = (current_idx + 1) % ordered.len();
                    if let Some((new_id, _)) = ordered.get(new_idx) {
                        set_palette_id.set(new_id.clone());
                        set_toast_message.set(Some(format!("Palette: {}", new_id)));
                    }
                }
                "ArrowUp" => {
                    // Increase cycle count (Shift = +50, otherwise +1)
                    let amount = if e.shift_key() { 50 } else { 1 };
                    set_render_settings.update(|settings| {
                        settings.cycle_up_by(amount);
                        set_toast_message.set(Some(format!("Cycles: {}", settings.cycle_count)));
                    });
                }
                "ArrowDown" => {
                    // Decrease cycle count (Shift = -50, otherwise -1)
                    let amount = if e.shift_key() { 50 } else { 1 };
                    set_render_settings.update(|settings| {
                        settings.cycle_down_by(amount);
                        set_toast_message.set(Some(format!("Cycles: {}", settings.cycle_count)));
                    });
                }
                "g" | "G" => {
                    // Toggle GPU rendering
                    set_render_settings.update(|settings| {
                        settings.use_gpu = !settings.use_gpu;
                        let msg = if settings.use_gpu {
                            "GPU: On"
                        } else {
                            "GPU: Off"
                        };
                        set_toast_message.set(Some(msg.to_string()));
                    });
                }
                _ => {}
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref());
        }

        keyboard_handler.set_value(Some(handler));

        on_cleanup(move || {
            keyboard_handler.with_value(|handler_opt| {
                if let Some(handler) = handler_opt {
                    if let Some(window) = web_sys::window() {
                        let _ = window.remove_event_listener_with_callback(
                            "keydown",
                            handler.as_ref().unchecked_ref(),
                        );
                    }
                }
            });
            keyboard_handler.set_value(None);
        });
    });

    let on_palette_edit = Callback::new(move |name: String| {
        let palette_val = palette.get_untracked();
        let factory = factory_names.get_untracked();

        // If editing a factory palette that hasn't been shadowed, it's a duplicate
        let is_factory = factory.contains(&name);
        let state = if is_factory && Palette::load(&name).is_none() {
            // Factory palette, not shadowed - treat as duplicate (but keep name for shadowing)
            PaletteEditorState::duplicate(palette_val, name)
        } else {
            // Custom palette or shadowed factory - edit mode
            PaletteEditorState::edit(palette_val)
        };
        editor_state.set(Some(state));
    });

    view! {
        <InteractiveCanvas
            viewport=viewport.into()
            on_viewport_change=on_viewport_change
            config=config.into()
            on_resize=on_resize
            on_progress_signal=on_progress_signal
            cancel_trigger=cancel_trigger
            subdivide_trigger=subdivide_trigger
            xray_enabled=xray_enabled
            palette=render_palette
            render_settings=render_settings.into()
        />
        <UIPanel
            viewport=viewport.into()
            config=config.into()
            precision_bits=precision_bits.into()
            on_home_click=on_home_click
            palette_options=palette_options
            selected_palette_id=Signal::derive(move || palette_id.get())
            on_palette_select=Callback::new(move |name: String| {
                set_palette_id.set(name.clone());
                set_toast_message.set(Some(format!("Palette: {}", name)));
            })
            cycle_count=Signal::derive(move || cycle_count.get())
            on_cycle_up=Callback::new(move |_| {
                set_render_settings.update(|settings| {
                    settings.cycle_up();
                    set_toast_message.set(Some(format!("Cycles: {}", settings.cycle_count)));
                });
            })
            on_cycle_down=Callback::new(move |_| {
                set_render_settings.update(|settings| {
                    settings.cycle_down();
                    set_toast_message.set(Some(format!("Cycles: {}", settings.cycle_count)));
                });
            })
            use_gpu=Signal::derive(move || render_settings.get().use_gpu)
            on_gpu_toggle=Callback::new(move |_| {
                set_render_settings.update(|settings| {
                    settings.use_gpu = !settings.use_gpu;
                    let msg = if settings.use_gpu { "GPU: On" } else { "GPU: Off" };
                    set_toast_message.set(Some(msg.to_string()));
                });
            })
            render_progress=render_progress.into()
            is_visible=ui_visibility.is_visible
            set_is_hovering=ui_visibility.set_is_hovering
            on_cancel=on_cancel
            xray_enabled=xray_enabled
            set_xray_enabled=set_xray_enabled
            on_edit=on_palette_edit
            on_palette_reorder=on_palette_reorder
        />
        <PaletteEditor
            state=editor_state
            active_palette=palette
            all_palette_names=all_palette_names
            factory_names=factory_names
        />
        <Toast
            message=Signal::derive(move || toast_message.get())
            ui_visible=ui_visibility.is_visible.into()
        />
        <CircularProgress
            progress=render_progress.into()
            is_ui_visible=ui_visibility.is_visible
        />
    }
}
