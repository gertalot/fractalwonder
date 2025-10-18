use leptos::*;
use leptos_use::use_raf_fn;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

const INTERACTION_TIMEOUT_MS: i32 = 1500;
const ZOOM_SENSITIVITY: f64 = 0.0005;

/// Transformation result returned when user interaction ends (after 1.5s of inactivity)
///
/// Contains both discrete values and a pre-computed affine transformation matrix.
/// All coordinates are in screen pixel space.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformResult {
    /// Horizontal offset in pixels
    pub offset_x: f64,
    /// Vertical offset in pixels
    pub offset_y: f64,
    /// Cumulative zoom factor (1.0 = no zoom, 2.0 = 2x zoom, 0.5 = 0.5x zoom)
    pub zoom_factor: f64,
    /// 2D affine transformation matrix \[3x3\] encoding offset + zoom
    pub matrix: [[f64; 3]; 3],
}

/// Handle returned by the canvas interaction hook
///
/// Provides event handlers to attach to canvas element and reactive interaction state.
pub struct InteractionHandle {
    /// Reactive signal indicating whether user is currently interacting
    pub is_interacting: Signal<bool>,
    /// Event handler for pointerdown events
    pub on_pointer_down: Rc<dyn Fn(web_sys::PointerEvent)>,
    /// Event handler for pointermove events
    pub on_pointer_move: Rc<dyn Fn(web_sys::PointerEvent)>,
    /// Event handler for pointerup events
    pub on_pointer_up: Rc<dyn Fn(web_sys::PointerEvent)>,
    /// Event handler for wheel events (zoom)
    pub on_wheel: Rc<dyn Fn(web_sys::WheelEvent)>,
    /// Reset all interaction state
    pub reset: Rc<dyn Fn()>,
}

/// Builds a 2D affine transformation matrix from offset, zoom, and optional zoom center
fn build_transform_matrix(
    offset: (f64, f64),
    zoom: f64,
    zoom_center: Option<(f64, f64)>,
) -> [[f64; 3]; 3] {
    let mut matrix = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    // Apply scale (zoom)
    matrix[0][0] = zoom;
    matrix[1][1] = zoom;

    // Apply translation
    if let Some((cx, cy)) = zoom_center {
        // Translate to zoom center, scale, translate back
        matrix[0][2] = offset.0 + cx * (1.0 - zoom);
        matrix[1][2] = offset.1 + cy * (1.0 - zoom);
    } else {
        matrix[0][2] = offset.0;
        matrix[1][2] = offset.1;
    }

    matrix
}

