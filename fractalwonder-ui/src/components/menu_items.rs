//! Reusable menu item components for consistent UI across menus.

use leptos::*;

/// A menu item with selection/enabled indicator, label, and optional shortcut.
#[component]
pub fn MenuItem(
    /// Whether this item shows as active (enabled for toggles, selected for lists)
    #[prop(into)]
    active: Signal<bool>,
    /// Display label
    #[prop(into)]
    label: String,
    /// Optional keyboard shortcut hint (e.g., "[G]")
    #[prop(optional, into)]
    shortcut: Option<String>,
    /// Click handler
    on_click: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white rounded transition-colors flex items-center justify-between"
            on:click=move |_| on_click.call(())
        >
            <span class="flex items-center gap-2">
                <span class="w-4 text-center">
                    {move || if active.get() { "✓" } else { " " }}
                </span>
                {label.clone()}
            </span>
            {shortcut.map(|s| view! { <span class="text-xs text-gray-500">{s}</span> })}
        </button>
    }
}

/// A stepper menu item with decrease/increase buttons and keyboard shortcut.
#[allow(unused_parens)]
#[component]
pub fn StepperMenuItem<T, F>(
    /// Current value
    value: Signal<T>,
    /// Callback to decrease value
    on_decrease: Callback<()>,
    /// Callback to increase value
    on_increase: Callback<()>,
    /// Format function for displaying the value
    format_value: F,
    /// Minimum value check
    is_at_min: Signal<bool>,
    /// Maximum value check
    is_at_max: Signal<bool>,
    /// Keyboard shortcut hint
    #[prop(into)]
    shortcut: String,
    /// Minimum width for value display
    #[prop(default = "min-w-8".to_string())]
    #[prop(into)]
    value_width: String,
) -> impl IntoView
where
    T: Clone + 'static,
    F: Fn(T) -> String + 'static + Copy,
{
    view! {
        <div class="px-4 py-2 text-sm text-gray-300 flex items-center justify-between">
            <div class="flex items-center gap-3">
                <button
                    class="text-gray-400 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    on:click=move |_| on_decrease.call(())
                    prop:disabled=move || is_at_min.get()
                >
                    "◀"
                </button>
                <span class=format!("{} text-center font-mono", value_width)>
                    {move || format_value(value.get())}
                </span>
                <button
                    class="text-gray-400 hover:text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    on:click=move |_| on_increase.call(())
                    prop:disabled=move || is_at_max.get()
                >
                    "▶"
                </button>
            </div>
            <span class="text-xs text-gray-500">{shortcut.clone()}</span>
        </div>
    }
}

/// A section header divider for menus.
#[component]
pub fn MenuSection(
    /// Section title
    #[prop(into)]
    title: String,
    /// Whether to show top border (default: true for non-first sections)
    #[prop(default = true)]
    show_top_border: bool,
) -> impl IntoView {
    let border_class = if show_top_border {
        "border-t border-b border-gray-800"
    } else {
        "border-b border-gray-800"
    };

    view! {
        <div class=format!("px-3 py-2 text-xs text-gray-400 uppercase tracking-wider {}", border_class)>
            {title}
        </div>
    }
}
