use crate::hooks::use_canvas_interaction::TransformResult;
use crate::rendering::{
    coords::{Coord, Rect},
    viewport::Viewport,
};

/// A 2D affine transformation in pixel/canvas space
#[derive(Debug, Clone, PartialEq)]
pub enum Transform {
    /// Translate by (dx, dy) in pixels. Positive dx moves right, positive dy moves down.
    Translate { dx: f64, dy: f64 },
    /// Scale by factor around point (center_x, center_y). Factor < 1 zooms out, > 1 zooms in.
    /// The center point remains fixed during scaling.
    Scale {
        factor: f64,
        center_x: f64,
        center_y: f64,
    },
}

/// A 3x3 homogeneous transformation matrix for 2D affine transformations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3 {
    /// Row-major order: [[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]]
    pub data: [[f64; 3]; 3],
}

impl Mat3 {
    /// Returns the identity matrix (no transformation)
    pub fn identity() -> Self {
        Self {
            data: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Creates a translation matrix for moving by (dx, dy)
    pub fn translation(dx: f64, dy: f64) -> Self {
        Self {
            data: [[1.0, 0.0, dx], [0.0, 1.0, dy], [0.0, 0.0, 1.0]],
        }
    }

    /// Creates a scale matrix around a point (cx, cy)
    ///
    /// This is equivalent to: translate(-cx, -cy) → scale(factor) → translate(cx, cy)
    /// The point (cx, cy) remains fixed during the scaling operation.
    pub fn scale_around(factor: f64, cx: f64, cy: f64) -> Self {
        Self {
            data: [
                [factor, 0.0, cx * (1.0 - factor)],
                [0.0, factor, cy * (1.0 - factor)],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    /// Multiplies this matrix by another (self × other)
    ///
    /// For transformations, left-multiplying applies the transformation:
    /// To compose transformations [T1, T2, T3], compute: T3 × T2 × T1
    pub fn multiply(&self, other: &Mat3) -> Self {
        let mut result = [[0.0; 3]; 3];

        for (i, row) in result.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = self.data[i][0] * other.data[0][j]
                    + self.data[i][1] * other.data[1][j]
                    + self.data[i][2] * other.data[2][j];
            }
        }

        Self { data: result }
    }
}

/// Composes a sequence of 2D affine transformations into a single transformation matrix
///
/// Transformations are applied in order: the first transformation in the sequence
/// is applied first to any point transformed by the resulting matrix.
///
/// # Example
/// ```ignore
/// use fractalwonder::rendering::transforms::{Transform, compose_affine_transformations};
///
/// // Translate right 200px, then scale 0.5x around point (200, 0)
/// let transforms = vec![
///     Transform::Translate { dx: 200.0, dy: 0.0 },
///     Transform::Scale { factor: 0.5, center_x: 200.0, center_y: 0.0 },
/// ];
///
/// let matrix = compose_affine_transformations(transforms);
/// // Point (0, 0) transforms to (200, 0): moved right 200px, then stays there during scaling
/// ```
pub fn compose_affine_transformations(transforms: impl IntoIterator<Item = Transform>) -> Mat3 {
    let mut result = Mat3::identity();

    for transform in transforms {
        let matrix = match transform {
            Transform::Translate { dx, dy } => Mat3::translation(dx, dy),
            Transform::Scale {
                factor,
                center_x,
                center_y,
            } => Mat3::scale_around(factor, center_x, center_y),
        };

        // Left-multiply: result = matrix × result
        // This ensures transformations apply in the correct order
        result = matrix.multiply(&result);
    }

    result
}

pub fn calculate_aspect_ratio(canvas_width: u32, canvas_height: u32) -> f64 {
    canvas_width as f64 / canvas_height as f64
}

pub fn pan_viewport(viewport: &Viewport<f64>, offset_x: f64, offset_y: f64) -> Viewport<f64> {
    let new_center = Coord::new(
        *viewport.center.x() + offset_x,
        *viewport.center.y() + offset_y,
    );

    Viewport::new(new_center, viewport.zoom, viewport.natural_bounds.clone())
}

pub fn zoom_viewport_at_point(
    viewport: &Viewport<f64>,
    zoom_factor: f64,
    pixel_x: f64,
    pixel_y: f64,
    canvas_width: u32,
    canvas_height: u32,
) -> Viewport<f64> {
    let current_bounds = calculate_visible_bounds(viewport, canvas_width, canvas_height);

    let bounds_width = current_bounds.width();
    let bounds_height = current_bounds.height();

    // Convert zoom point from pixel space to image space
    let zoom_point_image_x =
        *current_bounds.min.x() + (pixel_x / canvas_width as f64) * bounds_width;
    let zoom_point_image_y =
        *current_bounds.min.y() + (pixel_y / canvas_height as f64) * bounds_height;

    let new_zoom = viewport.zoom * zoom_factor;

    // Calculate new view dimensions
    let canvas_aspect = calculate_aspect_ratio(canvas_width, canvas_height);

    let new_view_width = (viewport.natural_bounds.width() / new_zoom)
        * if canvas_aspect > 1.0 {
            canvas_aspect
        } else {
            1.0
        };

    let new_view_height = (viewport.natural_bounds.height() / new_zoom)
        * if canvas_aspect < 1.0 {
            1.0 / canvas_aspect
        } else {
            1.0
        };

    // Calculate new center to keep zoom point fixed
    let new_center_x = zoom_point_image_x - (pixel_x / canvas_width as f64 - 0.5) * new_view_width;
    let new_center_y =
        zoom_point_image_y - (pixel_y / canvas_height as f64 - 0.5) * new_view_height;

    let new_center = Coord::new(new_center_x, new_center_y);

    Viewport::new(new_center, new_zoom, viewport.natural_bounds.clone())
}

/// Applies a pixel-space transformation to a viewport, returning a new viewport
///
/// Converts pixel-space transformations from user interactions (TransformResult)
/// into viewport changes in image-space coordinates. This function bridges the gap
/// between the interaction system (which works in pixels) and the viewport system
/// (which works in image coordinates).
///
/// TransformResult offsets are **center-relative**, meaning (0, 0) represents a
/// transformation centered at the canvas center point. This function converts them
/// to absolute coordinates for calculation.
///
/// This handles both pure panning, pure zooming, and combined pan+zoom operations.
pub fn apply_pixel_transform_to_viewport(
    viewport: &Viewport<f64>,
    transform: &TransformResult,
    canvas_width: u32,
    canvas_height: u32,
) -> Viewport<f64> {
    let current_bounds = calculate_visible_bounds(viewport, canvas_width, canvas_height);
    let new_zoom = viewport.zoom * transform.zoom_factor;

    // Convert center-relative offset to absolute offset
    let canvas_center_x = canvas_width as f64 / 2.0;
    let canvas_center_y = canvas_height as f64 / 2.0;
    let absolute_offset_x = transform.offset_x + canvas_center_x * (1.0 - transform.zoom_factor);
    let absolute_offset_y = transform.offset_y + canvas_center_y * (1.0 - transform.zoom_factor);

    // Special case: pure translation (zoom = 1.0)
    // When zoom_factor = 1.0, transformation is: new_pixel = old_pixel + offset
    // There's no fixed point (or equivalently, fixed point is at infinity)
    // The offset directly translates the viewport in image space
    if (transform.zoom_factor - 1.0).abs() < 1e-10 {
        // Pure pan: offset moves pixels, so viewport moves in opposite direction
        // offset in pixels → offset in image space
        let image_offset_x = (absolute_offset_x / canvas_width as f64) * current_bounds.width();
        let image_offset_y = (absolute_offset_y / canvas_height as f64) * current_bounds.height();

        // Viewport moves opposite to pixel offset (dragging right = looking left)
        let new_center_x = *viewport.center.x() - image_offset_x;
        let new_center_y = *viewport.center.y() - image_offset_y;

        return Viewport::new(
            Coord::new(new_center_x, new_center_y),
            new_zoom,
            viewport.natural_bounds.clone(),
        );
    }

    // General case: transformation with zoom
    //
    // The transformation represents how pixels in the CAPTURED canvas are transformed.
    // During preview, pixels are literally moved: new_pixel_pos = old_pixel_pos * zoom + offset
    //
    // We need to figure out what viewport would produce the same visual result.
    // Strategy: Pick a reference pixel, trace where it ends up, and ensure the new viewport
    // shows the same image content at that new position.
    //
    // We'll use the canvas center as our reference point.

    let canvas_center_px = canvas_width as f64 / 2.0;
    let canvas_center_py = canvas_height as f64 / 2.0;

    // What image point is at canvas center in the CURRENT (original) viewport?
    let image_at_original_center = pixel_to_image(
        canvas_center_px,
        canvas_center_py,
        &current_bounds,
        canvas_width,
        canvas_height,
    );

    // During the preview transformation, the pixel at canvas_center moves to a new position:
    // new_pos = canvas_center * zoom + offset (using absolute offset)
    let new_center_px = canvas_center_px * transform.zoom_factor + absolute_offset_x;
    let new_center_py = canvas_center_py * transform.zoom_factor + absolute_offset_y;

    // So the image point that was at canvas center should now be at new_center_px.
    // We need to create a viewport where that image point appears at new_center_px.

    // Calculate new bounds dimensions at new zoom
    let new_view_width_unscaled = viewport.natural_bounds.width() / new_zoom;
    let new_view_height_unscaled = viewport.natural_bounds.height() / new_zoom;

    // Adjust for canvas aspect ratio
    let canvas_aspect = calculate_aspect_ratio(canvas_width, canvas_height);
    let (new_view_width, new_view_height) = if canvas_aspect > 1.0 {
        (
            new_view_height_unscaled * canvas_aspect,
            new_view_height_unscaled,
        )
    } else {
        (
            new_view_width_unscaled,
            new_view_width_unscaled / canvas_aspect,
        )
    };

    // We want: image_at_original_center appears at pixel new_center_px
    // Formula: image_coord = bounds.min + (pixel / canvas_width) * bounds.width
    // So: image_at_original_center = new_bounds.min + (new_center_px / canvas_width) * new_view_width
    // Also: new_bounds.min = new_viewport_center - new_view_width / 2
    // Substituting:
    // image_at_original_center = (new_viewport_center - new_view_width/2) +
    //                            (new_center_px / canvas_width) * new_view_width
    // Solving for new_viewport_center:
    // new_viewport_center = image_at_original_center + new_view_width/2 - (new_center_px / canvas_width) * new_view_width

    let new_viewport_center_x = *image_at_original_center.x() + new_view_width / 2.0
        - (new_center_px / canvas_width as f64) * new_view_width;
    let new_viewport_center_y = *image_at_original_center.y() + new_view_height / 2.0
        - (new_center_py / canvas_height as f64) * new_view_height;

    Viewport::new(
        Coord::new(new_viewport_center_x, new_viewport_center_y),
        new_zoom,
        viewport.natural_bounds.clone(),
    )
}

pub fn calculate_visible_bounds<T>(
    viewport: &Viewport<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> Rect<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Div<f64, Output = T>
        + std::ops::Mul<f64, Output = T>,
{
    let natural_width = viewport.natural_bounds.width();
    let natural_height = viewport.natural_bounds.height();

    // Apply zoom (1.0 = show entire natural bounds)
    let view_width = natural_width / viewport.zoom;
    let view_height = natural_height / viewport.zoom;

    // Adjust for canvas aspect ratio - extend the wider dimension
    let canvas_aspect = calculate_aspect_ratio(canvas_width, canvas_height);

    let (final_width, final_height) = if canvas_aspect > 1.0 {
        // Landscape - extend width
        (view_height.clone() * canvas_aspect, view_height)
    } else {
        // Portrait - extend height
        (view_width.clone(), view_width / canvas_aspect)
    };

    // Calculate bounds centered on viewport.center
    let half_width = final_width.clone() / 2.0;
    let half_height = final_height.clone() / 2.0;

    Rect::new(
        Coord::new(
            viewport.center.x().clone() - half_width.clone(),
            viewport.center.y().clone() - half_height.clone(),
        ),
        Coord::new(
            viewport.center.x().clone() + half_width,
            viewport.center.y().clone() + half_height,
        ),
    )
}

pub fn pixel_to_image<T>(
    pixel_x: f64,
    pixel_y: f64,
    target_rect: &Rect<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> Coord<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Mul<f64, Output = T>,
{
    let bounds_width = target_rect.width();
    let bounds_height = target_rect.height();

    Coord::new(
        target_rect.min.x().clone() + bounds_width * (pixel_x / canvas_width as f64),
        target_rect.min.y().clone() + bounds_height * (pixel_y / canvas_height as f64),
    )
}

pub fn image_to_pixel<T>(
    image: &Coord<T>,
    target_rect: &Rect<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> (f64, f64)
where
    T: Clone + std::ops::Sub<Output = T> + std::ops::Div<Output = T>,
    f64: std::ops::Mul<T, Output = f64>,
{
    let bounds_width = target_rect.width();
    let bounds_height = target_rect.height();

    let normalized_x = (image.x().clone() - target_rect.min.x().clone()) / bounds_width;
    let normalized_y = (image.y().clone() - target_rect.min.y().clone()) / bounds_height;

    (
        canvas_width as f64 * normalized_x,
        canvas_height as f64 * normalized_y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_visible_bounds_landscape() {
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        // Landscape canvas: 1600x900 (aspect ratio ~1.78)
        let bounds = calculate_visible_bounds(&viewport, 1600, 900);

        // At zoom 1.0, should show entire natural height (100 units)
        // Width should extend to maintain aspect ratio
        assert_eq!(bounds.height(), 100.0);
        assert!((bounds.width() - 177.77).abs() < 0.1); // 100 * 1.78
        assert_eq!(*bounds.min.y(), -50.0);
        assert_eq!(*bounds.max.y(), 50.0);
    }

    #[test]
    fn test_calculate_visible_bounds_portrait() {
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        // Portrait canvas: 900x1600
        let bounds = calculate_visible_bounds(&viewport, 900, 1600);

        // At zoom 1.0, should show entire natural width (100 units)
        // Height should extend to maintain aspect ratio
        assert_eq!(bounds.width(), 100.0);
        assert!((bounds.height() - 177.77).abs() < 0.1);
        assert_eq!(*bounds.min.x(), -50.0);
        assert_eq!(*bounds.max.x(), 50.0);
    }

    #[test]
    fn test_calculate_visible_bounds_zoom() {
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            2.0, // 2x zoom
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        // Square canvas
        let bounds = calculate_visible_bounds(&viewport, 1000, 1000);

        // At zoom 2.0, should show half the natural area (50 units)
        assert_eq!(bounds.width(), 50.0);
        assert_eq!(bounds.height(), 50.0);
    }

    #[test]
    fn test_pixel_to_image_center() {
        let bounds = Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0));
        let image = pixel_to_image(500.0, 500.0, &bounds, 1000, 1000);

        assert_eq!(*image.x(), 0.0);
        assert_eq!(*image.y(), 0.0);
    }

    #[test]
    fn test_pixel_to_image_corners() {
        let bounds = Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0));

        // Top-left corner
        let image = pixel_to_image(0.0, 0.0, &bounds, 1000, 1000);
        assert_eq!(*image.x(), -50.0);
        assert_eq!(*image.y(), -50.0);

        // Bottom-right corner
        let image = pixel_to_image(1000.0, 1000.0, &bounds, 1000, 1000);
        assert_eq!(*image.x(), 50.0);
        assert_eq!(*image.y(), 50.0);
    }

    #[test]
    fn test_image_to_pixel_center() {
        let bounds = Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0));
        let image = Coord::new(0.0, 0.0);
        let (px, py) = image_to_pixel(&image, &bounds, 1000, 1000);

        assert_eq!(px, 500.0);
        assert_eq!(py, 500.0);
    }

    #[test]
    fn test_round_trip_pixel_image_pixel() {
        let bounds = Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0));
        let (orig_x, orig_y) = (123.0, 456.0);

