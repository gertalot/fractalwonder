//! Reusable collapsible section component.

use leptos::*;

/// Collapsible section with header and expandable content.
#[component]
pub fn CollapsibleSection(
    /// Section title displayed in header
    title: &'static str,
    /// Expanded state signal
    expanded: RwSignal<bool>,
    /// Child content
    children: Children,
) -> impl IntoView {
    let children = store_value(children());

    view! {
        <div class="border border-white/10 rounded-lg overflow-hidden">
            <button
                class="w-full flex items-center justify-between px-3 py-2 bg-white/5 \
                       hover:bg-white/10 transition-colors text-white text-sm"
                on:click=move |_| expanded.update(|v| *v = !*v)
            >
                <span>{title}</span>
                {move || if expanded.get() {
                    view! { <ChevronDownIcon /> }.into_view()
                } else {
                    view! { <ChevronRightIcon /> }.into_view()
                }}
            </button>

            <Show when=move || expanded.get()>
                <div class="p-3 space-y-3">
                    {children.get_value()}
                </div>
            </Show>
        </div>
    }
}

#[component]
fn ChevronDownIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m6 9 6 6 6-6"/>
        </svg>
    }
}

#[component]
fn ChevronRightIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m9 18 6-6-6-6"/>
        </svg>
    }
}
