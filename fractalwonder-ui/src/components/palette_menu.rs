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
    /// Callback when palettes are reordered (receives: from_id, to_id)
    on_reorder: Option<Callback<(String, String)>>,
) -> impl IntoView
where
    F: Fn(String) + 'static + Copy,
{
    // Track which item is being dragged
    let (dragged_id, set_dragged_id) = create_signal(Option::<String>::None);

    view! {
        <Menu is_open=is_open set_is_open=set_is_open label=label>
            <For
                each=move || options.get()
                key=|(id, _)| id.clone()
                children=move |(id, name)| {
                    let id_for_selected = id.clone();
                    let id_for_click = id.clone();
                    let id_for_edit = id.clone();
                    let id_for_drag = id.clone();
                    let is_selected = Signal::derive(move || selected_id.get() == id_for_selected);

                    // Common props
                    let on_click_cb = Callback::new(move |_| {
                        on_select(id_for_click.clone());
                        set_is_open.set(false);
                    });
                    let on_edit_cb = Callback::new(move |_| {
                        on_edit.call(id_for_edit.clone());
                        set_is_open.set(false);
                    });

                    // Render with or without drag support
                    if let Some(on_reorder) = on_reorder {
                        let on_drag_start_cb = Callback::new(move |id: String| {
                            set_dragged_id.set(Some(id));
                        });
                        let on_drag_over_cb = Callback::new(move |_: String| {
                            // Visual feedback handled by MenuItem
                        });
                        let on_drop_cb = Callback::new(move |target_id: String| {
                            if let Some(from_id) = dragged_id.get_untracked() {
                                if from_id != target_id {
                                    on_reorder.call((from_id, target_id));
                                }
                            }
                            set_dragged_id.set(None);
                        });

                        view! {
                            <MenuItem
                                active=is_selected
                                label=name
                                on_click=on_click_cb
                                on_edit=on_edit_cb
                                edit_tooltip="Edit palette"
                                item_id=id_for_drag.clone()
                                on_drag_start=on_drag_start_cb
                                on_drag_over=on_drag_over_cb
                                on_drop=on_drop_cb
                            />
                        }.into_view()
                    } else {
                        view! {
                            <MenuItem
                                active=is_selected
                                label=name
                                on_click=on_click_cb
                                on_edit=on_edit_cb
                                edit_tooltip="Edit palette"
                            />
                        }.into_view()
                    }
                }
            />
        </Menu>
    }
}
