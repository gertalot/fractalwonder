// fractalwonder-ui/src/components/ui_panel.rs
use crate::components::{FullscreenButton, HomeButton, InfoButton};
use crate::config::FractalConfig;
use crate::hooks::{use_ui_visibility, UiVisibility};
use fractalwonder_core::Viewport;
use leptos::*;

#[component]
pub fn UIPanel(
    /// Canvas dimensions (width, height)
    canvas_size: Signal<(u32, u32)>,
    /// Current viewport in fractal space
    viewport: Signal<Viewport>,
    /// Current fractal configuration
    config: Signal<&'static FractalConfig>,
    /// Calculated precision bits
    precision_bits: Signal<usize>,
    /// Callback when home button is clicked
    on_home_click: Callback<()>,
) -> impl IntoView {
    let UiVisibility {
        is_visible,
        is_hovering: _,
        set_is_visible: _,
        set_is_hovering,
    } = use_ui_visibility();

    // Info panel state - lifted here so we can prevent auto-hide when open
    let (is_info_open, set_is_info_open) = create_signal(false);

    // Prevent auto-hide when info panel is open
    create_effect(move |_| {
        if is_info_open.get() {
            set_is_hovering.set(true);
        }
    });

    let opacity_class = move || {
        if is_visible.get() {
            "opacity-100"
        } else {
            "opacity-0 pointer-events-none"
        }
    };

    view! {
        <div
            class=move || format!(
                "fixed inset-x-0 bottom-0 z-50 transition-opacity duration-300 {}",
                opacity_class()
            )
            on:mouseenter=move |_| set_is_hovering.set(true)
            on:mouseleave=move |_| {
                // Don't set hovering to false if info panel is open
                if !is_info_open.get() {
                    set_is_hovering.set(false)
                }
            }
        >
            <div class="flex items-center justify-between px-4 py-3 bg-black/50 backdrop-blur-sm">
                // Left section: info button and home button
                <div class="flex items-center space-x-2">
                    <InfoButton is_open=is_info_open set_is_open=set_is_info_open />
                    <HomeButton on_click=on_home_click />
                </div>

                // Center section: fractal info
                <div class="flex-1 text-center text-white text-xs font-mono">
                    {move || {
                        let cfg = config.get();
                        let vp = viewport.get();
                        let bits = precision_bits.get();
                        let (canvas_w, canvas_h) = canvas_size.get();

                        let cx = format_coordinate_from_log2(vp.center.0.log2_approx());
                        let cy = format_coordinate_from_log2(vp.center.1.log2_approx());
                        let w = format_dimension_from_log2(vp.width.log2_approx());
                        let h = format_dimension_from_log2(vp.height.log2_approx());

                        format!(
                            "{} | Center: ({}, {}) | Size: {} x {} | Canvas: {}x{} | Precision: {} bits",
                            cfg.display_name, cx, cy, w, h, canvas_w, canvas_h, bits
                        )
                    }}
                </div>

                // Right section: fullscreen
                <div>
                    <FullscreenButton />
                </div>
            </div>
        </div>
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

/// Format a dimension for display using log2 approximation.
///
/// Works at any zoom level, including extreme depths beyond f64 range.
fn format_dimension_from_log2(log2_val: f64) -> String {
    use std::f64::consts::LOG2_10;

    if log2_val.is_infinite() || log2_val.is_nan() {
        return "0".to_string();
    }

    // Convert log2 to log10 for display
    let log10_val = log2_val / LOG2_10;

    // If exponent is in displayable range, show the actual value
    if log10_val > -15.0 && log10_val < 15.0 {
        let val = 10.0_f64.powf(log10_val);
        if val < 0.001 {
            format!("{:.2e}", val)
        } else if val < 1.0 {
            format!("{:.4}", val)
        } else {
            format!("{:.2}", val)
        }
    } else {
        // Extreme value - show as power of 10
        format!("~10^{:.0}", log10_val)
    }
}
