// fractalwonder-ui/src/components/fullscreen_button.rs
use crate::hooks::use_fullscreen;
use leptos::*;

#[component]
fn MaximizeIcon() -> impl IntoView {
    view! {
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"/>
        </svg>
    }
}

#[component]
fn MinimizeIcon() -> impl IntoView {
    view! {
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M8 3v3a2 2 0 0 1-2 2H3m18 0h-3a2 2 0 0 1-2-2V3m0 18v-3a2 2 0 0 1 2-2h3M3 16h3a2 2 0 0 1 2 2v3"/>
        </svg>
    }
}

#[component]
pub fn FullscreenButton() -> impl IntoView {
    let (is_fullscreen, toggle) = use_fullscreen();

    view! {
        <button
            class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
            on:click=move |_| toggle()
        >
            {move || if is_fullscreen.get() {
                view! { <MinimizeIcon /> }.into_view()
            } else {
                view! { <MaximizeIcon /> }.into_view()
            }}
        </button>
    }
}
