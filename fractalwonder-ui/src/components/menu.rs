//! Reusable menu item components for consistent UI across menus.

use leptos::*;

/// A simple horizontal separator line for menus.
#[component]
pub fn MenuSeparator() -> impl IntoView {
    view! {
        <div class="my-1 mx-2 border-t border-gray-700" />
    }
}

/// Pencil/edit icon for menu items.
#[component]
fn PencilIcon() -> impl IntoView {
    view! {
        <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/>
            <path d="m15 5 4 4"/>
        </svg>
    }
}

/// Drag handle icon (6-dot 2x3 grip pattern) for reorderable items.
#[component]
fn DragHandleIcon() -> impl IntoView {
    view! {
        <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="currentColor"
        >
            // 2x3 grid of circles (grip pattern)
            <circle cx="9" cy="6" r="2"/>
            <circle cx="15" cy="6" r="2"/>
            <circle cx="9" cy="12" r="2"/>
            <circle cx="15" cy="12" r="2"/>
            <circle cx="9" cy="18" r="2"/>
            <circle cx="15" cy="18" r="2"/>
        </svg>
    }
}

/// A menu item with selection/enabled indicator, label, and optional shortcut.
/// Optionally shows an edit (pencil) icon and drag handle on hover.
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
    /// Optional edit handler - when provided, shows pencil icon on hover
    #[prop(optional)]
    on_edit: Option<Callback<()>>,
    /// Tooltip for edit button (default: "Edit")
    #[prop(optional, into)]
    edit_tooltip: Option<String>,
    /// Item ID for drag-and-drop (enables drag handle when provided)
    #[prop(optional, into)]
    item_id: Option<String>,
    /// Called when this item starts being dragged (receives item_id)
    #[prop(optional)]
    on_drag_start: Option<Callback<String>>,
    /// Called when dragging over this item (receives item_id)
    #[prop(optional)]
    on_drag_over: Option<Callback<String>>,
    /// Called when an item is dropped on this item (receives item_id)
    #[prop(optional)]
    on_drop: Option<Callback<String>>,
) -> impl IntoView {
    let has_edit = on_edit.is_some();
    let has_drag = item_id.is_some() && on_drag_start.is_some();
    let tooltip = edit_tooltip.unwrap_or_else(|| "Edit".to_string());

    // Track whether we're currently dragging (for cursor style)
    let (is_dragging, set_is_dragging) = create_signal(false);
    // Track whether we're being dragged over (for drop indicator)
    let (is_drag_over, set_is_drag_over) = create_signal(false);

    // Clone item_id for each closure that needs it
    let item_id_for_drag = item_id.clone();
    let item_id_for_dragover = item_id.clone();
    let item_id_for_drop = item_id.clone();

    view! {
        <div
            class=move || {
                let base = "group w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white rounded transition-colors flex items-center justify-between";
                if is_drag_over.get() {
                    format!("{} border-t-2 border-blue-400", base)
                } else {
                    base.to_string()
                }
            }
            on:click=move |_| on_click.call(())
            on:dragover=move |e| {
                if on_drag_over.is_some() {
                    e.prevent_default();
                    set_is_drag_over.set(true);
                    if let Some(ref id) = item_id_for_dragover {
                        if let Some(cb) = on_drag_over {
                            cb.call(id.clone());
                        }
                    }
                }
            }
            on:dragleave=move |_| {
                set_is_drag_over.set(false);
            }
            on:drop=move |e| {
                e.prevent_default();
                set_is_drag_over.set(false);
                if let Some(ref id) = item_id_for_drop {
                    if let Some(cb) = on_drop {
                        cb.call(id.clone());
                    }
                }
            }
        >
            <span class="flex items-center gap-2">
                <span class="w-4 text-center">
                    {move || if active.get() { "✓" } else { " " }}
                </span>
                {label.clone()}
            </span>
            <span class="flex items-center gap-2">
                {shortcut.map(|s| view! { <span class="text-xs text-gray-500">{s}</span> })}
                {has_edit.then(|| {
                    let on_edit = on_edit.unwrap();
                    let tooltip = tooltip.clone();
                    view! {
                        <span
                            class="opacity-0 group-hover:opacity-50 hover:!opacity-100 active:!opacity-30 transition-opacity cursor-pointer"
                            on:click=move |e| {
                                e.stop_propagation();
                                on_edit.call(());
                            }
                            title=tooltip
                        >
                            <PencilIcon />
                        </span>
                    }
                })}
                {has_drag.then(|| {
                    let on_drag_start = on_drag_start.unwrap();
                    let item_id = item_id_for_drag.clone().unwrap();
                    view! {
                        <span
                            class=move || {
                                let base = "opacity-0 group-hover:opacity-50 hover:!opacity-100 transition-opacity";
                                if is_dragging.get() {
                                    format!("{} cursor-grabbing", base)
                                } else {
                                    format!("{} cursor-grab", base)
                                }
                            }
                            draggable="true"
                            on:dragstart=move |e| {
                                e.stop_propagation();
                                set_is_dragging.set(true);
                                // Set drag data
                                if let Some(dt) = e.data_transfer() {
                                    let _ = dt.set_data("text/plain", &item_id);
                                }
                                on_drag_start.call(item_id.clone());
                            }
                            on:dragend=move |_| {
                                set_is_dragging.set(false);
                            }
                            title="Drag to reorder"
                        >
                            <DragHandleIcon />
                        </span>
                    }
                })}
            </span>
        </div>
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

/// A dropdown menu with toggle button and dropdown content.
/// Provides consistent styling and behavior for all menus.
/// Either `label` or `icon` should be provided; icon takes precedence.
#[component]
pub fn Menu(
    /// Menu open state
    is_open: ReadSignal<bool>,
    /// Set menu open state
    set_is_open: WriteSignal<bool>,
    /// Button label (displayed with dropdown chevron)
    #[prop(optional, into)]
    label: Option<String>,
    /// Optional icon to display instead of label
    #[prop(optional, into)]
    icon: Option<ViewFn>,
    /// Dropdown content
    children: ChildrenFn,
) -> impl IntoView {
    let button_class = if icon.is_some() {
        "text-white hover:text-gray-200 hover:bg-white/10 rounded-full p-2 transition-colors"
    } else {
        "text-white hover:text-gray-200 hover:bg-white/10 rounded-lg px-3 py-2 transition-colors flex items-center gap-2"
    };

    view! {
        <div class="relative">
            <button
                class=button_class
                on:click=move |e| {
                    e.stop_propagation();
                    set_is_open.update(|v| *v = !*v);
                }
            >
                {match (icon, label) {
                    (Some(icon_fn), _) => icon_fn.run().into_view(),
                    (None, Some(text)) => view! {
                        <span class="text-sm">{text}</span>
                        <span class="text-xs opacity-70">"▾"</span>
                    }.into_view(),
                    (None, None) => view! {
                        <span class="text-sm">"Menu"</span>
                        <span class="text-xs opacity-70">"▾"</span>
                    }.into_view(),
                }}
            </button>

            <Show when=move || is_open.get()>
                <div
                    class="absolute bottom-full mb-2 left-0 min-w-48 bg-black/70 backdrop-blur-sm border border-gray-800 rounded-lg overflow-hidden"
                    on:click=|e| e.stop_propagation()
                >
                    {children()}
                </div>
            </Show>
        </div>
    }
}
