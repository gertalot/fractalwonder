//! Interactive curve editor for transfer and falloff curves.

use crate::rendering::colorizers::{Curve, CurvePoint};
use crate::rendering::get_2d_context;
use leptos::*;
use web_sys::HtmlCanvasElement;

/// Interactive curve editor component.
#[component]
pub fn CurveEditor(
    /// The curve to edit (None when editor closed)
    curve: Signal<Option<Curve>>,
    /// Called when curve changes
    on_change: Callback<Curve>,
    /// Canvas size in logical pixels
    #[prop(default = 320)]
    size: u32,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();
    let hover_index = create_rw_signal(None::<usize>);
    let drag_index = create_rw_signal(None::<usize>);
    let is_dragging = create_rw_signal(false);

    // Draw curve when it changes or hover state changes
    create_effect(move |_| {
        let Some(crv) = curve.get() else { return };
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let _ = hover_index.get(); // Track hover for redraw

        draw_curve(&canvas, &crv, size, hover_index.get());
    });

    // Document-level mouse handlers for drag
    create_effect(move |_| {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let window = web_sys::window().expect("window");
        let document = window.document().expect("document");

        let size_copy = size;
        let canvas_ref_copy = canvas_ref;

        let mousemove_closure =
            Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                if !is_dragging.get() {
                    return;
                }
                let Some(idx) = drag_index.get() else { return };
                let Some(canvas) = canvas_ref_copy.get() else {
                    return;
                };
                let Some(mut crv) = curve.get() else { return };

                let (canvas_x, canvas_y) = mouse_to_canvas(&e, &canvas, size_copy);
                let x = canvas_x / size_copy as f64;
                let y = 1.0 - (canvas_y / size_copy as f64); // Invert Y

                let (x, y) = clamp_point(x, y, idx, crv.points.len());

                let target_point = CurvePoint { x, y };
                if idx < crv.points.len() {
                    crv.points[idx] = target_point.clone();
                    crv.points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
                    // Find new index of the point we just moved (it may have shifted due to sort)
                    if let Some(new_idx) = crv
                        .points
                        .iter()
                        .position(|p| p.x == target_point.x && p.y == target_point.y)
                    {
                        drag_index.set(Some(new_idx));
                    }
                    on_change.call(crv);
                }
            });

        let mouseup_closure =
            Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
                is_dragging.set(false);
                drag_index.set(None);
            });

        let _ = document.add_event_listener_with_callback(
            "mousemove",
            mousemove_closure.as_ref().unchecked_ref(),
        );
        let _ = document
            .add_event_listener_with_callback("mouseup", mouseup_closure.as_ref().unchecked_ref());

        mousemove_closure.forget();
        mouseup_closure.forget();
    });

    view! {
        <Show when=move || curve.get().is_some()>
            <div class="bg-white/5 border border-white/10 rounded-lg p-4 space-y-2">
                <div class="text-white/50 text-xs mb-2">"Transfer Curve"</div>
                <canvas
                    node_ref=canvas_ref
                    width=size
                    height=size
                    class="cursor-crosshair rounded"
                    style="width: 100%; height: auto;"
                    on:mousemove=move |e| {
                        let Some(canvas) = canvas_ref.get() else { return };
                        let Some(crv) = curve.get() else { return };
                        let (x, y) = mouse_to_canvas(&e, &canvas, size);
                        hover_index.set(find_point_at(&crv, x, y, size as f64));
                    }
                    on:mouseleave=move |_| {
                        hover_index.set(None);
                    }
                    on:mousedown=move |e| {
                        let Some(canvas) = canvas_ref.get() else { return };
                        let Some(crv) = curve.get() else { return };
                        let (canvas_x, canvas_y) = mouse_to_canvas(&e, &canvas, size);

                        if let Some(idx) = find_point_at(&crv, canvas_x, canvas_y, size as f64) {
                            // Start dragging existing point
                            e.prevent_default();
                            is_dragging.set(true);
                            drag_index.set(Some(idx));
                        } else {
                            // Add new point at click position
                            let x = canvas_x / size as f64;
                            let y = 1.0 - (canvas_y / size as f64);
                            let mut new_curve = crv.clone();
                            new_curve.points.push(CurvePoint { x, y });
                            new_curve.points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
                            on_change.call(new_curve);
                        }
                    }
                    on:dblclick=move |e| {
                        let Some(canvas) = canvas_ref.get() else { return };
                        let Some(crv) = curve.get() else { return };
                        let (canvas_x, canvas_y) = mouse_to_canvas(&e, &canvas, size);

                        if let Some(idx) = find_point_at(&crv, canvas_x, canvas_y, size as f64) {
                            // Don't delete if only 2 points remain
                            if crv.points.len() <= 2 {
                                return;
                            }
                            // Don't delete first or last point
                            if idx == 0 || idx == crv.points.len() - 1 {
                                return;
                            }
                            let mut new_curve = crv.clone();
                            new_curve.points.remove(idx);
                            on_change.call(new_curve);
                        }
                    }
                />
                <div class="text-white/50 text-xs">
                    "Click to add points · Drag to move · Double-click to remove"
                </div>
            </div>
        </Show>
    }
}

