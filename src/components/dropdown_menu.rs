use leptos::*;

#[component]
pub fn DropdownMenu<F>(
    label: String,
    options: Signal<Vec<(String, String)>>, // (id, display_name)
    selected_id: Signal<String>,
    on_select: F,
) -> impl IntoView
where
    F: Fn(String) + 'static,
{
    let (is_open, set_is_open) = create_signal(false);

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
                            let is_selected = move || selected_id.get() == id;
                            let id_clone = id.clone();
                            view! {
                                <button
                                    class=move || format!(
                                        "w-full text-left px-4 py-2 text-sm transition-colors {}",
                                        if is_selected() {
                                            "bg-white/20 text-white"
                                        } else {
                                            "text-gray-300 hover:bg-white/10 hover:text-white"
                                        }
                                    )
                                    on:click=move |_| {
                                        on_select(id_clone.clone());
                                        set_is_open.set(false);
                                    }
                                >
                                    {name}
                                </button>
                            }
                        }
                    />
                </div>
            })}
        </div>
    }
}
