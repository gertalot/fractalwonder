//! Interactive curve editor for transfer and falloff curves.

use crate::rendering::colorizers::Curve;
use crate::rendering::get_2d_context;
use leptos::*;
use web_sys::HtmlCanvasElement;

/// Interactive curve editor component.
#[component]
pub fn CurveEditor(
    /// The curve to edit (None when editor closed)
    curve: Signal<Option<Curve>>,
    /// Called when curve changes
    #[allow(unused)]
    on_change: Callback<Curve>,
    /// Canvas size in logical pixels
    #[prop(default = 320)]
    size: u32,
) -> impl IntoView {
    let canvas_ref = create_node_ref::<leptos::html::Canvas>();
    let hover_index = create_rw_signal(None::<usize>);

    // Draw curve when it changes or hover state changes
    create_effect(move |_| {
        let Some(crv) = curve.get() else { return };
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let _ = hover_index.get(); // Track hover for redraw

        draw_curve(&canvas, &crv, size, hover_index.get());
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
