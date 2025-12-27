//! Reusable slider component for lighting parameters.

use leptos::*;

/// Slider with label and value display for lighting parameters.
#[component]
pub fn LightingSlider(
    /// Label text displayed on the left
    label: &'static str,
    /// Current value signal
    value: Signal<f64>,
    /// Called when value changes
    on_change: Callback<f64>,
    /// Minimum value
    min: f64,
    /// Maximum value
    max: f64,
    /// Step increment
    step: f64,
    /// Decimal places for value display
    #[prop(default = 2)]
    precision: u8,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <div class="text-white text-xs w-20">{label}</div>
            <input
                type="range"
                class="flex-1 accent-white"
                prop:min=min
                prop:max=max
                prop:step=step
                prop:value=move || value.get()
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                        on_change.call(v);
                    }
                }
            />
            <div class="text-white text-xs w-10 text-right">
                {move || format!("{:.prec$}", value.get(), prec = precision as usize)}
            </div>
        </div>
    }
}
