use crate::points::{Point, Rect};
use crate::viewport::Viewport;

/// Transformation result returned when user interaction ends
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
    /// 2D affine transformation matrix [3x3] encoding offset + zoom in absolute coordinates
    /// (used internally for canvas rendering, not for external interpretation)
    pub matrix: [[f64; 3]; 3],
}

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

pub fn pan_viewport<T>(viewport: &Viewport<T>, offset_x: T, offset_y: T) -> Viewport<T>
where
    T: Clone + std::ops::Add<Output = T>,
{
    let new_center = Point::new(
        viewport.center.x().clone() + offset_x,
        viewport.center.y().clone() + offset_y,
    );

    Viewport::new(new_center, viewport.zoom)
}

pub fn zoom_viewport_at_point<T>(
    viewport: &Viewport<T>,
    natural_bounds: &Rect<T>,
    zoom_factor: f64,
    pixel_x: f64,
    pixel_y: f64,
    canvas_width: u32,
    canvas_height: u32,
) -> Viewport<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Div<f64, Output = T>
        + std::ops::Mul<f64, Output = T>,
{
    let current_bounds =
        calculate_visible_bounds(viewport, natural_bounds, canvas_width, canvas_height);

    let bounds_width = current_bounds.width();
    let bounds_height = current_bounds.height();

    // Convert zoom point from pixel space to image space
    let zoom_point_image_x =
        current_bounds.min.x().clone() + bounds_width.clone() * (pixel_x / canvas_width as f64);
    let zoom_point_image_y =
        current_bounds.min.y().clone() + bounds_height.clone() * (pixel_y / canvas_height as f64);

    let new_zoom = viewport.zoom * zoom_factor;

    // Calculate new view dimensions
    let canvas_aspect = calculate_aspect_ratio(canvas_width, canvas_height);

    let new_view_width = (natural_bounds.width() / new_zoom)
        * if canvas_aspect > 1.0 {
            canvas_aspect
        } else {
            1.0
        };

    let new_view_height = (natural_bounds.height() / new_zoom)
        * if canvas_aspect < 1.0 {
            1.0 / canvas_aspect
        } else {
            1.0
        };

    // Calculate new center to keep zoom point fixed
    let new_center_x =
        zoom_point_image_x - new_view_width.clone() * (pixel_x / canvas_width as f64 - 0.5);
    let new_center_y =
        zoom_point_image_y - new_view_height.clone() * (pixel_y / canvas_height as f64 - 0.5);

    let new_center = Point::new(new_center_x, new_center_y);

    Viewport::new(new_center, new_zoom)
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
pub fn apply_pixel_transform_to_viewport<T>(
    viewport: &Viewport<T>,
    natural_bounds: &Rect<T>,
    transform: &TransformResult,
    canvas_width: u32,
    canvas_height: u32,
) -> Viewport<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Div<f64, Output = T>
        + std::ops::Mul<f64, Output = T>,
{
    let current_bounds =
        calculate_visible_bounds(viewport, natural_bounds, canvas_width, canvas_height);
    let new_zoom = viewport.zoom * transform.zoom_factor;

    // Convert center-relative offset to absolute offset (pixel space)
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
        let image_offset_x = current_bounds.width() * (absolute_offset_x / canvas_width as f64);
        let image_offset_y = current_bounds.height() * (absolute_offset_y / canvas_height as f64);

        // Viewport moves opposite to pixel offset (dragging right = looking left)
        let new_center = Point::new(
            viewport.center.x().clone() - image_offset_x,
            viewport.center.y().clone() - image_offset_y,
        );

        return Viewport::new(new_center, new_zoom);
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
    let new_view_width_unscaled = natural_bounds.width() / new_zoom;
    let new_view_height_unscaled = natural_bounds.height() / new_zoom;

    // Adjust for canvas aspect ratio
    let canvas_aspect = calculate_aspect_ratio(canvas_width, canvas_height);
    let (new_view_width, new_view_height) = if canvas_aspect > 1.0 {
        (
            new_view_height_unscaled.clone() * canvas_aspect,
            new_view_height_unscaled,
        )
    } else {
        (
            new_view_width_unscaled.clone(),
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

    let new_viewport_center = Point::new(
        image_at_original_center.x().clone() + new_view_width.clone() / 2.0
            - new_view_width * (new_center_px / canvas_width as f64),
        image_at_original_center.y().clone() + new_view_height.clone() / 2.0
            - new_view_height * (new_center_py / canvas_height as f64),
    );

    Viewport::new(new_viewport_center, new_zoom)
}

pub fn calculate_visible_bounds<T>(
    viewport: &Viewport<T>,
    natural_bounds: &Rect<T>,
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
    let natural_width = natural_bounds.width();
    let natural_height = natural_bounds.height();

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
        Point::new(
            viewport.center.x().clone() - half_width.clone(),
            viewport.center.y().clone() - half_height.clone(),
        ),
        Point::new(
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
) -> Point<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Mul<f64, Output = T>,
{
    let bounds_width = target_rect.width();
    let bounds_height = target_rect.height();

    Point::new(
        target_rect.min.x().clone() + bounds_width * (pixel_x / canvas_width as f64),
        target_rect.min.y().clone() + bounds_height * (pixel_y / canvas_height as f64),
    )
}

pub fn image_to_pixel<T>(
    image: &Point<T>,
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
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

        // Landscape canvas: 1600x900 (aspect ratio ~1.78)
        let bounds = calculate_visible_bounds(&viewport, &natural_bounds, 1600, 900);

        // At zoom 1.0, should show entire natural height (100 units)
        // Width should extend to maintain aspect ratio
        assert_eq!(bounds.height(), 100.0);
        assert!((bounds.width() - 177.77).abs() < 0.1); // 100 * 1.78
        assert_eq!(*bounds.min.y(), -50.0);
        assert_eq!(*bounds.max.y(), 50.0);
    }

    #[test]
    fn test_calculate_visible_bounds_portrait() {
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

        // Portrait canvas: 900x1600
        let bounds = calculate_visible_bounds(&viewport, &natural_bounds, 900, 1600);

        // At zoom 1.0, should show entire natural width (100 units)
        // Height should extend to maintain aspect ratio
        assert_eq!(bounds.width(), 100.0);
        assert!((bounds.height() - 177.77).abs() < 0.1);
        assert_eq!(*bounds.min.x(), -50.0);
        assert_eq!(*bounds.max.x(), 50.0);
    }

    #[test]
    fn test_calculate_visible_bounds_zoom() {
        let viewport = Viewport::new(Point::new(0.0, 0.0), 2.0); // 2x zoom
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

        // Square canvas
        let bounds = calculate_visible_bounds(&viewport, &natural_bounds, 1000, 1000);

        // At zoom 2.0, should show half the natural area (50 units)
        assert_eq!(bounds.width(), 50.0);
        assert_eq!(bounds.height(), 50.0);
    }

    #[test]
    fn test_pixel_to_image_center() {
        let bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
        let image = pixel_to_image(500.0, 500.0, &bounds, 1000, 1000);

        assert_eq!(*image.x(), 0.0);
        assert_eq!(*image.y(), 0.0);
    }

    #[test]
    fn test_pixel_to_image_corners() {
        let bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

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
        let bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
        let image = Point::new(0.0, 0.0);
        let (px, py) = image_to_pixel(&image, &bounds, 1000, 1000);

        assert_eq!(px, 500.0);
        assert_eq!(py, 500.0);
    }

    #[test]
    fn test_round_trip_pixel_image_pixel() {
        let bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
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
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);

        // Pan right 10 image units
        let new_viewport = pan_viewport(&viewport, 10.0, 0.0);

        assert_eq!(*new_viewport.center.x(), 10.0);
        assert_eq!(*new_viewport.center.y(), 0.0);
        assert_eq!(new_viewport.zoom, 1.0);
    }

    #[test]
    fn test_pan_viewport_from_offset_position() {
        let viewport = Viewport::new(Point::new(20.0, -10.0), 1.0);

        // Pan left 5 units, down 3 units
        let new_viewport = pan_viewport(&viewport, -5.0, 3.0);

        assert_eq!(*new_viewport.center.x(), 15.0);
        assert_eq!(*new_viewport.center.y(), -7.0);
        assert_eq!(new_viewport.zoom, 1.0);
    }

    #[test]
    fn test_zoom_viewport_at_center_point() {
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

        let canvas_width = 800;
        let canvas_height = 600;

        // Zoom 2x at canvas center
        let zoom_point_x = canvas_width as f64 / 2.0;
        let zoom_point_y = canvas_height as f64 / 2.0;

        let new_viewport = zoom_viewport_at_point(
            &viewport,
            &natural_bounds,
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
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

        let canvas_width = 800;
        let canvas_height = 600;

        // Zoom 2x at top-left corner
        let new_viewport = zoom_viewport_at_point(
            &viewport,
            &natural_bounds,
            2.0,
            0.0,
            0.0,
            canvas_width,
            canvas_height,
        );

        // Center should move toward top-left
        assert!(*new_viewport.center.x() < 0.0);
        assert!(*new_viewport.center.y() < 0.0);
        assert_eq!(new_viewport.zoom, 2.0);
    }

    #[test]
    fn test_zoom_viewport_out() {
        let viewport = Viewport::new(Point::new(0.0, 0.0), 2.0);
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

        let canvas_width = 800;
        let canvas_height = 600;

        // Zoom out 0.5x at center
        let zoom_point_x = canvas_width as f64 / 2.0;
        let zoom_point_y = canvas_height as f64 / 2.0;

        let new_viewport = zoom_viewport_at_point(
            &viewport,
            &natural_bounds,
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
        use crate::TransformResult;

        // Start at origin
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

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

        let new_viewport = apply_pixel_transform_to_viewport(
            &viewport,
            &natural_bounds,
            &result,
            canvas_width,
            canvas_height,
        );

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
        let new_bounds =
            calculate_visible_bounds(&new_viewport, &natural_bounds, canvas_width, canvas_height);
        let zoom_point_in_new_viewport_x =
            *new_bounds.min.x() + (zoom_point_x / canvas_width as f64) * new_bounds.width();

        // This should equal where the zoom point was in the original (dragged) viewport
        let original_dragged_center_x = -(drag_offset / canvas_width as f64)
            * calculate_visible_bounds(&viewport, &natural_bounds, canvas_width, canvas_height)
                .width();
        let original_bounds_at_drag = Rect::new(
            Point::new(original_dragged_center_x - 66.67, -50.0),
            Point::new(original_dragged_center_x + 66.67, 50.0),
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
        use crate::TransformResult;

        // Start at origin
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

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

        let new_viewport = apply_pixel_transform_to_viewport(
            &viewport,
            &natural_bounds,
            &result,
            canvas_width,
            canvas_height,
        );

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
        use crate::TransformResult;

        // This test verifies that when dragging + zooming in a single interaction,
        // the zoom operation keeps the point under the mouse fixed.
        //
        // Key insight: After dragging, the image content under the mouse has changed.
        // When we then zoom, we want to keep THAT content (post-drag) fixed, not the
        // original content that was there before the drag.

        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));

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
        let original_bounds =
            calculate_visible_bounds(&viewport, &natural_bounds, canvas_width, canvas_height);
        let drag_in_image_units = (drag_offset / canvas_width as f64) * original_bounds.width();

        // After dragging right, we're looking left, so subtract
        let intermediate_center_x = *viewport.center.x() - drag_in_image_units;
        let intermediate_viewport = Viewport::new(
            Point::new(intermediate_center_x, *viewport.center.y()),
            viewport.zoom,
        );

        let intermediate_bounds = calculate_visible_bounds(
            &intermediate_viewport,
            &natural_bounds,
            canvas_width,
            canvas_height,
        );
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
        let new_viewport = apply_pixel_transform_to_viewport(
            &viewport,
            &natural_bounds,
            &result,
            canvas_width,
            canvas_height,
        );

        println!(
            "New viewport center: ({}, {})",
            new_viewport.center.x(),
            new_viewport.center.y()
        );
        println!("New viewport zoom: {}", new_viewport.zoom);

        // Get the image point that's under the mouse AFTER the complete transformation
        let new_bounds =
            calculate_visible_bounds(&new_viewport, &natural_bounds, canvas_width, canvas_height);
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

/// Comprehensive test suite for apply_pixel_transform_to_viewport
///
/// Tests the critical function that translates pixel-space transformations
/// (from user interactions) into viewport-space coordinates (for rendering).
///
/// The golden invariant: After applying a TransformResult to a viewport,
/// rendering the new viewport should produce the same visual output as if
/// we had applied the pixel transformation matrix to the original render.
#[cfg(test)]
mod apply_pixel_transform_tests {
    use super::*;
    use crate::TransformResult;

    // Helper: Create TransformResult from simpler parameters
    fn create_transform_result(
        offset_x: f64,
        offset_y: f64,
        zoom_factor: f64,
        canvas_width: u32,
        canvas_height: u32,
    ) -> TransformResult {
        // Convert center-relative to absolute for matrix
        let canvas_center_x = canvas_width as f64 / 2.0;
        let canvas_center_y = canvas_height as f64 / 2.0;
        let absolute_offset_x = offset_x + canvas_center_x * (1.0 - zoom_factor);
        let absolute_offset_y = offset_y + canvas_center_y * (1.0 - zoom_factor);

        TransformResult {
            offset_x,
            offset_y,
            zoom_factor,
            matrix: [
                [zoom_factor, 0.0, absolute_offset_x],
                [0.0, zoom_factor, absolute_offset_y],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    // Helper: Verify that an image point appears at expected pixel location
    fn verify_image_point_at_pixel(
        image_point: &Point<f64>,
        expected_pixel: (f64, f64),
        viewport: &Viewport<f64>,
        natural_bounds: &Rect<f64>,
        canvas_width: u32,
        canvas_height: u32,
    ) {
        let bounds =
            calculate_visible_bounds(viewport, natural_bounds, canvas_width, canvas_height);
        let actual_pixel = image_to_pixel(image_point, &bounds, canvas_width, canvas_height);

        assert!(
            (actual_pixel.0 - expected_pixel.0).abs() < 0.1
                && (actual_pixel.1 - expected_pixel.1).abs() < 0.1,
            "Image point {:?} should appear at pixel {:?}, but appears at {:?}",
            image_point,
            expected_pixel,
            actual_pixel
        );
    }

    // Helper: Verify zoom factor
    fn verify_zoom(viewport: &Viewport<f64>, expected_zoom: f64, tolerance: f64) {
        assert!(
            (viewport.zoom - expected_zoom).abs() < tolerance,
            "Expected zoom {}, got {}",
            expected_zoom,
            viewport.zoom
        );
    }

    mod pan_tests {
        use super::*;

        #[test]
        fn test_pan_right() {
            // Start at origin
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // User drags right 200px (looking left in image space)
            let transform = create_transform_result(200.0, 0.0, 1.0, canvas_width, canvas_height);

            // Apply transformation
            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Verify zoom unchanged
            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Verify viewport moved left (negative x)
            // Dragging right means looking left, so viewport center should move left
            assert!(
                *new_viewport.center.x() < 0.0,
                "After dragging right, viewport should move left (negative x), got x={}",
                new_viewport.center.x()
            );

            // The original center point should now appear 200px to the right
            let original_center = Point::new(0.0, 0.0);
            let canvas_center_x = canvas_width as f64 / 2.0;
            let canvas_center_y = canvas_height as f64 / 2.0;
            verify_image_point_at_pixel(
                &original_center,
                (canvas_center_x + 200.0, canvas_center_y),
                &new_viewport,
                &natural_bounds,
                canvas_width,
                canvas_height,
            );
        }

        #[test]
        fn test_pan_left() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // User drags left 200px (looking right in image space)
            let transform = create_transform_result(-200.0, 0.0, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Viewport should move right (positive x)
            assert!(
                *new_viewport.center.x() > 0.0,
                "After dragging left, viewport should move right (positive x), got x={}",
                new_viewport.center.x()
            );
        }

        #[test]
        fn test_pan_down() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // User drags down 150px (looking up in image space)
            let transform = create_transform_result(0.0, 150.0, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Viewport should move up (negative y)
            assert!(
                *new_viewport.center.y() < 0.0,
                "After dragging down, viewport should move up (negative y), got y={}",
                new_viewport.center.y()
            );
        }

        #[test]
        fn test_pan_up() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // User drags up 150px (looking down in image space)
            let transform = create_transform_result(0.0, -150.0, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Viewport should move down (positive y)
            assert!(
                *new_viewport.center.y() > 0.0,
                "After dragging up, viewport should move down (positive y), got y={}",
                new_viewport.center.y()
            );
        }

        #[test]
        fn test_pan_diagonal() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Drag diagonally: right 100px, down 100px
            let transform = create_transform_result(100.0, 100.0, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Both coordinates should move opposite to drag direction
            assert!(
                *new_viewport.center.x() < 0.0,
                "x should be negative after dragging right"
            );
            assert!(
                *new_viewport.center.y() < 0.0,
                "y should be negative after dragging down"
            );
        }

        #[test]
        fn test_pan_large_offset() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Large pan beyond canvas size
            let transform = create_transform_result(2000.0, 0.0, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Should handle large offsets correctly
            assert!(
                new_viewport.center.x().is_finite(),
                "Viewport center should remain finite"
            );
        }

        #[test]
        fn test_pan_small_offset() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Sub-pixel pan
            let transform = create_transform_result(0.5, 0.3, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Should still apply transformation, even if small
            assert!(
                *new_viewport.center.x() != 0.0 || *new_viewport.center.y() != 0.0,
                "Small pan should still move viewport"
            );
        }

        #[test]
        fn test_pan_from_non_origin() {
            // Start from an offset position
            let viewport = Viewport::new(Point::new(20.0, -15.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Pan right 100px
            let transform = create_transform_result(100.0, 0.0, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Should move relative to starting position
            assert!(
                *new_viewport.center.x() < *viewport.center.x(),
                "After dragging right, x should decrease from starting position"
            );
            assert_eq!(
                *new_viewport.center.y(),
                *viewport.center.y(),
                "Y should remain unchanged"
            );
        }
    }

    mod zoom_tests {
        use super::*;

        #[test]
        fn test_zoom_in_at_center() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Zoom 2x at canvas center
            let transform = create_transform_result(0.0, 0.0, 2.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Verify zoom doubled
            verify_zoom(&new_viewport, 2.0, 0.0001);

            // Center should remain at origin when zooming at center
            assert!(
                (*new_viewport.center.x()).abs() < 0.1 && (*new_viewport.center.y()).abs() < 0.1,
                "Center should stay near origin when zooming at canvas center, got ({}, {})",
                new_viewport.center.x(),
                new_viewport.center.y()
            );
        }

        #[test]
        fn test_zoom_out_at_center() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 2.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Zoom out 0.5x at center
            let transform = create_transform_result(0.0, 0.0, 0.5, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Verify zoom: 2.0 * 0.5 = 1.0
            verify_zoom(&new_viewport, 1.0, 0.0001);

            // Center should remain near origin
            assert!(
                (*new_viewport.center.x()).abs() < 0.1,
                "Center x should stay near origin, got {}",
                new_viewport.center.x()
            );
        }

        #[test]
        fn test_zoom_in_at_corner() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Zoom 2x at top-left corner (pixel 0, 0)
            // The image content at pixel (0, 0) should remain at pixel (0, 0)
            let canvas_center_x = canvas_width as f64 / 2.0;
            let canvas_center_y = canvas_height as f64 / 2.0;

            // For zoom at corner: offset = corner * (1 - zoom)
            let zoom_factor = 2.0;
            let corner_x = 0.0;
            let corner_y = 0.0;
            let offset_x = corner_x * (1.0 - zoom_factor) - canvas_center_x * (1.0 - zoom_factor);
            let offset_y = corner_y * (1.0 - zoom_factor) - canvas_center_y * (1.0 - zoom_factor);

            let transform = create_transform_result(
                offset_x,
                offset_y,
                zoom_factor,
                canvas_width,
                canvas_height,
            );

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 2.0, 0.0001);

            // Viewport center should shift (it was centered, now corner is fixed point)
            // When zooming at corner, center moves
            assert!(
                *new_viewport.center.x() < 0.0,
                "Center should move left when zooming at top-left corner"
            );
            assert!(
                *new_viewport.center.y() < 0.0,
                "Center should move up when zooming at top-left corner"
            );
        }

        #[test]
        fn test_zoom_large_factor() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Large zoom 100x at center
            let transform = create_transform_result(0.0, 0.0, 100.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 100.0, 0.01);

            // Should handle extreme zoom
            assert!(
                new_viewport.center.x().is_finite() && new_viewport.center.y().is_finite(),
                "Viewport center should remain finite"
            );
        }

        #[test]
        fn test_zoom_small_factor() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Small zoom 1.1x (just 10% zoom)
            let transform = create_transform_result(0.0, 0.0, 1.1, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.1, 0.0001);
        }

        #[test]
        fn test_deep_zoom_scenario() {
            // Start at already deep zoom
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1000.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Zoom 2x more
            let transform = create_transform_result(0.0, 0.0, 2.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 2000.0, 0.1);
        }
    }

    mod combined_tests {
        use super::*;

        #[test]
        fn test_drag_right_then_zoom_in() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Simulate: drag right 200px, then zoom 2x at the dragged position
            // After dragging right 200px, the center content is now at pixel (600, 300)
            // We want to zoom there
            let drag_offset = 200.0;
            let zoom_factor = 2.0;
            let zoom_point_x = 600.0; // Where content moved to
            let zoom_point_y = 300.0;

            // Formula from use_canvas_interaction.rs
            let absolute_offset_x = drag_offset * zoom_factor + zoom_point_x * (1.0 - zoom_factor);
            let absolute_offset_y = 0.0 * zoom_factor + zoom_point_y * (1.0 - zoom_factor);

            let canvas_center_x = canvas_width as f64 / 2.0;
            let canvas_center_y = canvas_height as f64 / 2.0;
            let offset_x = absolute_offset_x - canvas_center_x * (1.0 - zoom_factor);
            let offset_y = absolute_offset_y - canvas_center_y * (1.0 - zoom_factor);

            let transform = TransformResult {
                offset_x,
                offset_y,
                zoom_factor,
                matrix: [
                    [zoom_factor, 0.0, absolute_offset_x],
                    [0.0, zoom_factor, absolute_offset_y],
                    [0.0, 0.0, 1.0],
                ],
            };

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 2.0, 0.0001);

            // The zoom point should remain fixed
            // Calculate where zoom point should be in intermediate (dragged) state
            let original_bounds =
                calculate_visible_bounds(&viewport, &natural_bounds, canvas_width, canvas_height);
            let drag_in_image = (drag_offset / canvas_width as f64) * original_bounds.width();
            let intermediate_center_x = *viewport.center.x() - drag_in_image;

            let intermediate_viewport =
                Viewport::new(Point::new(intermediate_center_x, 0.0), viewport.zoom);
            let intermediate_bounds = calculate_visible_bounds(
                &intermediate_viewport,
                &natural_bounds,
                canvas_width,
                canvas_height,
            );
            let image_at_zoom_point = pixel_to_image(
                zoom_point_x,
                zoom_point_y,
                &intermediate_bounds,
                canvas_width,
                canvas_height,
            );

            // After full transformation, that image point should still be at zoom_point
            verify_image_point_at_pixel(
                &image_at_zoom_point,
                (zoom_point_x, zoom_point_y),
                &new_viewport,
                &natural_bounds,
                canvas_width,
                canvas_height,
            );
        }

        #[test]
        fn test_drag_left_then_zoom_out() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Drag left 150px, then zoom out 0.5x
            let drag_offset = -150.0;
            let zoom_factor = 0.5;
            let zoom_point_x = 400.0; // Canvas center
            let zoom_point_y = 300.0;

            let absolute_offset_x = drag_offset * zoom_factor + zoom_point_x * (1.0 - zoom_factor);
            let absolute_offset_y = 0.0 * zoom_factor + zoom_point_y * (1.0 - zoom_factor);

            let canvas_center_x = canvas_width as f64 / 2.0;
            let canvas_center_y = canvas_height as f64 / 2.0;
            let offset_x = absolute_offset_x - canvas_center_x * (1.0 - zoom_factor);
            let offset_y = absolute_offset_y - canvas_center_y * (1.0 - zoom_factor);

            let transform = TransformResult {
                offset_x,
                offset_y,
                zoom_factor,
                matrix: [
                    [zoom_factor, 0.0, absolute_offset_x],
                    [0.0, zoom_factor, absolute_offset_y],
                    [0.0, 0.0, 1.0],
                ],
            };

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 0.5, 0.0001);

            // Should be valid viewport
            assert!(
                new_viewport.center.x().is_finite() && new_viewport.center.y().is_finite(),
                "Viewport center should be finite"
            );
        }

        #[test]
        fn test_zoom_then_pan() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // First zoom 2x at center
            let transform1 = create_transform_result(0.0, 0.0, 2.0, canvas_width, canvas_height);
            let viewport_after_zoom = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform1,
                canvas_width,
                canvas_height,
            );

            // Then pan right 100px
            let transform2 = create_transform_result(100.0, 0.0, 1.0, canvas_width, canvas_height);
            let final_viewport = apply_pixel_transform_to_viewport(
                &viewport_after_zoom,
                &natural_bounds,
                &transform2,
                canvas_width,
                canvas_height,
            );

            // Zoom should still be 2.0
            verify_zoom(&final_viewport, 2.0, 0.0001);

            // Should have moved left from zoomed position
            assert!(
                *final_viewport.center.x() < *viewport_after_zoom.center.x(),
                "After panning right, center should move left"
            );
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_transform_at_exact_corner() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Zoom at exact corner (0, 0)
            let zoom_factor = 2.0;
            let canvas_center_x = canvas_width as f64 / 2.0;
            let canvas_center_y = canvas_height as f64 / 2.0;
            let offset_x = -canvas_center_x * (1.0 - zoom_factor);
            let offset_y = -canvas_center_y * (1.0 - zoom_factor);

            let transform = create_transform_result(
                offset_x,
                offset_y,
                zoom_factor,
                canvas_width,
                canvas_height,
            );

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Should not crash or produce NaN
            assert!(
                new_viewport.center.x().is_finite() && new_viewport.center.y().is_finite(),
                "Viewport center should be finite"
            );
            verify_zoom(&new_viewport, 2.0, 0.0001);
        }

        #[test]
        fn test_extreme_wide_canvas() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 3840;
            let canvas_height = 600;

            // Pan and zoom on ultra-wide canvas
            let transform = create_transform_result(200.0, 0.0, 1.5, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.5, 0.0001);
            assert!(
                new_viewport.center.x().is_finite(),
                "Should handle extreme aspect ratio"
            );
        }

        #[test]
        fn test_extreme_tall_canvas() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 600;
            let canvas_height = 3840;

            let transform = create_transform_result(0.0, 200.0, 1.5, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            verify_zoom(&new_viewport, 1.5, 0.0001);
            assert!(
                new_viewport.center.y().is_finite(),
                "Should handle extreme aspect ratio"
            );
        }

        #[test]
        fn test_small_canvas() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 10;
            let canvas_height = 10;

            let transform = create_transform_result(5.0, 5.0, 2.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Should not have numerical instability
            assert!(
                new_viewport.center.x().is_finite() && new_viewport.center.y().is_finite(),
                "Should handle tiny canvas"
            );
            verify_zoom(&new_viewport, 2.0, 0.0001);
        }

        #[test]
        fn test_very_large_canvas() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 10000;
            let canvas_height = 10000;

            let transform =
                create_transform_result(1000.0, 1000.0, 2.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Should not overflow
            assert!(
                new_viewport.center.x().is_finite() && new_viewport.center.y().is_finite(),
                "Should handle large canvas"
            );
            verify_zoom(&new_viewport, 2.0, 0.0001);
        }

        #[test]
        fn test_viewport_at_extreme_zoom() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1e10);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            let transform = create_transform_result(0.0, 0.0, 2.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Should handle extreme starting zoom
            assert!(new_viewport.zoom.is_finite(), "Zoom should remain finite");
            verify_zoom(&new_viewport, 2e10, 1e9);
        }
    }

    mod precision_tests {
        use super::*;

        #[test]
        fn test_very_small_offsets() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Sub-pixel offsets
            let transform = create_transform_result(0.01, 0.02, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Should maintain precision
            assert!(
                *new_viewport.center.x() != 0.0 || *new_viewport.center.y() != 0.0,
                "Should not lose precision on tiny offsets"
            );
        }

        #[test]
        fn test_commutative_pans() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Two separate pans
            let transform1 = create_transform_result(100.0, 0.0, 1.0, canvas_width, canvas_height);
            let viewport1 = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform1,
                canvas_width,
                canvas_height,
            );

            let transform2 = create_transform_result(50.0, 0.0, 1.0, canvas_width, canvas_height);
            let viewport2 = apply_pixel_transform_to_viewport(
                &viewport1,
                &natural_bounds,
                &transform2,
                canvas_width,
                canvas_height,
            );

            // Single combined pan
            let transform_combined =
                create_transform_result(150.0, 0.0, 1.0, canvas_width, canvas_height);
            let viewport_combined = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform_combined,
                canvas_width,
                canvas_height,
            );

            // Results should be similar (within floating point tolerance)
            assert!(
                (*viewport2.center.x() - *viewport_combined.center.x()).abs() < 0.1,
                "Sequential pans should match combined pan: got {} vs {}",
                viewport2.center.x(),
                viewport_combined.center.x()
            );
        }

        #[test]
        fn test_floating_point_edge_values() {
            let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
            let natural_bounds = Rect::new(Point::new(-50.0, -50.0), Point::new(50.0, 50.0));
            let canvas_width = 800;
            let canvas_height = 600;

            // Very small offset
            let transform = create_transform_result(1e-10, 0.0, 1.0, canvas_width, canvas_height);

            let new_viewport = apply_pixel_transform_to_viewport(
                &viewport,
                &natural_bounds,
                &transform,
                canvas_width,
                canvas_height,
            );

            // Should not produce NaN or Infinity
            assert!(
                new_viewport.center.x().is_finite() && new_viewport.center.y().is_finite(),
                "Should handle very small values"
            );
            assert!(!new_viewport.zoom.is_nan(), "Zoom should not be NaN");
        }
    }
}
