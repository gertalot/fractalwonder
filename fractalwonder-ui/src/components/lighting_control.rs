//! Circular light direction control for azimuth and elevation.

use leptos::*;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Circular picker for light direction.
///
/// Azimuth and elevation are in radians. Display shows degrees.
#[component]
pub fn LightingControl(
    /// Light azimuth in radians (0 = top, clockwise)
    azimuth: Signal<f64>,
    /// Light elevation in radians (0 = horizon, PI/2 = overhead)
    elevation: Signal<f64>,
    /// Called when direction changes (azimuth, elevation) in radians
    on_change: Callback<(f64, f64)>,
) -> impl IntoView {
    let circle_ref = create_node_ref::<leptos::html::Div>();
    let is_dragging = create_rw_signal(false);

    // Calculate azimuth/elevation from mouse position
    let calculate_from_mouse = move |client_x: i32, client_y: i32| {
        let Some(circle) = circle_ref.get() else {
            return;
        };
        let rect = circle.get_bounding_client_rect();

        let center_x = rect.left() + rect.width() / 2.0;
        let center_y = rect.top() + rect.height() / 2.0;
        let radius = rect.width() / 2.0;

        let dx = (client_x as f64 - center_x) / radius;
        let dy = (client_y as f64 - center_y) / radius;

        // Azimuth: atan2 + 90° offset (so 0 = top)
        let mut new_azimuth = dy.atan2(dx) + FRAC_PI_2;
        if new_azimuth < 0.0 {
            new_azimuth += TAU;
        }

        // Elevation: center = PI/2, edge = 0
        let distance = (dx * dx + dy * dy).sqrt().min(1.0);
        let new_elevation = FRAC_PI_2 * (1.0 - distance);

        on_change.call((new_azimuth, new_elevation));
    };

    // Document-level mouse handlers with proper cleanup
    type MouseClosure = Closure<dyn Fn(web_sys::MouseEvent)>;
    let mousemove_handler = store_value(None::<MouseClosure>);
    let mouseup_handler = store_value(None::<MouseClosure>);

    {
        let window = web_sys::window().expect("window");
        let document = window.document().expect("document");

        let mousemove_closure =
            Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                // Guard against disposed signals
                if is_dragging.try_get().unwrap_or(false) {
                    calculate_from_mouse(e.client_x(), e.client_y());
                }
            });

        let mouseup_closure =
            Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
                let _ = is_dragging.try_set(false);
            });

        let _ = document.add_event_listener_with_callback(
            "mousemove",
            mousemove_closure.as_ref().unchecked_ref(),
        );
        let _ = document
            .add_event_listener_with_callback("mouseup", mouseup_closure.as_ref().unchecked_ref());

        mousemove_handler.set_value(Some(mousemove_closure));
        mouseup_handler.set_value(Some(mouseup_closure));
    }

    // Cleanup event listeners when component is disposed
    on_cleanup(move || {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                mousemove_handler.with_value(|opt| {
                    if let Some(handler) = opt {
                        let _ = document.remove_event_listener_with_callback(
                            "mousemove",
                            handler.as_ref().unchecked_ref(),
                        );
                    }
                });
                mouseup_handler.with_value(|opt| {
                    if let Some(handler) = opt {
                        let _ = document.remove_event_listener_with_callback(
                            "mouseup",
                            handler.as_ref().unchecked_ref(),
                        );
                    }
                });
            }
        }
        mousemove_handler.set_value(None);
        mouseup_handler.set_value(None);
    });

    // Convert radians to position percentage
    let position = Signal::derive(move || {
        let az = azimuth.get();
        let el = elevation.get();

        let angle = az - FRAC_PI_2;
        let radius_pct = (1.0 - el / FRAC_PI_2) * 50.0;

        let x = 50.0 + radius_pct * angle.cos();
        let y = 50.0 + radius_pct * angle.sin();
        (x, y)
    });

    // Display in degrees
    let azimuth_deg = Signal::derive(move || (azimuth.get() * 180.0 / PI).round() as i32);
    let elevation_deg = Signal::derive(move || (elevation.get() * 180.0 / PI).round() as i32);

    view! {
        <div class="bg-white/5 border border-white/10 rounded-lg p-3 space-y-3">
            <div
                node_ref=circle_ref
                class="relative w-full aspect-square bg-white/5 rounded-full border border-white/20 cursor-crosshair"
                on:mousedown=move |e| {
                    e.prevent_default();
                    is_dragging.set(true);
                    calculate_from_mouse(e.client_x(), e.client_y());
                }
            >
                // Center dot
                <div class="absolute top-1/2 left-1/2 w-2 h-2 bg-white/30 rounded-full -translate-x-1/2 -translate-y-1/2" />

                // Concentric guide circles
                {[0.25, 0.5, 0.75, 1.0].into_iter().map(|r| {
                    let size = format!("{}%", r * 100.0);
                    view! {
                        <div
                            class="absolute top-1/2 left-1/2 border border-white/10 rounded-full -translate-x-1/2 -translate-y-1/2"
                            style:width=size.clone()
                            style:height=size
                        />
                    }
                }).collect_view()}

                // Light position indicator
                <div
                    class="absolute w-4 h-4 bg-white rounded-full shadow-lg -translate-x-1/2 -translate-y-1/2"
                    style:left=move || format!("{}%", position.get().0)
                    style:top=move || format!("{}%", position.get().1)
                />
            </div>

            // Azimuth/Elevation display
            <div class="grid grid-cols-2 gap-2 text-xs">
                <div class="space-y-0.5">
                    <div class="text-white/70">"Azimuth"</div>
                    <div class="text-white">{move || format!("{}°", azimuth_deg.get())}</div>
                </div>
                <div class="space-y-0.5">
                    <div class="text-white/70">"Elevation"</div>
                    <div class="text-white">{move || format!("{}°", elevation_deg.get())}</div>
                </div>
            </div>

            <div class="text-white/50 text-xs">
                "Drag to adjust light direction"
            </div>
        </div>
    }
}
