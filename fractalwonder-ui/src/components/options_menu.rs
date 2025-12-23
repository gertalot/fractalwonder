//! Options dropdown menu with grouped sections for Effects and Cycles.

use crate::components::{MenuItem, MenuSection, StepperMenuItem};
use crate::rendering::colorizers::settings::{MAX_TRANSFER_BIAS, MIN_TRANSFER_BIAS};
use leptos::*;

#[component]
pub fn OptionsMenu(
    /// Menu open state
    is_open: ReadSignal<bool>,
    /// Set menu open state
    set_is_open: WriteSignal<bool>,
    /// 3D shading enabled state
    shading_enabled: Signal<bool>,
    /// Callback when 3D is toggled
    on_shading_toggle: Callback<()>,
    /// Smooth iteration enabled state
    smooth_enabled: Signal<bool>,
    /// Callback when smooth is toggled
    on_smooth_toggle: Callback<()>,
    /// Histogram equalization enabled state
    histogram_enabled: Signal<bool>,
    /// Callback when histogram is toggled
    on_histogram_toggle: Callback<()>,
    /// Current cycle count
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
    /// GPU rendering enabled state
    use_gpu: Signal<bool>,
    /// Callback when GPU toggle is clicked
    on_gpu_toggle: Callback<()>,
    /// X-ray mode enabled state
    xray_enabled: Signal<bool>,
    /// Callback when X-ray toggle is clicked
    on_xray_toggle: Callback<()>,
) -> impl IntoView {
    // Derived signals for stepper bounds
    let cycle_at_min = Signal::derive(move || cycle_count.get() <= 1);
    let cycle_at_max = Signal::derive(move || cycle_count.get() >= 1024);
    let bias_at_min = Signal::derive(move || transfer_bias.get() <= MIN_TRANSFER_BIAS);
    let bias_at_max = Signal::derive(move || transfer_bias.get() >= MAX_TRANSFER_BIAS);

    view! {
        <div class="relative">
            <button
                class="text-white hover:text-gray-200 hover:bg-white/10 rounded-lg px-3 py-2 transition-colors flex items-center gap-2"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                <span class="text-sm">"Options"</span>
                <span class="text-xs opacity-70">"▾"</span>
            </button>

            {move || is_open.get().then(|| view! {
                <div class="absolute bottom-full mb-2 left-0 min-w-48 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg overflow-hidden">
                    <MenuSection title="Renderer" show_top_border=false />
                    <MenuItem
                        active=use_gpu
                        on_click=on_gpu_toggle
                        label="Use GPU"
                        shortcut="[G]"
                    />

                    <MenuSection title="Effects" />
                    <MenuItem
                        active=shading_enabled
                        on_click=on_shading_toggle
                        label="3D"
                        shortcut="[3]"
                    />
                    <MenuItem
                        active=smooth_enabled
                        on_click=on_smooth_toggle
                        label="Smooth"
                        shortcut="[S]"
                    />
                    <MenuItem
                        active=histogram_enabled
                        on_click=on_histogram_toggle
                        label="Histogram"
                        shortcut="[H]"
                    />

                    <MenuSection title="Cycles" />
                    <StepperMenuItem
                        value=cycle_count
                        on_decrease=on_cycle_down
                        on_increase=on_cycle_up
                        format_value=|v: u32| v.to_string()
                        is_at_min=cycle_at_min
                        is_at_max=cycle_at_max
                        shortcut="[↑↓ / ⇧±50]"
                    />

                    <MenuSection title="Bias" />
                    <StepperMenuItem
                        value=transfer_bias
                        on_decrease=on_bias_down
                        on_increase=on_bias_up
                        format_value=|v: f32| format!("{:.1}", v)
                        is_at_min=bias_at_min
                        is_at_max=bias_at_max
                        shortcut="[[ ]]"
                        value_width="min-w-12"
                    />

                    <MenuSection title="Debug" />
                    <MenuItem
                        active=xray_enabled
                        on_click=on_xray_toggle
                        label="X-ray"
                        shortcut="[X]"
                    />
                </div>
            })}
        </div>
    }
}
