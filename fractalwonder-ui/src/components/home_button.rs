// fractalwonder-ui/src/components/home_button.rs
use leptos::*;

#[component]
fn HomeIcon() -> impl IntoView {
    view! {
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
            <polyline points="9 22 9 12 15 12 15 22"/>
        </svg>
    }
}

#[component]
pub fn HomeButton(on_click: Callback<()>) -> impl IntoView {
    view! {
        <button
            class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
            on:click=move |_| on_click.call(())
            title="Reset to home view"
        >
            <HomeIcon />
        </button>
    }
}
