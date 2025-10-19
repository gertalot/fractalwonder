use crate::rendering::renderer_info::RendererInfoData;
use crate::utils::fullscreen::use_fullscreen;
use leptos::*;

#[component]
fn InfoIcon() -> impl IntoView {
    view! {
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="16" x2="12" y2="12"/>
        <circle cx="12" cy="8" r="0.5" fill="currentColor"/>
      </svg>
    }
}

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
fn GithubIcon() -> impl IntoView {
    view! {
      <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
      </svg>
    }
}

#[component]
fn InfoButton(is_open: ReadSignal<bool>, set_is_open: WriteSignal<bool>) -> impl IntoView {
    view! {
      <div class="relative">
        <button
          class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
          on:click=move |_| set_is_open.set(!is_open.get())
        >
          <InfoIcon />
        </button>

        {move || is_open.get().then(|| view! {
          <div class="absolute bottom-full mb-3 left-0 w-80 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg p-4 text-white">
            <h3 class="font-medium mb-2">"Fractal Wonder"</h3>
            <p class="text-sm text-gray-300 mb-4">
              "Use mouse/touch to pan and zoom. Keyboard shortcuts: [ and ] to cycle color schemes."
            </p>
            <div class="flex items-center gap-2 text-sm text-gray-400">
              <a
                href="https://github.com/gertalot/fractalwonder"
                target="_blank"
                rel="noopener noreferrer"
                class="text-white hover:text-gray-200 transition-colors"
              >
                <GithubIcon />
              </a>
              <span>"Made by Gert"</span>
            </div>
          </div>
        })}
      </div>
    }
}

#[component]
fn HomeButton(on_click: impl Fn() + 'static) -> impl IntoView {
    view! {
      <button
        class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
        on:click=move |_| on_click()
      >
        <HomeIcon />
      </button>
    }
}

#[component]
fn FullscreenButton(on_click: impl Fn() + 'static) -> impl IntoView {
    let (is_fullscreen, _) = use_fullscreen();

    view! {
      <button
        class="text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
        on:click=move |_| on_click()
      >
        {move || if is_fullscreen.get() {
          view! { <MinimizeIcon /> }
        } else {
          view! { <MaximizeIcon /> }
        }}
      </button>
    }
}

#[component]
fn InfoDisplay(info: ReadSignal<RendererInfoData>) -> impl IntoView {
    view! {
      <div class="text-white text-sm">
        <p>
          {move || {
            let i = info.get();
            format!("Center: {}, zoom: {}", i.center_display, i.zoom_display)
          }}
          {move || {
            info.get().render_time_ms.map(|ms|
              format!(", render: {:.2}s", ms / 1000.0)
            ).unwrap_or_default()
          }}
        </p>
        <p>
          "Algorithm: "
          {move || info.get().name}
          {move || {
            info.get().custom_params.iter()
              .map(|(k, v)| format!(" | {}: {}", k, v))
              .collect::<Vec<_>>()
              .join("")
          }}
        </p>
      </div>
    }
}

#[component]
pub fn UI(
    info: ReadSignal<RendererInfoData>,
    is_visible: ReadSignal<bool>,
    set_is_hovering: WriteSignal<bool>,
    on_home_click: impl Fn() + 'static,
    on_fullscreen_click: impl Fn() + 'static,
) -> impl IntoView {
    let (is_popover_open, set_is_popover_open) = create_signal(false);

    // Keep UI visible when popover is open
    create_effect(move |_| {
        if is_popover_open.get() {
            set_is_hovering.set(true);
        }
    });

    let opacity_class = move || {
        if is_visible.get() {
            "opacity-100"
        } else {
            "opacity-0"
        }
    };

    view! {
      <div
        class=move || format!(
          "fixed inset-x-0 bottom-0 transition-opacity duration-300 {}",
          opacity_class()
        )
        on:mouseenter=move |_| set_is_hovering.set(true)
        on:mouseleave=move |_| set_is_hovering.set(false)
      >
        <div class="flex items-center justify-between px-4 py-3 bg-black/50 backdrop-blur-sm">
          // Left section: buttons
          <div class="flex items-center space-x-4">
            <InfoButton is_open=is_popover_open set_is_open=set_is_popover_open />
            <HomeButton on_click=on_home_click />
          </div>

          // Center section: info display
          <div class="flex-1 text-center">
            <InfoDisplay info=info />
          </div>

          // Right section: fullscreen
          <div>
            <FullscreenButton on_click=on_fullscreen_click />
          </div>
        </div>
      </div>
    }
}
