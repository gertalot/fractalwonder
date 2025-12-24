//! Options dropdown menu with grouped sections for Effects and Cycles.

use crate::components::{Menu, MenuItem, MenuSection, StepperMenuItem};
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

    view! {
        <Menu is_open=is_open set_is_open=set_is_open label="Options">
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

            <MenuSection title="Debug" />
            <MenuItem
                active=xray_enabled
                on_click=on_xray_toggle
                label="X-ray"
                shortcut="[X]"
            />
        </Menu>
    }
}
