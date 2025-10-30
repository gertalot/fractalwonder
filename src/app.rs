use crate::components::interactive_canvas::InteractiveCanvas;
use crate::components::ui::UI;
use crate::hooks::fullscreen::toggle_fullscreen;
use crate::hooks::ui_visibility::use_ui_visibility;
use crate::rendering::{
    get_color_scheme, get_config, renderer_info::RendererInfo, CanvasRenderer,
    TilingCanvasRenderer, Viewport, RENDER_CONFIGS,
};
use crate::state::{AppState, RendererState};
use leptos::*;
use std::time::Duration;

#[component]
pub fn App() -> impl IntoView {
    // ========== Load state from localStorage ==========
    let initial_state = AppState::load();

    let (selected_renderer_id, set_selected_renderer_id) =
        create_signal(initial_state.selected_renderer_id.clone());
    let (renderer_states, set_renderer_states) = create_signal(initial_state.renderer_states);

    // Get initial config
    let initial_config = get_config(&initial_state.selected_renderer_id).unwrap();
    let initial_renderer_state = initial_state
        .renderer_states
        .get(&initial_state.selected_renderer_id)
        .unwrap();

    // ========== Create initial renderer ==========
    let initial_renderer = (initial_config.create_renderer)();
    let initial_colorizer = get_color_scheme(
        initial_config,
        &initial_renderer_state.color_scheme_id,
    )
    .unwrap()
    .colorizer;

    let natural_bounds = initial_renderer.natural_bounds();
    let (viewport, set_viewport) = create_signal(initial_renderer_state.viewport.clone());

    // ========== Canvas renderer with cache ==========
    let canvas_renderer: RwSignal<Box<dyn CanvasRenderer>> = create_rw_signal(Box::new(
        TilingCanvasRenderer::new(initial_renderer, initial_colorizer, 128),
    ));

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
    create_effect(move |_| {
        let renderer_id = selected_renderer_id.get();
        let config = get_config(&renderer_id).unwrap();
        let states = renderer_states.get();
        let state = states.get(&renderer_id).unwrap();

        // Create new renderer
        let new_renderer = (config.create_renderer)();

        // Find colorizer for restored color scheme
        let colorizer = get_color_scheme(config, &state.color_scheme_id)
            .unwrap()
            .colorizer;

        // Update canvas renderer (invalidates cache)
        canvas_renderer.update(|cr| {
            cr.set_renderer(new_renderer);
            cr.set_colorizer(colorizer);
        });

        // Restore viewport
        set_viewport.set(state.viewport.clone());

        // Save immediately
        AppState {
            selected_renderer_id: renderer_id.clone(),
            renderer_states: states,
        }
        .save();
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
        let vp = viewport.get();
        let renderer_id = selected_renderer_id.get();

        set_renderer_states.update(|states| {
            if let Some(state) = states.get_mut(&renderer_id) {
                state.viewport = vp;
            }
        });

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
        let config = get_config(&renderer_id).unwrap();

        let colorizer = get_color_scheme(config, &scheme_id).unwrap().colorizer;

        canvas_renderer.update(|cr| {
            cr.set_colorizer(colorizer);
        });

        set_renderer_states.update(|states| {
            if let Some(state) = states.get_mut(&renderer_id) {
                state.color_scheme_id = scheme_id.clone();
            }
        });

        let states = renderer_states.get();
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
        let renderer_id = selected_renderer_id.get();
        let config = get_config(&renderer_id).unwrap();
        let renderer = (config.create_renderer)();
        let natural_bounds = renderer.natural_bounds();
        set_viewport.set(Viewport::new(natural_bounds.center(), 1.0));
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
                natural_bounds=natural_bounds
            />
            <UI
                info=renderer_info
                is_visible=ui_visibility.is_visible
                set_is_hovering=ui_visibility.set_is_hovering
                on_home_click=on_home_click
                on_fullscreen_click=on_fullscreen_click
                render_function_options=render_function_options
                selected_renderer_id=Signal::derive(move || selected_renderer_id.get())
                on_renderer_select=move |id: String| set_selected_renderer_id.set(id)
                color_scheme_options=color_scheme_options
                selected_color_scheme_id=Signal::derive(move || selected_color_scheme_id.get())
                on_color_scheme_select=on_color_scheme_select
            />
        </div>
    }
}
