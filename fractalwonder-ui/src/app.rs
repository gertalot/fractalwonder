use crate::components::interactive_canvas::InteractiveCanvas;
use crate::components::ui::UI;
use crate::components::CircularProgress;
use crate::hooks::fullscreen::toggle_fullscreen;
use crate::hooks::ui_visibility::use_ui_visibility;
use crate::rendering::canvas_renderer::CanvasRenderer;
use crate::rendering::{
    get_config, AppData, Colorizer, ParallelCanvasRenderer, Viewport, RENDER_CONFIGS,
};
use crate::state::AppState;
use leptos::*;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen::JsValue;

fn create_canvas_renderer(
    colorizer: Colorizer<AppData>,
    renderer_id: String,
) -> Result<Rc<dyn CanvasRenderer<Scalar = f64, Data = AppData>>, JsValue> {
    Ok(Rc::new(ParallelCanvasRenderer::new(
        colorizer,
        renderer_id,
    )?))
}

#[component]
pub fn App() -> impl IntoView {
    // ========== Load state from localStorage ==========
    let initial_state = AppState::load();

    let (selected_renderer_id, set_selected_renderer_id) =
        create_signal(initial_state.selected_renderer_id.clone());

    // Get initial config and state before moving renderer_states
    let initial_config = get_config(&initial_state.selected_renderer_id).unwrap();
    let initial_renderer_state = initial_state
        .renderer_states
        .get(&initial_state.selected_renderer_id)
        .unwrap()
        .clone();

    let (renderer_states, set_renderer_states) = create_signal(initial_state.renderer_states);

    // ========== Create initial renderer ==========
    let initial_colorizer = crate::rendering::get_colorizer(
        &initial_state.selected_renderer_id,
        &initial_renderer_state.color_scheme_id,
    )
    .expect("Initial renderer/color scheme combination must be valid");

    let initial_canvas_renderer = create_canvas_renderer(
        initial_colorizer,
        initial_state.selected_renderer_id.clone(),
    )
    .expect("Failed to create canvas renderer");

    // ========== Viewport - derived from current renderer's state ==========
    // This ensures each renderer has its own isolated viewport
    let viewport = Signal::derive(move || {
        let renderer_id = selected_renderer_id.get();
        let states = renderer_states.get();
        states
            .get(&renderer_id)
            .map(|s| s.viewport.clone())
            .unwrap_or_else(|| Viewport::new(fractalwonder_core::Point::new(0.0, 0.0), 1.0))
    });

    let set_viewport = move |new_viewport: Viewport<f64>| {
        let renderer_id = selected_renderer_id.get_untracked();
        set_renderer_states.update(|states| {
            if let Some(state) = states.get_mut(&renderer_id) {
                state.viewport = new_viewport;
            }
        });
    };

    // ========== Canvas renderer with cache ==========
    let canvas_renderer: RwSignal<Rc<dyn CanvasRenderer<Scalar = f64, Data = AppData>>> =
        create_rw_signal(initial_canvas_renderer);

    // ========== Natural bounds - reactive to renderer changes ==========
    let natural_bounds = create_memo(move |_| canvas_renderer.with(|cr| cr.natural_bounds()));

    // ========== Progress tracking ==========
    let progress = Signal::derive(move || canvas_renderer.with(|cr| cr.progress().get()));

    // ========== RendererInfo for UI display ==========
    let initial_info = (initial_config.create_info_provider)().info(&viewport.get_untracked());
    let (render_time_ms, set_render_time_ms) = create_signal(None::<f64>);
    let (renderer_info, set_renderer_info) = create_signal(initial_info);

    // ========== Effect: Update renderer info when viewport or render time changes ==========
    create_effect(move |_| {
        let vp = viewport.get();
        let renderer_id = selected_renderer_id.get();
        let config = get_config(&renderer_id).unwrap();
        let info_provider = (config.create_info_provider)();
        let mut info = info_provider.info(&vp);
        info.render_time_ms = render_time_ms.get();
        set_renderer_info.set(info);
    });

    // ========== Effect: Renderer selection changed ==========
    let previous_renderer_id = create_rw_signal(initial_state.selected_renderer_id.clone());

    create_effect(move |_| {
        let new_renderer_id = selected_renderer_id.get();
        let old_renderer_id = previous_renderer_id.get_untracked();

        // Only create new renderer if renderer_id actually changed
        if new_renderer_id != old_renderer_id {
            previous_renderer_id.set(new_renderer_id.clone());

            // CRITICAL: Use get_untracked() to avoid re-running when color_scheme_id changes
            let states = renderer_states.get_untracked();
            let state = states.get(&new_renderer_id).unwrap();

            // Find colorizer for restored color scheme
            let colorizer =
                crate::rendering::get_colorizer(&new_renderer_id, &state.color_scheme_id)
                    .expect("Renderer/color scheme combination must be valid");

            // Create new canvas renderer
            let new_canvas_renderer = create_canvas_renderer(colorizer, new_renderer_id.clone())
                .expect("Failed to create canvas renderer");

            // Swap renderer
            canvas_renderer.set(new_canvas_renderer);

            // Save immediately
            let states = renderer_states.get_untracked();
            AppState {
                selected_renderer_id: new_renderer_id.clone(),
                renderer_states: states,
            }
            .save();
        }
    });

    // ========== Effect: Viewport changed (save debounced) ==========
    let (viewport_save_trigger, set_viewport_save_trigger) = create_signal(());

    create_effect(move |_| {
        viewport.get();
        set_timeout(
            move || {
                set_viewport_save_trigger.update(|_| {});
            },
            Duration::from_millis(500),
        );
    });

    create_effect(move |_| {
        viewport_save_trigger.get();
        let renderer_id = selected_renderer_id.get();
        let states = renderer_states.get();

        AppState {
            selected_renderer_id: renderer_id,
            renderer_states: states,
        }
        .save();
    });

    // ========== Derived signal: Selected color scheme ID ==========
    let selected_color_scheme_id = create_memo(move |_| {
        let renderer_id = selected_renderer_id.get();
        let states = renderer_states.get();
        states
            .get(&renderer_id)
            .map(|s| s.color_scheme_id.clone())
            .unwrap_or_default()
    });

    // ========== Effect: Color scheme changed ==========
    let on_color_scheme_select = move |scheme_id: String| {
        let renderer_id = selected_renderer_id.get();
        let colorizer = crate::rendering::get_colorizer(&renderer_id, &scheme_id)
            .expect("Renderer/color scheme combination must be valid");

        canvas_renderer.update(|cr| {
            cr.set_colorizer(colorizer);
        });

        set_renderer_states.update(|states| {
            if let Some(state) = states.get_mut(&renderer_id) {
                state.color_scheme_id = scheme_id.clone();
            }
        });

        let states = renderer_states.get_untracked();
        AppState {
            selected_renderer_id: renderer_id,
            renderer_states: states,
        }
        .save();
    };

    // ========== UI menu options ==========
    let render_function_options = create_memo(move |_| {
        RENDER_CONFIGS
            .iter()
            .map(|c| (c.id.to_string(), c.display_name.to_string()))
            .collect::<Vec<_>>()
    });

    let color_scheme_options = create_memo(move |_| {
        let renderer_id = selected_renderer_id.get();
        let config = get_config(&renderer_id).unwrap();
        config
            .color_schemes
            .iter()
            .map(|cs| (cs.id.to_string(), cs.display_name.to_string()))
            .collect::<Vec<_>>()
    });

    // ========== UI visibility and callbacks ==========
    let ui_visibility = use_ui_visibility();

    let on_home_click = move || {
        let bounds = natural_bounds.get();
        set_viewport(Viewport::new(bounds.center(), 1.0));
    };

    let on_fullscreen_click = move || {
        toggle_fullscreen();
    };

    view! {
        <div class="relative w-screen h-screen overflow-hidden bg-black">
            <InteractiveCanvas
                canvas_renderer=canvas_renderer
                viewport=viewport
                set_viewport=set_viewport
                set_render_time_ms=set_render_time_ms
                natural_bounds=natural_bounds.into()
            />
            <UI
                info=renderer_info
                is_visible=ui_visibility.is_visible
                set_is_hovering=ui_visibility.set_is_hovering
                on_home_click=on_home_click
                on_fullscreen_click=on_fullscreen_click
                render_function_options=render_function_options.into()
                selected_renderer_id=Signal::derive(move || selected_renderer_id.get())
                on_renderer_select=move |id: String| set_selected_renderer_id.set(id)
                color_scheme_options=color_scheme_options.into()
                selected_color_scheme_id=Signal::derive(move || selected_color_scheme_id.get())
                on_color_scheme_select=on_color_scheme_select
                progress=progress
            />
            <CircularProgress
                progress=progress
                is_ui_visible=ui_visibility.is_visible
            />
        </div>
    }
}
