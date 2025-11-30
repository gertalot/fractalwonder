//! Options dropdown menu with grouped sections for Effects and Cycles.
//!
//! Note: The `unused_parens` allow is required because the Leptos `view!` macro
//! misparses `>=` operators (interpreting `>` as HTML syntax).

#![allow(unused_parens)]

use leptos::*;

#[component]
pub fn OptionsMenu(
    /// 3D shading enabled state
    shading_enabled: Signal<bool>,
    /// Callback when 3D is toggled
    on_shading_toggle: Callback<()>,
    /// Smooth iteration enabled state
    smooth_enabled: Signal<bool>,
    /// Callback when smooth is toggled
    on_smooth_toggle: Callback<()>,
    /// Current cycle count
    cycle_count: Signal<u32>,
    /// Callback to increase cycles
    on_cycle_up: Callback<()>,
    /// Callback to decrease cycles
    on_cycle_down: Callback<()>,
) -> impl IntoView {
    let (is_open, set_is_open) = create_signal(false);

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
                    // Effects section
                    <div class="px-3 py-2 text-xs text-gray-400 uppercase tracking-wider border-b border-gray-800">
                        "Effects"
                    </div>
                    <button
                        class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center justify-between"
                        on:click=move |_| {
                            on_shading_toggle.call(());
                        }
                    >
                        <span class="flex items-center gap-2">
                            <span class=move || if shading_enabled.get() { "opacity-100" } else { "opacity-30" }>
                                {move || if shading_enabled.get() { "☑" } else { "☐" }}
                            </span>
                            "3D"
                        </span>
                        <span class="text-xs text-gray-500">"[3]"</span>
                    </button>
                    <button
                        class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center justify-between"
                        on:click=move |_| {
                            on_smooth_toggle.call(());
                        }
                    >
                        <span class="flex items-center gap-2">
                            <span class=move || if smooth_enabled.get() { "opacity-100" } else { "opacity-30" }>
                                {move || if smooth_enabled.get() { "☑" } else { "☐" }}
                            </span>
                            "Smooth"
                        </span>
                        <span class="text-xs text-gray-500">"[S]"</span>
                    </button>

                    // Cycles section
                    <div class="px-3 py-2 text-xs text-gray-400 uppercase tracking-wider border-t border-b border-gray-800">
                        "Cycles"
                    </div>
                    <div class="px-4 py-2 text-sm text-gray-300 flex items-center justify-between">
                        <div class="flex items-center gap-3">
                            <button
                                class="text-gray-400 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed"
                                on:click=move |_| on_cycle_down.call(())
                                prop:disabled=move || (cycle_count.get() <= 1)
                            >
                                "◀"
                            </button>
                            <span class="min-w-8 text-center font-mono">{move || cycle_count.get()}</span>
                            <button
                                class="text-gray-400 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed"
                                on:click=move |_| on_cycle_up.call(())
                                prop:disabled=move || (cycle_count.get() >= 1024)
                            >
                                "▶"
                            </button>
                        </div>
                        <span class="text-xs text-gray-500">"[↑↓]"</span>
                    </div>
                </div>
            })}
        </div>
    }
}
