// fractalwonder-ui/src/components/ui_panel.rs
use crate::components::{FullscreenButton, HomeButton, InfoMenu, OptionsMenu, PaletteMenu};
use crate::config::FractalConfig;
use crate::rendering::RenderProgress;
use fractalwonder_core::{calculate_max_iterations, BigFloat, Viewport};
use leptos::*;

#[component]
pub fn UIPanel(
    /// Current viewport in fractal space
    viewport: Signal<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Calculated precision bits
    precision_bits: Signal<usize>,
    /// Callback when home button is clicked
    on_home_click: Callback<()>,
    /// Palette options (id, display_name)
    palette_options: Signal<Vec<(String, String)>>,
    /// Currently selected palette ID
    selected_palette_id: Signal<String>,
    /// Callback when palette is selected
    on_palette_select: Callback<String>,
    /// 3D shading enabled
    shading_enabled: Signal<bool>,
    /// Callback to toggle 3D
    on_shading_toggle: Callback<()>,
    /// Smooth iteration enabled
    smooth_enabled: Signal<bool>,
    /// Callback to toggle smooth
    on_smooth_toggle: Callback<()>,
    /// Histogram equalization enabled
    histogram_enabled: Signal<bool>,
    /// Callback to toggle histogram
    on_histogram_toggle: Callback<()>,
    /// Cycle count
    cycle_count: Signal<u32>,
    /// Callback to increase cycles
    on_cycle_up: Callback<()>,
    /// Callback to decrease cycles
    on_cycle_down: Callback<()>,
    /// Current transfer bias
    transfer_bias: Signal<f32>,
    /// Callback to increase bias
    on_bias_up: Callback<()>,
    /// Callback to decrease bias
    on_bias_down: Callback<()>,
    /// GPU rendering enabled
    use_gpu: Signal<bool>,
    /// Callback to toggle GPU
    on_gpu_toggle: Callback<()>,
    /// Render progress signal
    render_progress: Signal<RwSignal<RenderProgress>>,
    /// UI visibility signal (from parent)
    is_visible: ReadSignal<bool>,
    /// Set hovering state (from parent)
    set_is_hovering: WriteSignal<bool>,
    /// Callback to cancel current render
    on_cancel: Callback<()>,
    /// X-ray mode enabled state
    xray_enabled: ReadSignal<bool>,
    /// Callback to toggle x-ray mode
    set_xray_enabled: WriteSignal<bool>,
) -> impl IntoView {
    // Menu open states - lifted here for coordination
    let (is_info_open, set_is_info_open) = create_signal(false);
    let (is_palette_open, set_is_palette_open) = create_signal(false);
    let (is_options_open, set_is_options_open) = create_signal(false);

    // Ensure only one menu is open at a time
    create_effect(move |_| {
        if is_info_open.get() {
            set_is_palette_open.set(false);
            set_is_options_open.set(false);
        }
    });
    create_effect(move |_| {
        if is_palette_open.get() {
            set_is_info_open.set(false);
            set_is_options_open.set(false);
        }
    });
    create_effect(move |_| {
        if is_options_open.get() {
            set_is_info_open.set(false);
            set_is_palette_open.set(false);
        }
    });

    // Check if any menu is open
    let any_menu_open =
        move || is_info_open.get() || is_palette_open.get() || is_options_open.get();

    // Track mouse position over the panel (separate from menu state)
    let (is_mouse_over, set_is_mouse_over) = create_signal(false);

    // Derive is_hovering for parent: blocks auto-hide when mouse over panel OR menu open
    create_effect(move |_| {
        set_is_hovering.set(is_mouse_over.get() || any_menu_open());
    });

    let opacity_class = move || {
        if is_visible.get() {
            "opacity-100"
        } else {
            "opacity-0 pointer-events-none"
        }
    };

    // Close all menus
    let close_all_menus = move || {
        set_is_info_open.set(false);
        set_is_palette_open.set(false);
        set_is_options_open.set(false);
    };

    view! {
        // Fullscreen overlay: closes menus when clicking outside (canvas, etc.)
        {move || any_menu_open().then(|| view! {
            <div
                class="fixed inset-0 z-40"
                on:click=move |_| close_all_menus()
            />
        })}

        <div
            class=move || format!(
                "fixed inset-x-0 bottom-0 z-50 transition-opacity duration-300 {}",
                opacity_class()
            )
            on:mouseenter=move |_| set_is_mouse_over.set(true)
            on:mouseleave=move |_| set_is_mouse_over.set(false)
            on:click=move |_| close_all_menus()
        >
            <div class="flex items-center justify-between px-4 py-3 bg-black/50 backdrop-blur-sm">
                // Left section: info button, home button, and menus
                <div class="flex items-center space-x-2">
                    <InfoMenu
                        is_open=is_info_open
                        set_is_open=set_is_info_open
                    />
                    <HomeButton on_click=on_home_click />
                    <PaletteMenu
                        is_open=is_palette_open
                        set_is_open=set_is_palette_open
                        label="Palette".to_string()
                        options=palette_options
                        selected_id=selected_palette_id
                        on_select=move |id| on_palette_select.call(id)
                    />
                    <OptionsMenu
                        is_open=is_options_open
                        set_is_open=set_is_options_open
                        shading_enabled=shading_enabled
                        on_shading_toggle=on_shading_toggle
                        smooth_enabled=smooth_enabled
                        on_smooth_toggle=on_smooth_toggle
                        histogram_enabled=histogram_enabled
                        on_histogram_toggle=on_histogram_toggle
                        cycle_count=cycle_count
                        on_cycle_up=on_cycle_up
                        on_cycle_down=on_cycle_down
                        transfer_bias=transfer_bias
                        on_bias_up=on_bias_up
                        on_bias_down=on_bias_down
                        use_gpu=use_gpu
                        on_gpu_toggle=on_gpu_toggle
                        xray_enabled=xray_enabled.into()
                        on_xray_toggle=Callback::new(move |_| set_xray_enabled.update(|v| *v = !*v))
                    />
                </div>

                // Center section: fractal info
                <div class="flex-1 text-center text-white text-xs font-mono">
                    <div>
                        {move || {
                            use std::f64::consts::LOG2_10;

                            let cfg = config.get();
                            let vp = viewport.get();
                            let bits = precision_bits.get();

                            // Calculate zoom: reference_width / current_width via log subtraction
                            let reference_width = cfg.default_viewport(bits).width;
                            let zoom_log2 = reference_width.log2_approx() - vp.width.log2_approx();
                            let zoom = format_zoom_from_log2(zoom_log2);

                            // Calculate max iterations from zoom exponent (log10)
                            let zoom_exponent = zoom_log2 / LOG2_10;
                            let max_iter = calculate_max_iterations(
                                zoom_exponent,
                                cfg.iteration_multiplier,
                                cfg.iteration_power,
                            );

                            let cx = format_signed_coordinate(&vp.center.0);
                            let cy = format_signed_coordinate(&vp.center.1);

                            format!(
                                "Zoom: {} | Center: ({}, {}) | {} bits | {} max iterations",
                                zoom, cx, cy, bits, max_iter
                            )
                        }}
                    </div>
                    <div class="mt-1 flex items-center justify-center gap-2">
                        <span>
                            {move || {
                                let progress_signal = render_progress.get();
                                let progress = progress_signal.get();

                                if progress.total_steps > 0 && !progress.is_complete {
                                    // During render: show progress and elapsed time
                                    format!(
                                        "Rendering: {}/{} ({:.1}s)",
                                        progress.completed_steps,
                                        progress.total_steps,
                                        progress.elapsed_ms / 1000.0
                                    )
                                } else if progress.is_complete && progress.total_steps > 0 {
                                    // After completion: show total render time
                                    format!("Rendered in {:.2}s", progress.elapsed_ms / 1000.0)
                                } else {
                                    String::new()
                                }
                            }}
                        </span>
                        // Cancel button - only visible during active render
                        {move || {
                            let progress_signal = render_progress.get();
                            let progress = progress_signal.get();
                            let is_rendering = progress.total_steps > 0 && !progress.is_complete;

                            if is_rendering {
                                view! {
                                    <button
                                        class="text-white/50 hover:text-white/90 transition-colors cursor-pointer text-sm leading-none"
                                        on:click=move |_| on_cancel.call(())
                                        title="Cancel render"
                                    >
                                        "×"
                                    </button>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                    </div>
                </div>

                // Right section: fullscreen
                <div>
                    <FullscreenButton />
                </div>
            </div>
        </div>
    }
}

/// Format a signed coordinate for display, preserving the sign.
///
/// Uses log2_approx for magnitude (which uses abs internally) and
/// to_f64() to determine the sign.
fn format_signed_coordinate(coord: &BigFloat) -> String {
    let is_negative = coord.to_f64() < 0.0;
    let magnitude = format_coordinate_from_log2(coord.log2_approx());

    if is_negative {
        format!("-{}", magnitude)
    } else {
        magnitude
    }
}

/// Format a coordinate for display using log2 approximation.
///
/// For coordinates that are 0 or very close to 0, log2_approx returns -inf.
/// For extreme values beyond f64 range, we display the exponent directly.
fn format_coordinate_from_log2(log2_val: f64) -> String {
    use std::f64::consts::LOG2_10;

    if log2_val.is_infinite() || log2_val.is_nan() {
        return "0".to_string();
    }

    // Convert log2 to log10 for display
    let log10_val = log2_val / LOG2_10;

    // If exponent is in displayable range, show the actual value
    if log10_val.abs() < 15.0 {
        // Reconstruct value for display (safe for f64 range)
        let val = 10.0_f64.powf(log10_val);
        if val.abs() < 0.0001 || val.abs() >= 10000.0 {
            format!("{:.4e}", val)
        } else {
            format!("{:.6}", val)
        }
    } else {
        // Extreme value - show as power of 10
        format!("~10^{:.0}", log10_val)
    }
}

/// Format a zoom level for display using log2 approximation.
///
/// Produces: "1×", "150×", "1.50 × 10^3", "10^2000"
fn format_zoom_from_log2(log2_val: f64) -> String {
    use std::f64::consts::LOG2_10;

    if log2_val.is_nan() || log2_val.is_infinite() {
        return "1×".to_string();
    }

    let log10_val = log2_val / LOG2_10;
    let exponent = log10_val.floor() as i64;
    let mantissa = 10.0_f64.powf(log10_val - exponent as f64);

    if exponent < 3 {
        // Simple format: "1×", "150×"
        let zoom = 10.0_f64.powf(log10_val);
        format!("{:.0}×", zoom)
    } else if mantissa < 1.05 {
        // Drop mantissa when ≈1: "10^3", "10^2000"
        format!("10^{}", exponent)
    } else {
        // Scientific: "1.50 × 10^3"
        format!("{:.2} × 10^{}", mantissa, exponent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::LOG2_10;

    #[test]
    fn format_zoom_1x() {
        assert_eq!(format_zoom_from_log2(0.0), "1×");
    }

    #[test]
    fn format_zoom_10x() {
        assert_eq!(format_zoom_from_log2(LOG2_10), "10×");
    }

    #[test]
    fn format_zoom_100x() {
        assert_eq!(format_zoom_from_log2(2.0 * LOG2_10), "100×");
    }

    #[test]
    fn format_zoom_1000x_becomes_scientific() {
        assert_eq!(format_zoom_from_log2(3.0 * LOG2_10), "10^3");
    }

    #[test]
    fn format_zoom_with_mantissa() {
        // 1.5 × 10^3 = 1500
        let log2_1500 = (1500.0_f64).log2();
        assert_eq!(format_zoom_from_log2(log2_1500), "1.50 × 10^3");
    }

    #[test]
    fn format_zoom_extreme() {
        // 10^2000
        assert_eq!(format_zoom_from_log2(2000.0 * LOG2_10), "10^2000");
    }

    #[test]
    fn format_zoom_nan_returns_1x() {
        assert_eq!(format_zoom_from_log2(f64::NAN), "1×");
    }

    #[test]
    fn format_zoom_infinity_returns_1x() {
        assert_eq!(format_zoom_from_log2(f64::INFINITY), "1×");
    }
}
