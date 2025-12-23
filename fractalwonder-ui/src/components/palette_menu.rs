use crate::components::MenuItem;
use leptos::*;

#[component]
pub fn PaletteMenu<F>(
    /// Menu open state
    is_open: ReadSignal<bool>,
    /// Set menu open state
    set_is_open: WriteSignal<bool>,
    /// Button label
    label: String,
    /// Options to display (id, display_name)
    options: Signal<Vec<(String, String)>>,
    /// Currently selected option ID
    selected_id: Signal<String>,
    /// Callback when an option is selected
    on_select: F,
) -> impl IntoView
where
    F: Fn(String) + 'static + Copy,
{
    view! {
        <div class="relative">
            <button
                class="text-white hover:text-gray-200 hover:bg-white/10 rounded-lg px-3 py-2 transition-colors flex items-center gap-2"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
            >
                <span class="text-sm">{label.clone()}</span>
                <span class="text-xs opacity-70">"▾"</span>
            </button>

            {move || is_open.get().then(|| view! {
                <div class="absolute bottom-full mb-2 left-0 min-w-40 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg overflow-hidden">
                    <For
                        each=move || options.get()
                        key=|(id, _)| id.clone()
                        children=move |(id, name)| {
                            let id_for_selected = id.clone();
                            let id_for_click = id.clone();
                            let is_selected = Signal::derive(move || selected_id.get() == id_for_selected);
                            view! {
                                <MenuItem
                                    active=is_selected
                                    label=name
                                    on_click=Callback::new(move |_| {
                                        on_select(id_for_click.clone());
                                        set_is_open.set(false);
                                    })
                                />
                            }
                        }
                    />
                </div>
            })}
        </div>
    }
}
