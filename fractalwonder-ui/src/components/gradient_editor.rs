//! Interactive gradient editor with color stops, midpoints, and zoom.

use crate::rendering::colorizers::{hex_to_rgb, rgb_to_hex, ColorStop, Gradient};
use crate::rendering::get_2d_context;
use leptos::*;
use wasm_bindgen::Clamped;
use web_sys::{HtmlCanvasElement, ImageData};

/// Interactive gradient editor component.
#[component]
pub fn GradientEditor(
    /// The gradient to edit (None when editor closed)
    gradient: Signal<Option<Gradient>>,
    /// Called when gradient changes (on mouse release)
    on_change: Callback<Gradient>,
) -> impl IntoView {
    // Internal state
    let selected_stop = create_rw_signal(None::<usize>);
    let zoom = create_rw_signal(1.0_f64);
    let is_dragging = create_rw_signal(false);
    let drag_index = create_rw_signal(None::<usize>);

    // Canvas ref
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();

    // Container ref for drag calculations
    let container_ref = create_node_ref::<leptos::html::Div>();

    // Handle drag start on a stop
    let start_drag = move |index: usize, e: web_sys::MouseEvent| {
        e.prevent_default();
        is_dragging.set(true);
        drag_index.set(Some(index));
        selected_stop.set(Some(index));
    };

    // Handle mouse move during drag
    let handle_mouse_move = move |e: web_sys::MouseEvent| {
        if !is_dragging.get() {
            return;
        }
        let Some(index) = drag_index.get() else {
            return;
        };
        let Some(container) = container_ref.get() else {
            return;
        };
        let Some(mut grad) = gradient.get() else {
            return;
        };

        let rect = container.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();
        let width = rect.width();
        let position = (x / width).clamp(0.0, 1.0);

        // Update stop position
        if index < grad.stops.len() {
            grad.stops[index].position = position;
            // Call on_change immediately for visual feedback during drag
            on_change.call(grad.clone());
        }
    };

    // Handle drag end
    let end_drag = move |_: web_sys::MouseEvent| {
        if is_dragging.get() {
            is_dragging.set(false);
            if let Some(grad) = gradient.get() {
                // Sort stops by position and call on_change
                let mut sorted = grad.clone();
                sorted
                    .stops
                    .sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
                on_change.call(sorted);
            }
        }
        drag_index.set(None);
    };

    // Handle click on gradient bar to add a stop
    let handle_bar_click = move |e: web_sys::MouseEvent| {
        let Some(container) = container_ref.get() else {
            return;
        };
        let Some(mut grad) = gradient.get() else {
            return;
        };

        let rect = container.get_bounding_client_rect();
        let x = e.client_x() as f64 - rect.left();
        let width = rect.width();
        let position = (x / width).clamp(0.0, 1.0);

        // Sample color from gradient at this position
        let lut = grad.to_preview_lut(1000);
        let lut_index = ((position * 999.0) as usize).min(999);
        let color = lut[lut_index];

        // Add new stop
        grad.stops.push(ColorStop { position, color });
        grad.stops
            .sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());

        // Update midpoints array to match new stop count
        let new_midpoint_count = grad.stops.len().saturating_sub(1);
        grad.midpoints.resize(new_midpoint_count, 0.5);

        // Find the index of the new stop and select it
        let new_index = grad.stops.iter().position(|s| s.position == position);
        selected_stop.set(new_index);

        on_change.call(grad);
    };

    // Document-level mouse handlers for drag
    create_effect(move |_| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let window = web_sys::window().expect("window");
        let document = window.document().expect("document");

        let mousemove_closure = Closure::<dyn Fn(web_sys::MouseEvent)>::new(handle_mouse_move);
        let mouseup_closure = Closure::<dyn Fn(web_sys::MouseEvent)>::new(end_drag);

        let _ = document.add_event_listener_with_callback(
            "mousemove",
            mousemove_closure.as_ref().unchecked_ref(),
        );
        let _ = document
            .add_event_listener_with_callback("mouseup", mouseup_closure.as_ref().unchecked_ref());

        // Leak closures (they live for app lifetime)
        mousemove_closure.forget();
        mouseup_closure.forget();
    });

    // Draw gradient when it changes
    create_effect(move |_| {
        let Some(grad) = gradient.get() else { return };
        let Some(canvas) = canvas_ref.get() else {
            return;
        };

        let canvas_el: &HtmlCanvasElement = &canvas;
        let width = canvas_el.width() as usize;
        let height = canvas_el.height() as usize;

        if width == 0 || height == 0 {
            return;
        }

        // Generate OKLAB-interpolated colors
        let lut = grad.to_preview_lut(width);

        // Convert to RGBA pixels (repeat each column for full height)
        let mut pixels = vec![0u8; width * height * 4];
        for (x, &[r, g, b]) in lut.iter().enumerate() {
            for y in 0..height {
                let idx = (y * width + x) * 4;
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                pixels[idx + 3] = 255;
            }
        }

        // Draw to canvas
        if let Ok(ctx) = get_2d_context(canvas_el) {
            if let Ok(image_data) = ImageData::new_with_u8_clamped_array_and_sh(
                Clamped(&pixels),
                width as u32,
                height as u32,
            ) {
                let _ = ctx.put_image_data(&image_data, 0.0, 0.0);
            }
        }
    });

    view! {
        <Show when=move || gradient.get().is_some()>
            <div class="space-y-2">
                // Zoom controls
                <div class="flex items-center justify-between px-1">
                    <div class="text-white/50 text-xs">
                        {move || if zoom.get() > 1.0 {
                            format!("Zoom: {:.1}x", zoom.get())
                        } else {
                            String::new()
                        }}
                    </div>
                    <div class="flex items-center gap-1">
                        <button
                            class="p-1 rounded hover:bg-white/10 text-white disabled:opacity-30 \
                                   disabled:cursor-not-allowed transition-colors"
                            prop:disabled=move || zoom.get() <= 1.0
                            on:click=move |_| zoom.update(|z| *z = (*z / 1.2).max(1.0))
                        >
                            <ZoomOutIcon />
                        </button>
                        <button
                            class="p-1 rounded hover:bg-white/10 text-white disabled:opacity-30 \
                                   disabled:cursor-not-allowed transition-colors"
                            prop:disabled=move || zoom.get() >= 10.0
                            on:click=move |_| zoom.update(|z| *z = (*z * 1.2).min(10.0))
                        >
                            <ZoomInIcon />
                        </button>
                    </div>
                </div>

                // Scrollable gradient container
                <div
                    class="overflow-x-auto overflow-y-visible"
                    style="max-width: 100%;"
                >
                    <div
                        node_ref=container_ref
                        class="relative"
                        style=move || format!("width: {}%;", zoom.get() * 100.0)
                    >
                        // Color stops (squares above gradient bar)
                        <div class="relative h-6 mb-1">
                            <For
                                each=move || {
                                    gradient
                                        .get()
                                        .map(|g| {
                                            g.stops
                                                .iter()
                                                .enumerate()
                                                .map(|(i, s)| (i, s.position, s.color))
                                                .collect::<Vec<_>>()
                                        })
                                        .unwrap_or_default()
                                }
                                key=|(i, _, _)| *i
                                children=move |(index, position, color)| {
                                    let is_selected = move || selected_stop.get() == Some(index);
                                    let color_hex = format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2]);

                                    view! {
                                        <div
                                            class="absolute top-0 w-3 h-3 cursor-ew-resize transition-shadow"
                                            style=move || format!(
                                                "left: {}%; transform: translateX(-50%); \
                                                 background-color: {}; \
                                                 border: 1px solid rgba(255, 255, 255, 0.3); \
                                                 box-shadow: {};",
                                                position * 100.0,
                                                color_hex,
                                                if is_selected() {
                                                    "0 0 6px 2px rgba(255, 255, 255, 0.7)"
                                                } else {
                                                    "none"
                                                }
                                            )
                                            on:mousedown=move |e| start_drag(index, e)
                                            on:click=move |e| {
                                                e.stop_propagation();
                                                selected_stop.set(Some(index));
                                            }
                                            on:dblclick=move |e| {
                                                e.stop_propagation();
                                                let Some(mut grad) = gradient.get() else {
                                                    return;
                                                };

                                                // Silently ignore if only 2 stops remain
                                                if grad.stops.len() <= 2 {
                                                    return;
                                                }

                                                // Remove the stop
                                                if index < grad.stops.len() {
                                                    grad.stops.remove(index);
                                                    // Update midpoints
                                                    let new_midpoint_count = grad.stops.len().saturating_sub(1);
                                                    grad.midpoints.resize(new_midpoint_count, 0.5);

                                                    selected_stop.set(None);
                                                    on_change.call(grad);
                                                }
                                            }
                                        />
                                    }
                                }
                            />
                        </div>

                        // Gradient bar (canvas)
                        <canvas
                            node_ref=canvas_ref
                            class="w-full rounded border border-white/20 cursor-crosshair"
                            width="320"
                            height="32"
                            style="height: 32px;"
                            on:click=handle_bar_click
                        />
                    </div>
                </div>

                // Color picker panel (shown when stop selected)
                <Show when=move || selected_stop.get().is_some()>
                    {move || {
                        let index = selected_stop.get().unwrap();
                        let grad = gradient.get();
                        let stop = grad.as_ref().and_then(|g| g.stops.get(index));

                        if let Some(stop) = stop {
                            let color = stop.color;
                            let hex = rgb_to_hex(color);

                            view! {
                                <div class="bg-white/5 border border-white/10 rounded p-2 space-y-2">
                                    <div class="flex items-center gap-2">
                                        // Native color picker
                                        <input
                                            type="color"
                                            value=hex.clone()
                                            class="w-12 h-8 rounded cursor-pointer bg-transparent"
                                            on:change=move |e| {
                                                let value = event_target_value(&e);
                                                if let Some(rgb) = hex_to_rgb(&value) {
                                                    let Some(mut grad) = gradient.get() else { return };
                                                    if let Some(stop) = grad.stops.get_mut(index) {
                                                        stop.color = rgb;
                                                        on_change.call(grad);
                                                    }
                                                }
                                            }
                                        />
                                        // Hex input
                                        <input
                                            type="text"
                                            value=hex
                                            class="flex-1 bg-white/5 border border-white/20 rounded px-2 py-1 \
                                                   text-white text-xs outline-none focus:border-white/40"
                                            on:change=move |e| {
                                                let value = event_target_value(&e);
                                                if let Some(rgb) = hex_to_rgb(&value) {
                                                    let Some(mut grad) = gradient.get() else { return };
                                                    if let Some(stop) = grad.stops.get_mut(index) {
                                                        stop.color = rgb;
                                                        on_change.call(grad);
                                                    }
                                                }
                                            }
                                        />
                                    </div>
                                </div>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </Show>
            </div>
        </Show>
    }
}

// Zoom icons
#[component]
fn ZoomOutIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"/>
            <path d="m21 21-4.3-4.3"/>
            <path d="M8 11h6"/>
        </svg>
    }
}

#[component]
fn ZoomInIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"/>
            <path d="m21 21-4.3-4.3"/>
            <path d="M11 8v6"/>
            <path d="M8 11h6"/>
        </svg>
    }
}
