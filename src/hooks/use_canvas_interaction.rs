use leptos::*;

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
    pub is_interacting: ReadSignal<bool>,
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
