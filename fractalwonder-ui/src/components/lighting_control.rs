//! Circular light direction control for azimuth and elevation.

use leptos::*;
use std::f64::consts::{FRAC_PI_2, PI};

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

    // Suppress unused warnings until drag interaction is added in Task 3
    let _ = &on_change;
    let _ = &circle_ref;

    // Convert radians to position percentage
    let position = Signal::derive(move || {
        let az = azimuth.get();
        let el = elevation.get();

        // Angle from top, clockwise
        let angle = az - FRAC_PI_2;
        // Radius: center = 0%, edge = 50%
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
