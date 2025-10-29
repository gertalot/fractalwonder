use crate::components::interactive_canvas::InteractiveCanvas;
use crate::components::ui::UI;
use crate::hooks::fullscreen::toggle_fullscreen;
use crate::hooks::ui_visibility::use_ui_visibility;
use crate::rendering::{
    renderer_info::RendererInfo, test_image_colorizer, AppData, AppDataRenderer, Colorizer,
    PixelRenderer, Renderer, TestImageComputer, TilingCanvasRenderer, Viewport,
};
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    // ========== Domain state ==========
    let test_computer = TestImageComputer::new();

    // Create renderer chain: TestImageComputer → PixelRenderer → AppDataRenderer
    let pixel_renderer = PixelRenderer::new(test_computer.clone());
    let app_renderer = AppDataRenderer::new(pixel_renderer, |d| AppData::TestImageData(*d));

    let (colorizer, _set_colorizer) = create_signal(test_image_colorizer as Colorizer<AppData>);

    let natural_bounds = app_renderer.natural_bounds();
    let natural_bounds_for_home = natural_bounds.clone();
    let (viewport, set_viewport) = create_signal(Viewport::new(natural_bounds.center(), 1.0));

    // ========== Canvas renderer with cache ==========
    let canvas_renderer = create_rw_signal(TilingCanvasRenderer::new(
        app_renderer,
        colorizer.get_untracked(),
        128,
    ));

    // Effect: Colorizer changed → preserve cache
    create_effect(move |_| {
        let new_colorizer = colorizer.get();
        canvas_renderer.update(|cr| {
            *cr = cr.with_colorizer(new_colorizer); // Arc::clone preserves cache!
        });
    });

    // ========== RendererInfo for UI display ==========
    let (render_time_ms, set_render_time_ms) = create_signal(None::<f64>);

    let (renderer_info, set_renderer_info) =
        create_signal(test_computer.info(&viewport.get_untracked()));

    // Effect: Update renderer info when viewport or render time changes
    create_effect(move |_| {
        let vp = viewport.get();
        let mut info = test_computer.info(&vp);
        info.render_time_ms = render_time_ms.get();
        set_renderer_info.set(info);
    });

    // ========== UI visibility and callbacks ==========
    let ui_visibility = use_ui_visibility();

    let on_home_click = move || {
        set_viewport.set(Viewport::new(natural_bounds_for_home.center(), 1.0));
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
            />
        </div>
    }
}
