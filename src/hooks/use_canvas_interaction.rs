use crate::rendering::transforms::{compose_affine_transformations, Mat3, Transform};
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
/// Offsets are **center-relative** for intuitive interpretation.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformResult {
    /// Horizontal offset in pixels relative to canvas center
    /// (0, 0) means no offset from center; positive = right, negative = left
    pub offset_x: f64,
    /// Vertical offset in pixels relative to canvas center
    /// (0, 0) means no offset from center; positive = down, negative = up
    pub offset_y: f64,
    /// Cumulative zoom factor (1.0 = no zoom, 2.0 = 2x zoom, 0.5 = 0.5x zoom)
    pub zoom_factor: f64,
    /// 2D affine transformation matrix \[3x3\] encoding offset + zoom in absolute coordinates
    /// (used internally for canvas rendering, not for external interpretation)
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
    /// Event handler for canvas resize events
    pub on_canvas_resize: Rc<dyn Fn(u32, u32)>,
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

    // Clear canvas to background
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    // Create a temporary canvas to hold the ImageData
    // We MUST use this approach (not put_image_data with offset) because:
    // - put_image_data discards pixels that go outside canvas bounds
    // - drawImage + transformation matrix preserves all pixels (browser clips naturally)
    // - This ensures dragging left then right shows the FULL original image
    let document = web_sys::window()
        .ok_or_else(|| JsValue::from_str("No window"))?
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;

    let temp_canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    temp_canvas.set_width(image_data.width());
    temp_canvas.set_height(image_data.height());

    let temp_context = temp_canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("Failed to get 2D context"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    // Put the FULL ImageData on temporary canvas
    temp_context.put_image_data(image_data, 0.0, 0.0)?;

    // Apply transformation matrix to main canvas
    let matrix = build_transform_matrix(offset, zoom, zoom_center);
    context.set_transform(
        matrix[0][0],
        matrix[1][0],
        matrix[0][1],
        matrix[1][1],
        matrix[0][2],
        matrix[1][2],
    )?;

    // Draw the transformed image from temporary canvas
    // The transformation ensures the FULL image is drawn with offset/zoom applied
    // Browser naturally clips what's outside canvas bounds (but pixels are preserved in ImageData)
    context.draw_image_with_html_canvas_element(&temp_canvas, 0.0, 0.0)?;

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
    let is_resizing = create_rw_signal(false);
    let is_interacting =
        create_memo(move |_| is_dragging.get() || is_zooming.get() || is_resizing.get());

    // Stored state (non-reactive)
    let initial_image_data = store_value::<Option<ImageData>>(None);
    let initial_canvas_size = store_value::<Option<(u32, u32)>>(None); // Canvas size when interaction started
    let drag_start = store_value::<Option<(f64, f64)>>(None);
    let base_offset = store_value((0.0, 0.0)); // Committed offset from all previous drags (for preview)
    let current_drag_offset = store_value((0.0, 0.0)); // Offset from current drag only (for preview)
    let accumulated_zoom = store_value(1.0); // Accumulated zoom (for preview)
    let zoom_center = store_value::<Option<(f64, f64)>>(None);
    let animation_frame_id = store_value::<Option<i32>>(None);
    let timeout_id = store_value::<Option<i32>>(None);

    // Transformation sequence - the source of truth for final result
    let transform_sequence = store_value::<Vec<Transform>>(Vec::new());

    // Reset function
    let reset = move || {
        is_dragging.set(false);
        is_zooming.set(false);
        is_resizing.set(false);
        initial_image_data.set_value(None);
        initial_canvas_size.set_value(None);
        drag_start.set_value(None);
        base_offset.set_value((0.0, 0.0));
        current_drag_offset.set_value((0.0, 0.0));
        accumulated_zoom.set_value(1.0);
        zoom_center.set_value(None);
        animation_frame_id.set_value(None);
        timeout_id.set_value(None);
        transform_sequence.set_value(Vec::new());
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
                // Check if canvas size changed during interaction
                let current_size = (canvas.width(), canvas.height());
                let size_offset = if let Some(initial_size) = initial_canvas_size.get_value() {
                    // Canvas resized - adjust offset to keep center of image centered
                    // If canvas grew, we need to shift image to stay centered
                    // If canvas shrunk, we need to shift image to stay centered
                    let width_change = (current_size.0 as f64 - initial_size.0 as f64) / 2.0;
                    let height_change = (current_size.1 as f64 - initial_size.1 as f64) / 2.0;
                    (width_change, height_change)
                } else {
                    (0.0, 0.0)
                };

                // Total offset = base (from previous drags + zoom adjustments) + current drag + resize adjustment
                let base = base_offset.get_value();
                let current = current_drag_offset.get_value();
                let total_offset = (
                    base.0 + current.0 + size_offset.0,
                    base.1 + current.1 + size_offset.1,
                );

                let zoom = accumulated_zoom.get_value();

                // Pass None for zoom_center since we're baking zoom adjustments into offset
                let _ = render_preview(&canvas, &image_data, total_offset, zoom, None);
            }
        }
    });

    // Interaction start helper
    let start_interaction = move || {
        let canvas_ref = canvas_ref_stored.get_value();
        if let Some(canvas) = canvas_ref.get_untracked() {
            if let Ok(image_data) = capture_canvas_image_data(&canvas) {
                initial_image_data.set_value(Some(image_data));
                initial_canvas_size.set_value(Some((canvas.width(), canvas.height())));
                base_offset.set_value((0.0, 0.0));
                current_drag_offset.set_value((0.0, 0.0));
                accumulated_zoom.set_value(1.0);
                zoom_center.set_value(None);
                transform_sequence.set_value(Vec::new());
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
        is_resizing.set(false);

        // Compose the transformation sequence to get the final matrix
        let sequence = transform_sequence.get_value();
        let composed_matrix: Mat3 = compose_affine_transformations(sequence);

        // Extract center-relative offset and zoom from the composed matrix
        // The matrix is in the form: [[zoom, 0, offset_x], [0, zoom, offset_y], [0, 0, 1]]
        let zoom_factor = composed_matrix.data[0][0];
        let absolute_offset_x = composed_matrix.data[0][2];
        let absolute_offset_y = composed_matrix.data[1][2];

        // Convert absolute pixel offset to center-relative offset
        // This makes the values more intuitive: (0, 0) means we zoomed at canvas center
        let canvas_ref = canvas_ref_stored.get_value();
        let (center_relative_x, center_relative_y) =
            if let Some(canvas) = canvas_ref.get_untracked() {
                let canvas_center_x = canvas.width() as f64 / 2.0;
                let canvas_center_y = canvas.height() as f64 / 2.0;

                // Offset is relative to top-left (0, 0), convert to relative to center
                (
                    absolute_offset_x - canvas_center_x * (1.0 - zoom_factor),
                    absolute_offset_y - canvas_center_y * (1.0 - zoom_factor),
                )
            } else {
                (absolute_offset_x, absolute_offset_y)
            };

        let result = TransformResult {
            offset_x: center_relative_x,
            offset_y: center_relative_y,
            zoom_factor,
            matrix: composed_matrix.data,
        };

        // Clear state
        initial_image_data.set_value(None);
        base_offset.set_value((0.0, 0.0));
        current_drag_offset.set_value((0.0, 0.0));
        accumulated_zoom.set_value(1.0);
        zoom_center.set_value(None);
        transform_sequence.set_value(Vec::new());

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

        // Only capture new imagedata if no interaction session is active
        // If we're within the timeout window, preserve accumulated state
        if initial_image_data.get_value().is_none() {
            start_interaction();
        }

        // Cancel timeout since user is actively dragging again
        if let Some(id) = timeout_id.get_value() {
            web_sys::window().unwrap().clear_timeout_with_handle(id);
            timeout_id.set_value(None);
        }

        is_dragging.set(true);
        drag_start.set_value(Some((ev.client_x() as f64, ev.client_y() as f64)));
        current_drag_offset.set_value((0.0, 0.0)); // Reset current drag offset
    };

    // Pointer move handler
    let on_pointer_move = move |ev: web_sys::PointerEvent| {
        if !is_dragging.get_untracked() {
            return;
        }

        if let Some(start) = drag_start.get_value() {
            let current_x = ev.client_x() as f64;
            let current_y = ev.client_y() as f64;
            // Calculate offset from THIS drag's start point only
            let drag_offset = (current_x - start.0, current_y - start.1);
            current_drag_offset.set_value(drag_offset);
        }
    };

    // Pointer up handler
    let restart_timeout_clone = store_value(restart_timeout);
    let on_pointer_up = move |_ev: web_sys::PointerEvent| {
        is_dragging.set(false);

        // Get current drag offset
        let current = current_drag_offset.get_value();

        // Add drag transformation to sequence (if there was actual movement)
        if current.0.abs() > 0.01 || current.1.abs() > 0.01 {
            transform_sequence.update_value(|seq| {
                seq.push(Transform::Translate {
                    dx: current.0,
                    dy: current.1,
                });
            });
        }

        // Commit current drag offset into base offset for next drag (for preview)
        let base = base_offset.get_value();
        base_offset.set_value((base.0 + current.0, base.1 + current.1));
        current_drag_offset.set_value((0.0, 0.0));

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

        // Get current zoom and offset state (for preview)
        let old_zoom = accumulated_zoom.get_value();
        let old_base = base_offset.get_value();

        // Calculate zoom factor from wheel delta
        let delta = ev.delta_y();
        let zoom_multiplier = (-delta * ZOOM_SENSITIVITY).exp();
        let new_zoom = old_zoom * zoom_multiplier;

        // Get pointer position relative to canvas
        let canvas_ref = canvas_ref_stored.get_value();
        if let Some(canvas) = canvas_ref.get_untracked() {
            let rect = canvas.get_bounding_client_rect();
            let mouse_x = ev.client_x() as f64 - rect.left();
            let mouse_y = ev.client_y() as f64 - rect.top();

            // Add scale transformation to sequence
            // The zoom is centered around the mouse position
            transform_sequence.update_value(|seq| {
                seq.push(Transform::Scale {
                    factor: zoom_multiplier,
                    center_x: mouse_x,
                    center_y: mouse_y,
                });
            });

            // Update preview state (for real-time rendering)
            // When zooming at point (mx, my) with current state (old_zoom, old_offset),
            // we want the image content at (mx, my) to stay at (mx, my).
            //
            // Transformation: new_pixel = old_pixel * new_zoom + new_offset
            // We want: mx = mx * new_zoom + new_offset_x
            // So: new_offset_x = mx * (1 - new_zoom)
            //
            // But we're accumulating! Current transformation is: pixel = original * old_zoom + old_offset
            // After new zoom: pixel = original * new_zoom + new_offset
            //
            // The old offset was already scaled, so when we apply new zoom:
            // new_offset = old_offset * zoom_multiplier + mx * (1 - zoom_multiplier)
            let new_offset_x = old_base.0 * zoom_multiplier + mouse_x * (1.0 - zoom_multiplier);
            let new_offset_y = old_base.1 * zoom_multiplier + mouse_y * (1.0 - zoom_multiplier);

            base_offset.set_value((new_offset_x, new_offset_y));
            accumulated_zoom.set_value(new_zoom);
        }

        // Restart timeout on every wheel event
        restart_timeout_clone2.with_value(|f| f());
    };

    // Canvas resize handler
    let restart_timeout_clone3 = store_value(restart_timeout);
    let on_canvas_resize = move |_new_width: u32, _new_height: u32| {
        // Start interaction if not already started (captures current ImageData and canvas size)
        if !is_dragging.get_untracked()
            && !is_zooming.get_untracked()
            && !is_resizing.get_untracked()
        {
            start_interaction();
        }

        // Mark that we're resizing
        // The RAF loop will automatically calculate offset adjustments based on
        // the difference between initial_canvas_size and current canvas size
        is_resizing.set(true);

        // Restart timeout - if user keeps resizing, we keep delaying the final callback
        restart_timeout_clone3.with_value(|f| f());
    };

    InteractionHandle {
        is_interacting: Signal::derive(move || is_interacting.get()),
        on_pointer_down: Rc::new(on_pointer_down),
        on_pointer_move: Rc::new(on_pointer_move),
        on_pointer_up: Rc::new(on_pointer_up),
        on_wheel: Rc::new(on_wheel),
        on_canvas_resize: Rc::new(on_canvas_resize),
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