/// Draw the curve editor canvas.
fn draw_curve(canvas: &HtmlCanvasElement, curve: &Curve, size: u32, hover_index: Option<usize>) {
    let Ok(ctx) = get_2d_context(canvas) else {
        return;
    };
    let size_f = size as f64;

    // Clear canvas
    ctx.clear_rect(0.0, 0.0, size_f, size_f);

    // Background
    ctx.set_fill_style_str("rgba(255, 255, 255, 0.05)");
    ctx.fill_rect(0.0, 0.0, size_f, size_f);

    // Grid (4x4)
    ctx.set_stroke_style_str("rgba(255, 255, 255, 0.1)");
    ctx.set_line_width(1.0);
    for i in 0..=4 {
        let pos = (i as f64 / 4.0) * size_f;
        ctx.begin_path();
        ctx.move_to(pos, 0.0);
        ctx.line_to(pos, size_f);
        ctx.stroke();
        ctx.begin_path();
        ctx.move_to(0.0, pos);
        ctx.line_to(size_f, pos);
        ctx.stroke();
    }

    // Diagonal reference line (dashed)
    ctx.set_stroke_style_str("rgba(255, 255, 255, 0.2)");
    ctx.set_line_dash(&js_sys::Array::of2(&5.0.into(), &5.0.into()))
        .ok();
    ctx.begin_path();
    ctx.move_to(0.0, size_f);
    ctx.line_to(size_f, 0.0);
    ctx.stroke();
    ctx.set_line_dash(&js_sys::Array::new()).ok();

    // Draw actual cubic spline curve (100 sample points)
    ctx.set_stroke_style_str("rgba(255, 255, 255, 0.8)");
    ctx.set_line_width(2.0);
    ctx.begin_path();
    for i in 0..=100 {
        let x = i as f64 / 100.0;
        let y = curve.evaluate(x);
        let canvas_x = x * size_f;
        let canvas_y = (1.0 - y) * size_f; // Invert Y
        if i == 0 {
            ctx.move_to(canvas_x, canvas_y);
        } else {
            ctx.line_to(canvas_x, canvas_y);
        }
    }
    ctx.stroke();

    // Draw control points
    for (i, point) in curve.points.iter().enumerate() {
        let canvas_x = point.x * size_f;
        let canvas_y = (1.0 - point.y) * size_f;
        let radius = if hover_index == Some(i) { 6.0 } else { 5.0 };

        ctx.set_fill_style_str(if hover_index == Some(i) {
            "rgba(255, 255, 255, 1.0)"
        } else {
            "rgba(255, 255, 255, 0.9)"
        });
        ctx.begin_path();
        ctx.arc(canvas_x, canvas_y, radius, 0.0, std::f64::consts::TAU)
            .ok();
        ctx.fill();

        ctx.set_stroke_style_str("rgba(0, 0, 0, 0.5)");
        ctx.set_line_width(2.0);
        ctx.stroke();
    }
}

/// Find the index of a control point near the given canvas coordinates.
/// Returns None if no point is within the hit radius (10 pixels).
fn find_point_at(curve: &Curve, canvas_x: f64, canvas_y: f64, size: f64) -> Option<usize> {
    const HIT_RADIUS: f64 = 10.0;

    for (i, point) in curve.points.iter().enumerate() {
        let px = point.x * size;
        let py = (1.0 - point.y) * size;
        let dist = ((canvas_x - px).powi(2) + (canvas_y - py).powi(2)).sqrt();
        if dist < HIT_RADIUS {
            return Some(i);
        }
    }
    None
}

/// Convert mouse event to canvas-relative coordinates.
fn mouse_to_canvas(e: &web_sys::MouseEvent, canvas: &HtmlCanvasElement, size: u32) -> (f64, f64) {
    let rect = canvas.get_bounding_client_rect();
    let scale_x = size as f64 / rect.width();
    let scale_y = size as f64 / rect.height();
    let x = (e.client_x() as f64 - rect.left()) * scale_x;
    let y = (e.client_y() as f64 - rect.top()) * scale_y;
    (x.clamp(0.0, size as f64), y.clamp(0.0, size as f64))
}

/// Clamp a curve value to valid range.
fn clamp_point(x: f64, y: f64, index: usize, point_count: usize) -> (f64, f64) {
    let x = if index == 0 {
        0.0 // First point locked to x=0
    } else if index == point_count - 1 {
        1.0 // Last point locked to x=1
    } else {
        x.clamp(0.0, 1.0)
    };
    (x, y.clamp(0.0, 1.0))
}