        let image = pixel_to_image(orig_x, orig_y, &bounds, 1000, 1000);
        let (result_x, result_y) = image_to_pixel(&image, &bounds, 1000, 1000);

        assert!((result_x - orig_x).abs() < 0.001);
        assert!((result_y - orig_y).abs() < 0.001);
    }

    #[test]
    fn test_calculate_aspect_ratio_landscape() {
        let aspect = calculate_aspect_ratio(1920, 1080);
        assert!((aspect - 1.7777).abs() < 0.001);
    }

    #[test]
    fn test_calculate_aspect_ratio_portrait() {
        let aspect = calculate_aspect_ratio(1080, 1920);
        assert!((aspect - 0.5625).abs() < 0.001);
    }

    #[test]
    fn test_calculate_aspect_ratio_square() {
        let aspect = calculate_aspect_ratio(1000, 1000);
        assert_eq!(aspect, 1.0);
    }

    #[test]
    fn test_pan_viewport_right() {
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        // Pan right 10 image units
        let new_viewport = pan_viewport(&viewport, 10.0, 0.0);

        assert_eq!(*new_viewport.center.x(), 10.0);
        assert_eq!(*new_viewport.center.y(), 0.0);
        assert_eq!(new_viewport.zoom, 1.0);
    }

    #[test]
    fn test_pan_viewport_from_offset_position() {
        let viewport = Viewport::new(
            Coord::new(20.0, -10.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        // Pan left 5 units, down 3 units
        let new_viewport = pan_viewport(&viewport, -5.0, 3.0);

        assert_eq!(*new_viewport.center.x(), 15.0);
        assert_eq!(*new_viewport.center.y(), -7.0);
        assert_eq!(new_viewport.zoom, 1.0);
    }

    #[test]
    fn test_zoom_viewport_at_center_point() {
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // Zoom 2x at canvas center
        let zoom_point_x = canvas_width as f64 / 2.0;
        let zoom_point_y = canvas_height as f64 / 2.0;

        let new_viewport = zoom_viewport_at_point(
            &viewport,
            2.0,
            zoom_point_x,
            zoom_point_y,
            canvas_width,
            canvas_height,
        );

        // Center should stay the same when zooming at center
        assert!((*new_viewport.center.x() - 0.0).abs() < 0.01);
        assert!((*new_viewport.center.y() - 0.0).abs() < 0.01);
        assert_eq!(new_viewport.zoom, 2.0);
    }

    #[test]
    fn test_zoom_viewport_at_corner() {
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // Zoom 2x at top-left corner
        let new_viewport =
            zoom_viewport_at_point(&viewport, 2.0, 0.0, 0.0, canvas_width, canvas_height);

        // Center should move toward top-left
        assert!(*new_viewport.center.x() < 0.0);
        assert!(*new_viewport.center.y() < 0.0);
        assert_eq!(new_viewport.zoom, 2.0);
    }

    #[test]
    fn test_zoom_viewport_out() {
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            2.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // Zoom out 0.5x at center
        let zoom_point_x = canvas_width as f64 / 2.0;
        let zoom_point_y = canvas_height as f64 / 2.0;

        let new_viewport = zoom_viewport_at_point(
            &viewport,
            0.5,
            zoom_point_x,
            zoom_point_y,
            canvas_width,
            canvas_height,
        );

        // Center should stay roughly the same
        assert!((*new_viewport.center.x() - 0.0).abs() < 0.01);
        assert!((*new_viewport.center.y() - 0.0).abs() < 0.01);
        assert_eq!(new_viewport.zoom, 1.0); // 2.0 * 0.5
    }

    #[test]
    fn test_drag_right_then_zoom_out() {
        use crate::hooks::use_canvas_interaction::TransformResult;

        // Start at origin
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // User drags RIGHT 300 pixels, then zooms OUT to 0.5x at the new image center
        // Dragging right 300px means offset = +300
        // New image center is 300px to the right of canvas center: 400 + 300 = 700
        // Zoom out 0.5x at pixel 700
        let drag_offset = 300.0;
        let zoom_factor = 0.5;
        let zoom_point_x = 700.0; // Where user's mouse is (new image center)
        let zoom_point_y = 300.0;

        // Absolute offset (internal calculation) = old_offset * zoom + mouse * (1 - zoom)
        // absolute = 300 * 0.5 + 700 * (1 - 0.5) = 150 + 350 = 500
        let absolute_offset_x = drag_offset * zoom_factor + zoom_point_x * (1.0 - zoom_factor);
        let absolute_offset_y = 0.0 * zoom_factor + zoom_point_y * (1.0 - zoom_factor);

        // Convert to center-relative for TransformResult
        let canvas_center_x = canvas_width as f64 / 2.0;
        let canvas_center_y = canvas_height as f64 / 2.0;
        let offset_x = absolute_offset_x - canvas_center_x * (1.0 - zoom_factor);
        let offset_y = absolute_offset_y - canvas_center_y * (1.0 - zoom_factor);

        let result = TransformResult {
            offset_x,
            offset_y,
            zoom_factor,
            matrix: [
                [zoom_factor, 0.0, absolute_offset_x],
                [0.0, zoom_factor, absolute_offset_y],
                [0.0, 0.0, 1.0],
            ],
        };

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        println!("Drag right 300px, zoom out 0.5x at pixel 700");
        println!(
            "offset_x: {}, offset_y: {}",
            result.offset_x, result.offset_y
        );
        println!("zoom_factor: {}", result.zoom_factor);
        println!("zoom point (mouse): ({}, {})", zoom_point_x, zoom_point_y);
        println!(
            "new viewport center: ({}, {})",
            new_viewport.center.x(),
            new_viewport.center.y()
        );
        println!("new viewport zoom: {}", new_viewport.zoom);

        // After dragging right, we're looking left (negative x)
        // After zooming out at the new center, that point should remain under the mouse

        // Let's calculate where pixel 700 should be in the new viewport
        // In the original viewport at zoom 1.0 with canvas 800x600:
        // - Canvas shows approximately -67 to +67 in x (landscape aspect)
        // - Pixel 700 corresponds to image x = -67 + (700/800) * 133 = -67 + 116.625 = 49.625

        // After dragging RIGHT 300px, we're looking LEFT
        // Image offset = -(300/800) * 133 = -50 image units
        // So we're centered at x = -50

        // The point at image x = 49.625 is now at pixel position...
        // Actually, let me just verify that the zoom point stays fixed

        // Calculate where the zoom point appears in the new viewport
        let new_bounds = calculate_visible_bounds(&new_viewport, canvas_width, canvas_height);
        let zoom_point_in_new_viewport_x =
            *new_bounds.min.x() + (zoom_point_x / canvas_width as f64) * new_bounds.width();

        // This should equal where the zoom point was in the original (dragged) viewport
        let original_dragged_center_x = -(drag_offset / canvas_width as f64)
            * calculate_visible_bounds(&viewport, canvas_width, canvas_height).width();
        let original_bounds_at_drag = Rect::new(
            Coord::new(original_dragged_center_x - 66.67, -50.0),
            Coord::new(original_dragged_center_x + 66.67, 50.0),
        );
        let zoom_point_in_original_x = *original_bounds_at_drag.min.x()
            + (zoom_point_x / canvas_width as f64) * original_bounds_at_drag.width();

        println!(
            "Zoom point in new viewport: {}",
            zoom_point_in_new_viewport_x
        );
        println!(
            "Zoom point in original (dragged) viewport: {}",
            zoom_point_in_original_x
        );

        // These should be equal (zoom point should stay fixed)
        assert!(
            (zoom_point_in_new_viewport_x - zoom_point_in_original_x).abs() < 0.1,
            "Zoom point should remain fixed: expected {}, got {}",
            zoom_point_in_original_x,
            zoom_point_in_new_viewport_x
        );
    }

    #[test]
    fn test_pan_left_then_zoom_at_new_center() {
        use crate::hooks::use_canvas_interaction::TransformResult;

        // Start at origin
        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // User drags left 100 pixels, then zooms 2x at canvas center
        // Absolute offset = old_offset * zoom + mouse * (1 - zoom)
        // absolute = -100 * 2.0 + 400 * (1 - 2.0) = -200 - 400 = -600
        let drag_offset = -100.0;
        let zoom_factor = 2.0;
        let mouse_x = canvas_width as f64 / 2.0;
        let mouse_y = canvas_height as f64 / 2.0;

        let absolute_offset_x = drag_offset * zoom_factor + mouse_x * (1.0 - zoom_factor);
        let absolute_offset_y = 0.0 * zoom_factor + mouse_y * (1.0 - zoom_factor);

        // Convert to center-relative
        let canvas_center_x = canvas_width as f64 / 2.0;
        let canvas_center_y = canvas_height as f64 / 2.0;
        let offset_x = absolute_offset_x - canvas_center_x * (1.0 - zoom_factor);
        let offset_y = absolute_offset_y - canvas_center_y * (1.0 - zoom_factor);

        let result = TransformResult {
            offset_x,
            offset_y,
            zoom_factor,
            matrix: [
                [zoom_factor, 0.0, absolute_offset_x],
                [0.0, zoom_factor, absolute_offset_y],
                [0.0, 0.0, 1.0],
            ],
        };

        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        // The image was dragged left by 100px, so we're looking at content to the right
        // Then we zoomed at canvas center
        // Expected: viewport center should be to the RIGHT of origin (positive x)
        // because dragging left means looking right

        // At zoom=1, dragging left 100px on an 800px canvas moves viewport right
        // by approximately 100/800 * bounds_width
        // After the pan, if we were at zoom=1, center would be at positive x
        // Then zooming 2x at the canvas center should keep that point relatively fixed

        // Verify zoom is correct
        assert_eq!(new_viewport.zoom, 2.0);

        // The viewport should have moved due to the pan
        // Dragging left means we're looking to the right, so center.x should be positive
        assert!(
            *new_viewport.center.x() > 0.0,
            "After dragging left, viewport center should be positive (looking right), got x={}",
            new_viewport.center.x()
        );
    }

    #[test]
    fn test_mat3_identity() {
        let id = Mat3::identity();
        assert_eq!(id.data, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    }

    #[test]
    fn test_mat3_translation() {
        let t = Mat3::translation(200.0, 100.0);
        assert_eq!(
            t.data,
            [[1.0, 0.0, 200.0], [0.0, 1.0, 100.0], [0.0, 0.0, 1.0]]
        );

        // Transform point (0, 0) → should move to (200, 100)
        let x = 0.0 * t.data[0][0] + 0.0 * t.data[0][1] + 1.0 * t.data[0][2];
        let y = 0.0 * t.data[1][0] + 0.0 * t.data[1][1] + 1.0 * t.data[1][2];
        assert_eq!(x, 200.0);
        assert_eq!(y, 100.0);
    }

    #[test]
    fn test_mat3_scale_around_origin() {
        let s = Mat3::scale_around(0.5, 0.0, 0.0);
        assert_eq!(s.data, [[0.5, 0.0, 0.0], [0.0, 0.5, 0.0], [0.0, 0.0, 1.0]]);
    }

    #[test]
    fn test_mat3_scale_around_point() {
        // Scale 0.5x around point (200, 0)
        let s = Mat3::scale_around(0.5, 200.0, 0.0);

        // Matrix should be: [[0.5, 0, 100], [0, 0.5, 0], [0, 0, 1]]
        // Because: cx(1-s) = 200(1-0.5) = 100
        assert_eq!(s.data[0][0], 0.5);
        assert_eq!(s.data[1][1], 0.5);
        assert_eq!(s.data[0][2], 100.0);
        assert_eq!(s.data[1][2], 0.0);

        // Point (200, 0) should stay at (200, 0) after scaling
        let x = 200.0 * s.data[0][0] + 0.0 * s.data[0][1] + 1.0 * s.data[0][2];
        let y = 200.0 * s.data[1][0] + 0.0 * s.data[1][1] + 1.0 * s.data[1][2];
        assert_eq!(x, 200.0);
        assert_eq!(y, 0.0);

        // Point (0, 0) should move toward (200, 0)
        let x = 0.0 * s.data[0][0] + 0.0 * s.data[0][1] + 1.0 * s.data[0][2];
        let y = 0.0 * s.data[1][0] + 0.0 * s.data[1][1] + 1.0 * s.data[1][2];
        assert_eq!(x, 100.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn test_mat3_multiply_identity() {
        let t = Mat3::translation(100.0, 50.0);
        let id = Mat3::identity();
        let result = t.multiply(&id);
        assert_eq!(result.data, t.data);
    }

    #[test]
    fn test_mat3_multiply_translations() {
        // Translate by (100, 0) then by (50, 0) = translate by (150, 0)
        let t1 = Mat3::translation(100.0, 0.0);
        let t2 = Mat3::translation(50.0, 0.0);
        let result = t2.multiply(&t1);

        // Transform point (0, 0)
        let x = 0.0 * result.data[0][0] + 0.0 * result.data[0][1] + 1.0 * result.data[0][2];
        assert_eq!(x, 150.0);
    }

    #[test]
    fn test_compose_single_translation() {
        let transforms = vec![Transform::Translate { dx: 200.0, dy: 0.0 }];
        let matrix = compose_affine_transformations(transforms);

        assert_eq!(
            matrix.data,
            [[1.0, 0.0, 200.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn test_compose_single_scale() {
        let transforms = vec![Transform::Scale {
            factor: 0.5,
            center_x: 200.0,
            center_y: 0.0,
        }];
        let matrix = compose_affine_transformations(transforms);

        assert_eq!(
            matrix.data,
            [[0.5, 0.0, 100.0], [0.0, 0.5, 0.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn test_compose_translate_then_scale() {
        // Your example: translate(200, 0) then scale(0.5) around (200, 0)
        let transforms = vec![
            Transform::Translate { dx: 200.0, dy: 0.0 },
            Transform::Scale {
                factor: 0.5,
                center_x: 200.0,
                center_y: 0.0,
            },
        ];
        let matrix = compose_affine_transformations(transforms);

        // Expected result: point (0,0) should end up at (200, 0)
        // T1 = [[1, 0, 200], [0, 1, 0], [0, 0, 1]]
        // T2 = [[0.5, 0, 100], [0, 0.5, 0], [0, 0, 1]]
        // Final = T2 × T1 = [[0.5, 0, 200], [0, 0.5, 0], [0, 0, 1]]

        println!("Result matrix: {:?}", matrix.data);

        // Transform point (0, 0)
        let x = 0.0 * matrix.data[0][0] + 0.0 * matrix.data[0][1] + 1.0 * matrix.data[0][2];
        let y = 0.0 * matrix.data[1][0] + 0.0 * matrix.data[1][1] + 1.0 * matrix.data[1][2];

        println!("Point (0,0) transforms to ({}, {})", x, y);

        assert!((x - 200.0).abs() < 0.0001, "Expected x=200, got x={}", x);
        assert!((y - 0.0).abs() < 0.0001, "Expected y=0, got y={}", y);
    }

    #[test]
    fn test_compose_empty_sequence() {
        let transforms: Vec<Transform> = vec![];
        let matrix = compose_affine_transformations(transforms);
        assert_eq!(matrix.data, Mat3::identity().data);
    }

    #[test]
    fn test_compose_multiple_translations() {
        let transforms = vec![
            Transform::Translate { dx: 100.0, dy: 0.0 },
            Transform::Translate { dx: 50.0, dy: 0.0 },
            Transform::Translate {
                dx: -20.0,
                dy: 30.0,
            },
        ];
        let matrix = compose_affine_transformations(transforms);

        // Transform point (0, 0) - should be at (130, 30)
        let x = 0.0 * matrix.data[0][0] + 0.0 * matrix.data[0][1] + 1.0 * matrix.data[0][2];
        let y = 0.0 * matrix.data[1][0] + 0.0 * matrix.data[1][1] + 1.0 * matrix.data[1][2];

        assert!((x - 130.0).abs() < 0.0001);
        assert!((y - 30.0).abs() < 0.0001);
    }

    #[test]
    fn test_compose_three_translations_to_identity() {
        // Three translations that cancel out to identity
        let transforms = vec![
            Transform::Translate { dx: 200.0, dy: 0.0 },
            Transform::Translate {
                dx: 0.0,
                dy: -200.0,
            },
            Transform::Translate {
                dx: -200.0,
                dy: 200.0,
            },
        ];
        let matrix = compose_affine_transformations(transforms);

        // Should be identity matrix
        let expected = Mat3::identity();
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (matrix.data[i][j] - expected.data[i][j]).abs() < 0.0001,
                    "Matrix element [{},{}]: expected {}, got {}",
                    i,
                    j,
                    expected.data[i][j],
                    matrix.data[i][j]
                );
            }
        }

        // Verify with a test point: (100, 100) should stay at (100, 100)
        let x = 100.0 * matrix.data[0][0] + 100.0 * matrix.data[0][1] + 1.0 * matrix.data[0][2];
        let y = 100.0 * matrix.data[1][0] + 100.0 * matrix.data[1][1] + 1.0 * matrix.data[1][2];
        assert!((x - 100.0).abs() < 0.0001);
        assert!((y - 100.0).abs() < 0.0001);
    }

    #[test]
    fn test_compose_translate_scale_translate_scale_to_identity() {
        // Complex sequence that cancels out to identity:
        // translate(200,0), scale(0.5, 0, 0), translate(-100,0), scale(2, 0, 0)
        let transforms = vec![
            Transform::Translate { dx: 200.0, dy: 0.0 },
            Transform::Scale {
                factor: 0.5,
                center_x: 0.0,
                center_y: 0.0,
            },
            Transform::Translate {
                dx: -100.0,
                dy: 0.0,
            },
            Transform::Scale {
                factor: 2.0,
                center_x: 0.0,
                center_y: 0.0,
            },
        ];
        let matrix = compose_affine_transformations(transforms);

        println!("Result matrix: {:?}", matrix.data);

        // Should be identity matrix
        let expected = Mat3::identity();
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (matrix.data[i][j] - expected.data[i][j]).abs() < 0.0001,
                    "Matrix element [{},{}]: expected {}, got {}",
                    i,
                    j,
                    expected.data[i][j],
                    matrix.data[i][j]
                );
            }
        }

        // Verify with test points
        // Point (0, 0) should stay at (0, 0)
        let x = 0.0 * matrix.data[0][0] + 0.0 * matrix.data[0][1] + 1.0 * matrix.data[0][2];
        let y = 0.0 * matrix.data[1][0] + 0.0 * matrix.data[1][1] + 1.0 * matrix.data[1][2];
        assert!((x - 0.0).abs() < 0.0001);
        assert!((y - 0.0).abs() < 0.0001);

        // Point (100, 100) should stay at (100, 100)
        let x = 100.0 * matrix.data[0][0] + 100.0 * matrix.data[0][1] + 1.0 * matrix.data[0][2];
        let y = 100.0 * matrix.data[1][0] + 100.0 * matrix.data[1][1] + 1.0 * matrix.data[1][2];
        println!("Point (100,100) transforms to ({}, {})", x, y);
        assert!((x - 100.0).abs() < 0.0001);
        assert!((y - 100.0).abs() < 0.0001);

        // Point (10, 20) should stay at (10, 20)
        let x = 10.0 * matrix.data[0][0] + 20.0 * matrix.data[0][1] + 1.0 * matrix.data[0][2];
        let y = 10.0 * matrix.data[1][0] + 20.0 * matrix.data[1][1] + 1.0 * matrix.data[1][2];
        println!("Point (10,20) transforms to ({}, {})", x, y);
        assert!((x - 10.0).abs() < 0.0001);
        assert!((y - 20.0).abs() < 0.0001);
    }

    #[test]
    fn test_compose_scale_then_translate() {
        // Scale 0.5x around origin, then translate (100, 0)
        let transforms = vec![
            Transform::Scale {
                factor: 0.5,
                center_x: 0.0,
                center_y: 0.0,
            },
            Transform::Translate { dx: 100.0, dy: 0.0 },
        ];
        let matrix = compose_affine_transformations(transforms);

        // Point (0, 0) should be at (100, 0)
        let x = 0.0 * matrix.data[0][0] + 0.0 * matrix.data[0][1] + 1.0 * matrix.data[0][2];
        let y = 0.0 * matrix.data[1][0] + 0.0 * matrix.data[1][1] + 1.0 * matrix.data[1][2];

        assert!((x - 100.0).abs() < 0.0001);
        assert!((y - 0.0).abs() < 0.0001);

        // Point (200, 0) should be at: scaled to (100, 0), then translated to (200, 0)
        let x = 200.0 * matrix.data[0][0] + 0.0 * matrix.data[0][1] + 1.0 * matrix.data[0][2];
        assert!((x - 200.0).abs() < 0.0001);
    }

    #[test]
    fn test_compound_drag_zoom_mouse_stays_fixed() {
        use crate::hooks::use_canvas_interaction::TransformResult;

        // This test verifies that when dragging + zooming in a single interaction,
        // the zoom operation keeps the point under the mouse fixed.
        //
        // Key insight: After dragging, the image content under the mouse has changed.
        // When we then zoom, we want to keep THAT content (post-drag) fixed, not the
        // original content that was there before the drag.

        let viewport = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );

        let canvas_width = 800;
        let canvas_height = 600;

        // Scenario: Drag right 300px, then zoom out 0.5x with mouse at pixel (700, 300)
        let drag_offset = 300.0;
        let zoom_factor = 0.5;
        let mouse_x = 700.0;
        let mouse_y = 300.0;

        // Interaction hook formula for absolute offset
        let absolute_offset_x = drag_offset * zoom_factor + mouse_x * (1.0 - zoom_factor);
        let absolute_offset_y = 0.0 * zoom_factor + mouse_y * (1.0 - zoom_factor);

        // Convert to center-relative
        let canvas_center_x = canvas_width as f64 / 2.0;
        let canvas_center_y = canvas_height as f64 / 2.0;
        let offset_x = absolute_offset_x - canvas_center_x * (1.0 - zoom_factor);
        let offset_y = absolute_offset_y - canvas_center_y * (1.0 - zoom_factor);

        let result = TransformResult {
            offset_x,
            offset_y,
            zoom_factor,
            matrix: [
                [zoom_factor, 0.0, absolute_offset_x],
                [0.0, zoom_factor, absolute_offset_y],
                [0.0, 0.0, 1.0],
            ],
        };

        // Calculate what image point is under the mouse AFTER the drag (but before zoom)
        // The drag moves the viewport, so we need to calculate the intermediate state
        let original_bounds = calculate_visible_bounds(&viewport, canvas_width, canvas_height);
        let drag_in_image_units = (drag_offset / canvas_width as f64) * original_bounds.width();

        // After dragging right, we're looking left, so subtract
        let intermediate_center_x = *viewport.center.x() - drag_in_image_units;
        let intermediate_viewport = Viewport::new(
            Coord::new(intermediate_center_x, *viewport.center.y()),
            viewport.zoom,
            viewport.natural_bounds.clone(),
        );

        let intermediate_bounds =
            calculate_visible_bounds(&intermediate_viewport, canvas_width, canvas_height);
        let image_point_at_mouse = pixel_to_image(
            mouse_x,
            mouse_y,
            &intermediate_bounds,
            canvas_width,
            canvas_height,
        );

        println!("Mouse at pixel ({}, {})", mouse_x, mouse_y);
        println!(
            "After drag, image point at mouse: ({}, {})",
            image_point_at_mouse.x(),
            image_point_at_mouse.y()
        );
        println!(
            "Transform: offset=({}, {}), zoom={}",
            offset_x, offset_y, zoom_factor
        );

        // Apply transformation
        let new_viewport =
            apply_pixel_transform_to_viewport(&viewport, &result, canvas_width, canvas_height);

        println!(
            "New viewport center: ({}, {})",
            new_viewport.center.x(),
            new_viewport.center.y()
        );
        println!("New viewport zoom: {}", new_viewport.zoom);

        // Get the image point that's under the mouse AFTER the complete transformation
        let new_bounds = calculate_visible_bounds(&new_viewport, canvas_width, canvas_height);
        let image_point_after =
            pixel_to_image(mouse_x, mouse_y, &new_bounds, canvas_width, canvas_height);

        println!(
            "After drag+zoom, image point at mouse: ({}, {})",
            image_point_after.x(),
            image_point_after.y()
        );

        // The image point that was under the mouse AFTER dragging should still be under the mouse AFTER zooming
        // This is the key invariant: zooming keeps the content under the cursor fixed
        assert!(
            (*image_point_after.x() - *image_point_at_mouse.x()).abs() < 0.1,
            "X coordinate under mouse should stay fixed during zoom: after_drag={}, after_zoom={}",
            image_point_at_mouse.x(),
            image_point_after.x()
        );
        assert!(
            (*image_point_after.y() - *image_point_at_mouse.y()).abs() < 0.1,
            "Y coordinate under mouse should stay fixed during zoom: after_drag={}, after_zoom={}",
            image_point_at_mouse.y(),
            image_point_after.y()
        );
    }
}
