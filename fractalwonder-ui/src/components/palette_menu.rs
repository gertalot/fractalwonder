use crate::components::{Menu, MenuItem};
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
    /// Callback when edit icon is clicked for a palette (receives palette id)
    on_edit: Callback<String>,
) -> impl IntoView
where
    F: Fn(String) + 'static + Copy,
{
    view! {
        <Menu is_open=is_open set_is_open=set_is_open label=label>
            <For
                each=move || options.get()
                key=|(id, _)| id.clone()
                children=move |(id, name)| {
                    let id_for_selected = id.clone();
                    let id_for_click = id.clone();
                    let id_for_edit = id.clone();
                    let is_selected = Signal::derive(move || selected_id.get() == id_for_selected);
                    view! {
                        <MenuItem
                            active=is_selected
                            label=name
                            on_click=Callback::new(move |_| {
                                on_select(id_for_click.clone());
                                set_is_open.set(false);
                            })
                            on_edit=Callback::new(move |_| {
                                on_edit.call(id_for_edit.clone());
                                set_is_open.set(false);
                            })
                            edit_tooltip="Edit palette"
                        />
                    }
                }
            />
        </Menu>
    }
}
