use leptos::*;
use web_sys::ImageData;

#[allow(dead_code)]
const INTERACTION_TIMEOUT_MS: i32 = 1500;
#[allow(dead_code)]
const ZOOM_SENSITIVITY: f64 = 0.0005;

/// Transformation result returned when interaction ends
#[derive(Debug, Clone, PartialEq)]
pub struct TransformResult {
    pub offset_x: f64,
    pub offset_y: f64,
    pub zoom_factor: f64,
    pub matrix: [[f64; 3]; 3],
}

/// Handle returned by the hook
pub struct InteractionHandle {
    pub is_interacting: Signal<bool>,
    pub reset: Box<dyn Fn()>,
}

/// Builds a 2D affine transformation matrix from offset, zoom, and optional zoom center
#[allow(dead_code)]
fn build_transform_matrix(
    offset: (f64, f64),
    zoom: f64,
    zoom_center: Option<(f64, f64)>,
) -> [[f64; 3]; 3] {
    let mut matrix = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];

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

pub fn use_canvas_interaction<F>(
    _canvas_ref: NodeRef<leptos::html::Canvas>,
    _on_interaction_end: F,
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

    // Reset function
    let reset = {
        Box::new(move || {
            is_dragging.set(false);
            is_zooming.set(false);
            initial_image_data.set_value(None);
            drag_start.set_value(None);
            accumulated_offset.set_value((0.0, 0.0));
            accumulated_zoom.set_value(1.0);
            zoom_center.set_value(None);
            animation_frame_id.set_value(None);
        })
    };

    InteractionHandle {
        is_interacting: Signal::derive(move || is_interacting.get()),
        reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let matrix = build_transform_matrix((0.0, 0.0), 1.0, None);
        assert_eq!(matrix, [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ]);
    }

    #[test]
    fn test_translation_matrix() {
        let matrix = build_transform_matrix((100.0, 50.0), 1.0, None);
        assert_eq!(matrix, [
            [1.0, 0.0, 100.0],
            [0.0, 1.0, 50.0],
            [0.0, 0.0, 1.0],
        ]);
    }

    #[test]
    fn test_zoom_matrix_no_center() {
        let matrix = build_transform_matrix((0.0, 0.0), 2.0, None);
        assert_eq!(matrix, [
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 1.0],
        ]);
    }

    #[test]
    fn test_zoom_matrix_with_center() {
        let matrix = build_transform_matrix((0.0, 0.0), 2.0, Some((100.0, 100.0)));
        // Zoom 2x centered at (100, 100)
        // Translation should be 100*(1-2) = -100 for both x and y
        assert_eq!(matrix, [
            [2.0, 0.0, -100.0],
            [0.0, 2.0, -100.0],
            [0.0, 0.0, 1.0],
        ]);
    }

    #[test]
    fn test_combined_transform() {
        let matrix = build_transform_matrix((50.0, 30.0), 1.5, Some((200.0, 150.0)));
        // offset + center*(1-zoom)
        // x: 50 + 200*(1-1.5) = 50 + 200*(-0.5) = 50 - 100 = -50
        // y: 30 + 150*(1-1.5) = 30 + 150*(-0.5) = 30 - 75 = -45
        assert_eq!(matrix, [
            [1.5, 0.0, -50.0],
            [0.0, 1.5, -45.0],
            [0.0, 0.0, 1.0],
        ]);
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

        let handle = use_canvas_interaction(
            canvas_ref,
            move |_result| {
                callback_fired.set(true);
            },
        );

        assert!(!handle.is_interacting.get());
    }
}
