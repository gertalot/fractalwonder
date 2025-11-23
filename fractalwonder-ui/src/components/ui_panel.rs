// fractalwonder-ui/src/components/ui_panel.rs
use crate::components::{FullscreenButton, InfoButton};
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
                // Left section: info button
                <div class="flex items-center space-x-2">
                    <InfoButton is_open=is_info_open set_is_open=set_is_info_open />
                </div>

                // Center section: fractal info
                <div class="flex-1 text-center text-white text-sm font-mono">
                    {move || {
                        let cfg = config.get();
                        let vp = viewport.get();
                        let bits = precision_bits.get();
                        let (canvas_w, canvas_h) = canvas_size.get();

                        let cx = format_coordinate(vp.center.0.to_f64());
                        let cy = format_coordinate(vp.center.1.to_f64());
                        let w = format_dimension(vp.width.to_f64());
                        let h = format_dimension(vp.height.to_f64());

                        format!(
                            "Fractal: {} | Viewport: ({}, {}) | {} x {} | Precision: {} bits | Canvas: {} x {}",
                            cfg.display_name, cx, cy, w, h, bits, canvas_w, canvas_h
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

/// Format a coordinate for display (6 significant figures)
fn format_coordinate(val: f64) -> String {
    if val.abs() < 0.0001 || val.abs() >= 10000.0 {
        format!("{:.4e}", val)
    } else {
        format!("{:.6}", val)
    }
}

/// Format a dimension for display (scientific notation for small values)
fn format_dimension(val: f64) -> String {
    if val < 0.001 {
        format!("{:.2e}", val)
    } else if val < 1.0 {
        format!("{:.4}", val)
    } else {
        format!("{:.2}", val)
    }
}