fn capture_canvas_image_data(canvas: &HtmlCanvasElement) -> Result<ImageData, JsValue> {
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("Failed to get 2D context"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    context.get_image_data(0.0, 0.0, canvas.width() as f64, canvas.height() as f64)
}

fn render_preview(
    canvas: &HtmlCanvasElement,
    image_data: &ImageData,
    offset: (f64, f64),
    zoom: f64,
    zoom_center: Option<(f64, f64)>,
) -> Result<(), JsValue> {
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("Failed to get 2D context"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    // Clear canvas
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    // Apply transformation matrix
    let matrix = build_transform_matrix(offset, zoom, zoom_center);
    context.set_transform(
        matrix[0][0],
        matrix[1][0],
        matrix[0][1],
        matrix[1][1],
        matrix[0][2],
        matrix[1][2],
    )?;

    // Draw the transformed image
    context.put_image_data(image_data, 0.0, 0.0)?;

    // Reset transform
    context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)?;

    Ok(())
}

/// Generic canvas interaction hook providing real-time pan/zoom preview
///
/// Designed for canvases where full re-renders are expensive (seconds to hours).
/// Captures canvas ImageData on interaction start, provides real-time preview
/// using pixel transformations, and fires callback after 1.5s of inactivity.
///
/// # Example
///
/// ```rust,no_run
/// use leptos::*;
/// use fractalwonder::hooks::use_canvas_interaction::{use_canvas_interaction, TransformResult};
///
/// #[component]
/// pub fn MyCanvas() -> impl IntoView {
///     let canvas_ref = create_node_ref::<leptos::html::Canvas>();
///
///     let handle = use_canvas_interaction(
///         canvas_ref,
///         move |result: TransformResult| {
///             // Convert pixel transform to domain coordinates
///             // Trigger expensive full re-render
///         },
///     );
///
///     view! {
///         <canvas
///             node_ref=canvas_ref
///             on:pointerdown=move |ev| (handle.on_pointer_down)(ev)
///             on:pointermove=move |ev| (handle.on_pointer_move)(ev)
///             on:pointerup=move |ev| (handle.on_pointer_up)(ev)
///             on:wheel=move |ev| (handle.on_wheel)(ev)
///         />
///     }
/// }
/// ```
///
/// # Arguments
///
/// * `canvas_ref` - Leptos NodeRef to canvas element
/// * `on_interaction_end` - Callback fired when interaction ends (1.5s inactivity)
///
/// # Returns
///
/// `InteractionHandle` with event handlers and interaction state signal
pub fn use_canvas_interaction<F>(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    on_interaction_end: F,
) -> InteractionHandle
where
    F: Fn(TransformResult) + 'static,
{
    // Interaction state signals
    let is_dragging = create_rw_signal(false);
    let is_zooming = create_rw_signal(false);
    let is_interacting = create_memo(move |_| is_dragging.get() || is_zooming.get());

    // Stored state (non-reactive)
    let initial_image_data = store_value::<Option<ImageData>>(None);
    let drag_start = store_value::<Option<(f64, f64)>>(None);
    let accumulated_offset = store_value((0.0, 0.0));
    let accumulated_zoom = store_value(1.0);
    let zoom_center = store_value::<Option<(f64, f64)>>(None);
    let animation_frame_id = store_value::<Option<i32>>(None);
    let timeout_id = store_value::<Option<i32>>(None);

    // Reset function
    let reset = move || {
        is_dragging.set(false);
        is_zooming.set(false);
        initial_image_data.set_value(None);
        drag_start.set_value(None);
        accumulated_offset.set_value((0.0, 0.0));
        accumulated_zoom.set_value(1.0);
        zoom_center.set_value(None);
        animation_frame_id.set_value(None);
        timeout_id.set_value(None);
    };

    // Store canvas_ref for multiple closures
    let canvas_ref_stored = store_value(canvas_ref);

    // Animation loop for preview rendering
    use_raf_fn(move |_| {
        // Only render if we're interacting and have image data
        if !is_interacting.get() {
            return;
        }

        let canvas_ref = canvas_ref_stored.get_value();
        if let Some(canvas) = canvas_ref.get() {
            if let Some(image_data) = initial_image_data.get_value() {
                let offset = accumulated_offset.get_value();
                let zoom = accumulated_zoom.get_value();
                let center = zoom_center.get_value();

                let _ = render_preview(&canvas, &image_data, offset, zoom, center);
            }
        }
    });

    // Interaction start helper
    let start_interaction = move || {
        let canvas_ref = canvas_ref_stored.get_value();
        if let Some(canvas) = canvas_ref.get_untracked() {
            if let Ok(image_data) = capture_canvas_image_data(&canvas) {
                initial_image_data.set_value(Some(image_data));
                accumulated_offset.set_value((0.0, 0.0));
                accumulated_zoom.set_value(1.0);
                zoom_center.set_value(None);
            }
        }
    };

    // Stop interaction handler - builds TransformResult and fires callback
    let on_interaction_end = store_value(on_interaction_end);
    let stop_interaction = move || {
        // Don't stop if still dragging (use get_untracked since we're in a timeout callback)
        if is_dragging.get_untracked() {
            return;
        }

        is_zooming.set(false);

        // Build final result
        let offset = accumulated_offset.get_value();
        let zoom = accumulated_zoom.get_value();
        let center = zoom_center.get_value();
        let matrix = build_transform_matrix(offset, zoom, center);

        let result = TransformResult {
            offset_x: offset.0,
            offset_y: offset.1,
            zoom_factor: zoom,
            matrix,
        };

        // Clear state
        initial_image_data.set_value(None);
        accumulated_offset.set_value((0.0, 0.0));
        accumulated_zoom.set_value(1.0);
        zoom_center.set_value(None);

        // Fire callback
        on_interaction_end.with_value(|cb| cb(result));
    };

    // Restart timeout helper - uses manual web-sys timeout
    let stop_interaction_stored = store_value(stop_interaction);
    let restart_timeout = move || {
        // Clear existing timeout
        if let Some(id) = timeout_id.get_value() {
            web_sys::window().unwrap().clear_timeout_with_handle(id);
        }

        // Set new timeout
        let callback = Closure::once(move || {
            stop_interaction_stored.with_value(|f| f());
        });

        let id = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                INTERACTION_TIMEOUT_MS,
            )
            .unwrap();

        callback.forget();
        timeout_id.set_value(Some(id));
    };

    // Pointer down handler
    let on_pointer_down = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();

        start_interaction();
        is_dragging.set(true);
        drag_start.set_value(Some((ev.client_x() as f64, ev.client_y() as f64)));
    };

    // Pointer move handler
    let on_pointer_move = move |ev: web_sys::PointerEvent| {
        if !is_dragging.get_untracked() {
            return;
        }

        if let Some(start) = drag_start.get_value() {
            let current_x = ev.client_x() as f64;
            let current_y = ev.client_y() as f64;
            let offset = (current_x - start.0, current_y - start.1);
            accumulated_offset.set_value(offset);
        }
    };

    // Pointer up handler
    let restart_timeout_clone = store_value(restart_timeout);
    let on_pointer_up = move |_ev: web_sys::PointerEvent| {
        is_dragging.set(false);
        restart_timeout_clone.with_value(|f| f());
    };

    // Wheel handler for zoom
    let restart_timeout_clone2 = store_value(restart_timeout);
    let on_wheel = move |ev: web_sys::WheelEvent| {
        ev.prevent_default();

        // Start interaction if not already started (use get_untracked since we're in an event handler)
        if !is_dragging.get_untracked() && !is_zooming.get_untracked() {
            start_interaction();
        }

        is_zooming.set(true);

        // Calculate zoom factor from wheel delta
        let delta = ev.delta_y();
        let zoom_multiplier = (-delta * ZOOM_SENSITIVITY).exp();
        let current_zoom = accumulated_zoom.get_value();
        accumulated_zoom.set_value(current_zoom * zoom_multiplier);

        // Store zoom center (pointer position relative to canvas)
        let canvas_ref = canvas_ref_stored.get_value();
        if let Some(canvas) = canvas_ref.get_untracked() {
            let rect = canvas.get_bounding_client_rect();
            let x = ev.client_x() as f64 - rect.left();
            let y = ev.client_y() as f64 - rect.top();
            zoom_center.set_value(Some((x, y)));
        }

        // Restart timeout on every wheel event
        restart_timeout_clone2.with_value(|f| f());
    };

    InteractionHandle {
        is_interacting: Signal::derive(move || is_interacting.get()),
        on_pointer_down: Rc::new(on_pointer_down),
        on_pointer_move: Rc::new(on_pointer_move),
        on_pointer_up: Rc::new(on_pointer_up),
        on_wheel: Rc::new(on_wheel),
        reset: Rc::new(reset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let matrix = build_transform_matrix((0.0, 0.0), 1.0, None);
        assert_eq!(matrix, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0],]);
    }

    #[test]
    fn test_translation_matrix() {
        let matrix = build_transform_matrix((100.0, 50.0), 1.0, None);
        assert_eq!(
            matrix,
            [[1.0, 0.0, 100.0], [0.0, 1.0, 50.0], [0.0, 0.0, 1.0],]
        );
    }

    #[test]
    fn test_zoom_matrix_no_center() {
        let matrix = build_transform_matrix((0.0, 0.0), 2.0, None);
        assert_eq!(matrix, [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0],]);
    }

    #[test]
    fn test_zoom_matrix_with_center() {
        let matrix = build_transform_matrix((0.0, 0.0), 2.0, Some((100.0, 100.0)));
        // Zoom 2x centered at (100, 100)
        // Translation should be 100*(1-2) = -100 for both x and y
        assert_eq!(
            matrix,
            [[2.0, 0.0, -100.0], [0.0, 2.0, -100.0], [0.0, 0.0, 1.0],]
        );
    }

    #[test]
    fn test_combined_transform() {
        let matrix = build_transform_matrix((50.0, 30.0), 1.5, Some((200.0, 150.0)));
        // offset + center*(1-zoom)
        // x: 50 + 200*(1-1.5) = 50 + 200*(-0.5) = 50 - 100 = -50
        // y: 30 + 150*(1-1.5) = 30 + 150*(-0.5) = 30 - 75 = -45
        assert_eq!(
            matrix,
            [[1.5, 0.0, -50.0], [0.0, 1.5, -45.0], [0.0, 0.0, 1.0],]
        );
    }
}

#[cfg(test)]
mod browser_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_hook_creates_handle() {
        let canvas_ref = create_node_ref::<leptos::html::Canvas>();
        let callback_fired = create_rw_signal(false);

        let handle = use_canvas_interaction(canvas_ref, move |_result| {
            callback_fired.set(true);
        });

        assert!(!handle.is_interacting.get());
    }
}
