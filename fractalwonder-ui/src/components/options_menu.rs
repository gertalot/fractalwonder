//! Options dropdown menu with renderer settings and cycles.
//! Note: 3D, Smooth, Histogram are palette properties, not options.

use crate::components::{Menu, MenuItem, MenuSection, StepperMenuItem};
use leptos::*;

#[component]
pub fn OptionsMenu(
    /// Menu open state
    is_open: ReadSignal<bool>,
    /// Set menu open state
    set_is_open: WriteSignal<bool>,
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
    /// Current CPU threads setting (0 = all cores, negative = leave cores free)
    cpu_threads: Signal<i32>,
    /// Callback to increase CPU threads
    on_cpu_threads_up: Callback<()>,
    /// Callback to decrease CPU threads
    on_cpu_threads_down: Callback<()>,
    /// Whether CPU threads is at minimum bound
    cpu_threads_at_min: Signal<bool>,
    /// Whether CPU threads is at maximum bound
    cpu_threads_at_max: Signal<bool>,
    /// X-ray mode enabled state
    xray_enabled: Signal<bool>,
    /// Callback when X-ray toggle is clicked
    on_xray_toggle: Callback<()>,
    /// Force HDRFloat mode enabled state
    force_hdr_float: Signal<bool>,
    /// Callback when Force HDRFloat toggle is clicked
    on_force_hdr_float_toggle: Callback<()>,
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

            <MenuSection title="CPU Threads" />
            <StepperMenuItem
                value=cpu_threads
                on_decrease=on_cpu_threads_down
                on_increase=on_cpu_threads_up
                format_value=|v: i32| v.to_string()
                is_at_min=cpu_threads_at_min
                is_at_max=cpu_threads_at_max
                shortcut=""
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
            <MenuItem
                active=force_hdr_float
                on_click=on_force_hdr_float_toggle
                label="Use HDRFloat"
                shortcut="[H]"
            />
        </Menu>
    }
}
