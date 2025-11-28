use crate::precision::calculate_precision_bits;
use crate::{BigFloat, Viewport};
use serde::{Deserialize, Serialize};

/// Transformation result returned when user interaction ends
///
/// Contains both discrete values and a pre-computed affine transformation matrix.
/// Offsets are **center-relative** for intuitive interpretation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PixelTransform {
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

impl PixelTransform {
    /// Create a new PixelTransform with computed matrix
    pub fn new(
        offset_x: f64,
        offset_y: f64,
        zoom_factor: f64,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Self {
        // Convert center-relative to absolute for matrix
        let canvas_center_x = canvas_width as f64 / 2.0;
        let canvas_center_y = canvas_height as f64 / 2.0;
        let absolute_offset_x = offset_x + canvas_center_x * (1.0 - zoom_factor);
        let absolute_offset_y = offset_y + canvas_center_y * (1.0 - zoom_factor);

        Self {
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

    /// Create identity transform (no change)
    pub fn identity() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom_factor: 1.0,
            matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }
}

/// A 2D affine transformation in pixel/canvas space
#[derive(Debug, Clone, PartialEq)]
pub enum AffinePrimitive {
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PixelMat3 {
    /// Row-major order: [[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]]
    pub data: [[f64; 3]; 3],
}

impl PixelMat3 {
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

    /// Creates a uniform scale matrix around the origin
    pub fn scale(factor: f64) -> Self {
        Self::scale_around(factor, 0.0, 0.0)
    }

    /// Multiplies this matrix by another (self × other)
    ///
    /// For transformations, left-multiplying applies the transformation:
    /// To compose transformations [T1, T2, T3], compute: T3 × T2 × T1
    pub fn multiply(&self, other: &PixelMat3) -> Self {
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

    /// Transform a point (x, y) using this matrix
    pub fn transform_point(&self, x: f64, y: f64) -> (f64, f64) {
        let new_x = x * self.data[0][0] + y * self.data[0][1] + self.data[0][2];
        let new_y = x * self.data[1][0] + y * self.data[1][1] + self.data[1][2];
        (new_x, new_y)
    }

    /// Returns the raw 3x3 array (for Canvas2D API compatibility)
    pub fn to_array(&self) -> [[f64; 3]; 3] {
        self.data
    }

    /// Creates a matrix combining scale (around optional center) and translation
    ///
    /// - `scale`: Scale factor (1.0 = no scaling)
    /// - `offset`: Translation (dx, dy)
    /// - `center`: If provided, scale is centered on this point; otherwise scales around origin
    pub fn from_scale_and_offset(
        scale: f64,
        offset: (f64, f64),
        center: Option<(f64, f64)>,
    ) -> Self {
        let scale_matrix = if let Some((cx, cy)) = center {
            PixelMat3::scale_around(scale, cx, cy)
        } else {
            PixelMat3::scale(scale)
        };

        let translation_matrix = PixelMat3::translation(offset.0, offset.1);

        // Apply scale first, then translate
        translation_matrix.multiply(&scale_matrix)
    }
}

/// Composes a sequence of 2D affine transformations into a single transformation matrix
///
/// Transformations are applied in order: the first transformation in the sequence
/// is applied first to any point transformed by the resulting matrix.
///
/// # Example
/// ```ignore
/// use fractalwonder_core::transforms::{AffinePrimitive, compose_affine_transformations};
///
/// // Translate right 200px, then scale 0.5x around point (200, 0)
/// let transforms = vec![
///     AffinePrimitive::Translate { dx: 200.0, dy: 0.0 },
///     AffinePrimitive::Scale { factor: 0.5, center_x: 200.0, center_y: 0.0 },
/// ];
///
/// let matrix = compose_affine_transformations(transforms);
/// // Point (0, 0) transforms to (200, 0): moved right 200px, then stays there during scaling
/// ```
pub fn compose_affine_transformations(
    transforms: impl IntoIterator<Item = AffinePrimitive>,
) -> PixelMat3 {
    let mut result = PixelMat3::identity();

    for transform in transforms {
        let matrix = match transform {
            AffinePrimitive::Translate { dx, dy } => PixelMat3::translation(dx, dy),
            AffinePrimitive::Scale {
                factor,
                center_x,
                center_y,
            } => PixelMat3::scale_around(factor, center_x, center_y),
        };

        // Left-multiply: result = matrix × result
        // This ensures transformations apply in the correct order
        result = matrix.multiply(&result);
    }

    result
}

/// Convert pixel coordinates to fractal coordinates
///
/// Uses BigFloat arithmetic throughout to preserve precision.
/// The viewport directly specifies the visible region (width, height) in fractal space.
pub fn pixel_to_fractal(
    pixel_x: f64,
    pixel_y: f64,
    viewport: &Viewport,
    canvas_size: (u32, u32),
    precision_bits: usize,
) -> (BigFloat, BigFloat) {
    let (canvas_width, canvas_height) = canvas_size;

    // Normalized pixel coordinates [-0.5, 0.5]
    let norm_x = BigFloat::with_precision(pixel_x, precision_bits)
        .div(&BigFloat::with_precision(
            canvas_width as f64,
            precision_bits,
        ))
        .sub(&BigFloat::with_precision(0.5, precision_bits));

    let norm_y = BigFloat::with_precision(pixel_y, precision_bits)
        .div(&BigFloat::with_precision(
            canvas_height as f64,
            precision_bits,
        ))
        .sub(&BigFloat::with_precision(0.5, precision_bits));

    // fractal_x = center_x + norm_x * width
    let fractal_x = viewport.center.0.add(&norm_x.mul(&viewport.width));

    // fractal_y = center_y + norm_y * height
    let fractal_y = viewport.center.1.add(&norm_y.mul(&viewport.height));

    (fractal_x, fractal_y)
}

/// Convert fractal coordinates to pixel coordinates
///
/// Note: This may lose precision when converting to f64 for pixel display.
/// The viewport directly specifies the visible region (width, height) in fractal space.
pub fn fractal_to_pixel(
    fractal_x: &BigFloat,
    fractal_y: &BigFloat,
    viewport: &Viewport,
    canvas_size: (u32, u32),
) -> (f64, f64) {
    let (canvas_width, canvas_height) = canvas_size;

    // norm_x = (fractal_x - center_x) / width
    let norm_x = fractal_x.sub(&viewport.center.0).div(&viewport.width);

    // norm_y = (fractal_y - center_y) / height
    let norm_y = fractal_y.sub(&viewport.center.1).div(&viewport.height);

    // pixel_x = (norm_x + 0.5) * canvas_width
    let pixel_x = (norm_x.to_f64() + 0.5) * canvas_width as f64;

    // pixel_y = (norm_y + 0.5) * canvas_height
    let pixel_y = (norm_y.to_f64() + 0.5) * canvas_height as f64;

    (pixel_x, pixel_y)
}

/// Apply user interaction transform to viewport
///
/// PixelTransform contains RELATIVE zoom_factor (2.0 = zoom in 2x from current).
/// Uses BigFloat arithmetic to preserve precision at extreme depths.
///
/// The transform's offset is **center-relative**: (0, 0) means the transformation
/// is centered at the canvas center. This function converts to absolute coordinates
/// for proper zoom-point handling.
///
/// Precision is calculated automatically based on the resulting viewport dimensions
/// and canvas size. This ensures the returned viewport always has adequate precision
/// for pixel-accurate rendering.
///
/// - zoom_factor > 1: zooming in, width/height shrink
/// - zoom_factor < 1: zooming out, width/height grow
pub fn apply_pixel_transform_to_viewport(
    viewport: &Viewport,
    transform: &PixelTransform,
    canvas_size: (u32, u32),
) -> Viewport {
    let (canvas_width, canvas_height) = canvas_size;
    let canvas_center_x = canvas_width as f64 / 2.0;
    let canvas_center_y = canvas_height as f64 / 2.0;

    // Convert center-relative offset to absolute offset (pixel space)
    let absolute_offset_x = transform.offset_x + canvas_center_x * (1.0 - transform.zoom_factor);
    let absolute_offset_y = transform.offset_y + canvas_center_y * (1.0 - transform.zoom_factor);

    // Step 1: Estimate new dimensions at current precision to calculate required precision
    let current_precision = viewport.precision_bits();
    let zoom_factor_estimate = BigFloat::with_precision(transform.zoom_factor, current_precision);
    let estimated_width = viewport.width.div(&zoom_factor_estimate);
    let estimated_height = viewport.height.div(&zoom_factor_estimate);

    // Step 2: Create temporary viewport to calculate required precision
    let temp_viewport = Viewport::with_bigfloat(
        viewport.center.0.clone(),
        viewport.center.1.clone(),
        estimated_width,
        estimated_height,
    );
    let required_precision = calculate_precision_bits(&temp_viewport, canvas_size);

    // Step 4: Calculate new viewport dimensions at final precision
    let zoom_factor_bf = BigFloat::with_precision(transform.zoom_factor, required_precision);
    let new_width = viewport.width.div(&zoom_factor_bf);
    let new_height = viewport.height.div(&zoom_factor_bf);

    // Special case: pure translation (zoom ≈ 1.0)
    if (transform.zoom_factor - 1.0).abs() < 1e-10 {
        let offset_x_norm = BigFloat::with_precision(absolute_offset_x, required_precision).div(
            &BigFloat::with_precision(canvas_width as f64, required_precision),
        );
        let offset_y_norm = BigFloat::with_precision(absolute_offset_y, required_precision).div(
            &BigFloat::with_precision(canvas_height as f64, required_precision),
        );

        let dx = offset_x_norm.mul(&viewport.width);
        let dy = offset_y_norm.mul(&viewport.height);

        let new_center = (viewport.center.0.sub(&dx), viewport.center.1.sub(&dy));

        // Ensure output viewport has the required precision (not inherited from old viewport)
        return Viewport {
            center: new_center,
            width: new_width,
            height: new_height,
        }
        .to_precision(required_precision);
    }

    // General case: transformation with zoom
    let image_at_center = pixel_to_fractal(
        canvas_center_x,
        canvas_center_y,
        viewport,
        canvas_size,
        required_precision,
    );

    let new_center_px = canvas_center_x * transform.zoom_factor + absolute_offset_x;
    let new_center_py = canvas_center_y * transform.zoom_factor + absolute_offset_y;

    let pixel_ratio_x =
        BigFloat::with_precision(new_center_px / canvas_width as f64, required_precision)
            .sub(&BigFloat::with_precision(0.5, required_precision));
    let pixel_ratio_y =
        BigFloat::with_precision(new_center_py / canvas_height as f64, required_precision)
            .sub(&BigFloat::with_precision(0.5, required_precision));

    let new_center_x = image_at_center.0.sub(&pixel_ratio_x.mul(&new_width));
    let new_center_y = image_at_center.1.sub(&pixel_ratio_y.mul(&new_height));

    // Ensure output viewport has the required precision (not inherited from old viewport)
    Viewport {
        center: (new_center_x, new_center_y),
        width: new_width,
        height: new_height,
    }
    .to_precision(required_precision)
}

pub fn calculate_aspect_ratio(canvas_width: u32, canvas_height: u32) -> f64 {
    canvas_width as f64 / canvas_height as f64
}

/// Expand viewport to match canvas aspect ratio.
///
/// The viewport is expanded (never shrunk) so the natural bounds remain
/// fully visible regardless of canvas shape. Center stays fixed.
///
/// # Arguments
/// * `natural_viewport` - The viewport with natural fractal bounds
/// * `canvas_size` - Canvas dimensions in pixels (width, height)
///
/// # Returns
/// A new viewport with dimensions adjusted to match canvas aspect ratio.
pub fn fit_viewport_to_canvas(natural_viewport: &Viewport, canvas_size: (u32, u32)) -> Viewport {
    let canvas_aspect = canvas_size.0 as f64 / canvas_size.1 as f64;

    // Safe at any zoom: ratio of similar-magnitude values gives reasonable f64
    let viewport_aspect = natural_viewport
        .width
        .div(&natural_viewport.height)
        .to_f64();

    let precision = natural_viewport.precision_bits();

    if canvas_aspect > viewport_aspect {
        // Canvas wider than viewport: expand width
        // new_width = height * canvas_aspect
        let aspect = BigFloat::with_precision(canvas_aspect, precision);
        let new_width = natural_viewport.height.mul(&aspect);

        Viewport::with_bigfloat(
            natural_viewport.center.0.clone(),
            natural_viewport.center.1.clone(),
            new_width,
            natural_viewport.height.clone(),
        )
    } else {
        // Canvas taller than viewport: expand height
        // new_height = width / canvas_aspect
        let aspect = BigFloat::with_precision(canvas_aspect, precision);
        let new_height = natural_viewport.width.div(&aspect);

        Viewport::with_bigfloat(
            natural_viewport.center.0.clone(),
            natural_viewport.center.1.clone(),
            natural_viewport.width.clone(),
            new_height,
        )
    }
}

/// Calculate maximum iterations based on zoom exponent.
///
/// Uses formula: multiplier * zoom_exp^power
/// Clamped to [1000, 10_000_000] for practical bounds.
pub fn calculate_max_iterations(zoom_exponent: f64, multiplier: f64, power: f64) -> u32 {
    let iterations = multiplier * zoom_exponent.abs().max(1.0).powf(power);
    iterations.clamp(1000.0, 10_000_000.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // PixelMat3 Tests
    // ============================================================================

    #[test]
    fn test_mat3_identity() {
        let id = PixelMat3::identity();
        assert_eq!(id.data, [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
    }

    #[test]
    fn test_mat3_translation() {
        let t = PixelMat3::translation(200.0, 100.0);
        assert_eq!(
            t.data,
            [[1.0, 0.0, 200.0], [0.0, 1.0, 100.0], [0.0, 0.0, 1.0]]
        );

        // Transform point (0, 0) → should move to (200, 100)
        let (x, y) = t.transform_point(0.0, 0.0);
        assert_eq!(x, 200.0);
        assert_eq!(y, 100.0);
    }

    #[test]
    fn test_mat3_scale_around_origin() {
        let s = PixelMat3::scale_around(0.5, 0.0, 0.0);
        assert_eq!(s.data, [[0.5, 0.0, 0.0], [0.0, 0.5, 0.0], [0.0, 0.0, 1.0]]);
    }

    #[test]
    fn test_mat3_scale_around_point() {
        // Scale 0.5x around point (200, 0)
        let s = PixelMat3::scale_around(0.5, 200.0, 0.0);

        // Matrix should be: [[0.5, 0, 100], [0, 0.5, 0], [0, 0, 1]]
        // Because: cx(1-s) = 200(1-0.5) = 100
        assert_eq!(s.data[0][0], 0.5);
        assert_eq!(s.data[1][1], 0.5);
        assert_eq!(s.data[0][2], 100.0);
        assert_eq!(s.data[1][2], 0.0);

        // Point (200, 0) should stay at (200, 0) after scaling
        let (x, y) = s.transform_point(200.0, 0.0);
        assert_eq!(x, 200.0);
        assert_eq!(y, 0.0);

        // Point (0, 0) should move toward (200, 0)
        let (x, y) = s.transform_point(0.0, 0.0);
        assert_eq!(x, 100.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn test_mat3_multiply_identity() {
        let t = PixelMat3::translation(100.0, 50.0);
        let id = PixelMat3::identity();
        let result = t.multiply(&id);
        assert_eq!(result.data, t.data);
    }

    #[test]
    fn test_mat3_multiply_translations() {
        // Translate by (100, 0) then by (50, 0) = translate by (150, 0)
        let t1 = PixelMat3::translation(100.0, 0.0);
        let t2 = PixelMat3::translation(50.0, 0.0);
        let result = t2.multiply(&t1);

        // Transform point (0, 0)
        let (x, _y) = result.transform_point(0.0, 0.0);
        assert_eq!(x, 150.0);
    }

    // ============================================================================
    // compose_affine_transformations Tests
    // ============================================================================

    #[test]
    fn test_compose_single_translation() {
        let transforms = vec![AffinePrimitive::Translate { dx: 200.0, dy: 0.0 }];
        let matrix = compose_affine_transformations(transforms);

        assert_eq!(
            matrix.data,
            [[1.0, 0.0, 200.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn test_compose_single_scale() {
        let transforms = vec![AffinePrimitive::Scale {
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
        // translate(200, 0) then scale(0.5) around (200, 0)
        let transforms = vec![
            AffinePrimitive::Translate { dx: 200.0, dy: 0.0 },
            AffinePrimitive::Scale {
                factor: 0.5,
                center_x: 200.0,
                center_y: 0.0,
            },
        ];
        let matrix = compose_affine_transformations(transforms);

        // Point (0, 0) should end up at (200, 0)
        let (x, y) = matrix.transform_point(0.0, 0.0);
        assert!((x - 200.0).abs() < 0.0001, "Expected x=200, got x={}", x);
        assert!((y - 0.0).abs() < 0.0001, "Expected y=0, got y={}", y);
    }

    #[test]
    fn test_compose_empty_sequence() {
        let transforms: Vec<AffinePrimitive> = vec![];
        let matrix = compose_affine_transformations(transforms);
        assert_eq!(matrix.data, PixelMat3::identity().data);
    }

    #[test]
    fn test_compose_multiple_translations() {
        let transforms = vec![
            AffinePrimitive::Translate { dx: 100.0, dy: 0.0 },
            AffinePrimitive::Translate { dx: 50.0, dy: 0.0 },
            AffinePrimitive::Translate {
                dx: -20.0,
                dy: 30.0,
            },
        ];
        let matrix = compose_affine_transformations(transforms);

        // Transform point (0, 0) - should be at (130, 30)
        let (x, y) = matrix.transform_point(0.0, 0.0);
        assert!((x - 130.0).abs() < 0.0001);
        assert!((y - 30.0).abs() < 0.0001);
    }

    #[test]
    fn test_compose_scale_then_translate() {
        // Scale 0.5x around origin, then translate (100, 0)
        let transforms = vec![
            AffinePrimitive::Scale {
                factor: 0.5,
                center_x: 0.0,
                center_y: 0.0,
            },
            AffinePrimitive::Translate { dx: 100.0, dy: 0.0 },
        ];
        let matrix = compose_affine_transformations(transforms);

        // Point (0, 0) should be at (100, 0)
        let (x, y) = matrix.transform_point(0.0, 0.0);
        assert!((x - 100.0).abs() < 0.0001);
        assert!((y - 0.0).abs() < 0.0001);

        // Point (200, 0) should be at: scaled to (100, 0), then translated to (200, 0)
        let (x, _y) = matrix.transform_point(200.0, 0.0);
        assert!((x - 200.0).abs() < 0.0001);
    }

    // ============================================================================
    // PixelTransform Tests
    // ============================================================================

    #[test]
    fn test_pixel_transform_identity() {
        let t = PixelTransform::identity();
        assert_eq!(t.offset_x, 0.0);
        assert_eq!(t.offset_y, 0.0);
        assert_eq!(t.zoom_factor, 1.0);
        assert_eq!(
            t.matrix,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
    }

    #[test]
    fn test_pixel_transform_new() {
        let t = PixelTransform::new(100.0, 50.0, 2.0, 800, 600);
        assert_eq!(t.offset_x, 100.0);
        assert_eq!(t.offset_y, 50.0);
        assert_eq!(t.zoom_factor, 2.0);
        // Matrix should encode the transformation
        assert_eq!(t.matrix[0][0], 2.0); // scale x
        assert_eq!(t.matrix[1][1], 2.0); // scale y
    }

    // ============================================================================
    // pixel_to_fractal() Tests
    // ============================================================================

    #[test]
    fn pixel_to_fractal_center_maps_to_viewport_center() {
        // Canvas 800x600, viewport centered at (0,0) with width=4.0, height=3.0
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 128);
        let canvas_size = (800, 600);

        // Center pixel (400, 300) should map to fractal center (0, 0)
        let (fx, fy) = pixel_to_fractal(400.0, 300.0, &viewport, canvas_size, 128);

        assert_eq!(fx, viewport.center.0);
        assert_eq!(fy, viewport.center.1);
    }

    #[test]
    fn pixel_to_fractal_top_left() {
        // Viewport: center=(0,0), width=4.0, height=3.0, canvas=800x600
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 128);
        let canvas_size = (800, 600);

        // Top-left pixel (0, 0) should map to (-2, -1.5)
        let (fx, fy) = pixel_to_fractal(0.0, 0.0, &viewport, canvas_size, 128);

        let expected_x = BigFloat::with_precision(-2.0, 128);
        let expected_y = BigFloat::with_precision(-1.5, 128);

        assert_eq!(fx, expected_x);
        assert_eq!(fy, expected_y);
    }

    #[test]
    fn pixel_to_fractal_smaller_region() {
        // Half the visible region: width=2.0, height=1.5
        let viewport = Viewport::from_f64(0.0, 0.0, 2.0, 1.5, 128);
        let canvas_size = (800, 600);

        // Top-left pixel should map to (-1.0, -0.75)
        let (fx, fy) = pixel_to_fractal(0.0, 0.0, &viewport, canvas_size, 128);

        let expected_x = BigFloat::with_precision(-1.0, 128);
        let expected_y = BigFloat::with_precision(-0.75, 128);

        assert_eq!(fx, expected_x);
        assert_eq!(fy, expected_y);
    }

    #[test]
    fn pixel_to_fractal_offset_center() {
        // Viewport centered at (-0.5, 0.3)
        let viewport = Viewport::from_f64(-0.5, 0.3, 4.0, 3.0, 128);
        let canvas_size = (800, 600);

        // Center pixel should map to viewport center
        let (fx, fy) = pixel_to_fractal(400.0, 300.0, &viewport, canvas_size, 128);

        assert_eq!(fx, viewport.center.0);
        assert_eq!(fy, viewport.center.1);
    }

    #[test]
    fn pixel_to_fractal_preserves_requested_precision() {
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 7000);
        let canvas_size = (800, 600);

        let (fx, fy) = pixel_to_fractal(100.0, 200.0, &viewport, canvas_size, 7000);

        assert_eq!(fx.precision_bits(), 7000);
        assert_eq!(fy.precision_bits(), 7000);
    }

    #[test]
    fn pixel_to_fractal_extreme_depth_produces_tiny_differences() {
        // At extreme depth (width ~10^-2000), adjacent pixels differ by tiny amounts
        let tiny_width = BigFloat::from_string("1e-2000", 7000).unwrap();
        let tiny_height = BigFloat::from_string("7.5e-2001", 7000).unwrap();
        let viewport = Viewport::with_bigfloat(
            BigFloat::zero(7000),
            BigFloat::zero(7000),
            tiny_width,
            tiny_height,
        );
        let canvas_size = (800, 600);

        // Get fractal coordinates for two adjacent pixels
        let (fx0, _) = pixel_to_fractal(400.0, 300.0, &viewport, canvas_size, 7000);
        let (fx1, _) = pixel_to_fractal(401.0, 300.0, &viewport, canvas_size, 7000);

        // The difference should be non-zero (distinguishable at 7000 bits precision)
        let diff = fx1.sub(&fx0);
        assert!(diff > BigFloat::zero(7000));

        // The difference should be extremely small
        let small_threshold = BigFloat::from_string("1e-300", 7000).unwrap();
        assert!(diff < small_threshold);
    }

    // ============================================================================
    // fractal_to_pixel() Tests
    // NOTE: These legitimately use f64 comparisons because the OUTPUT is f64
    // ============================================================================

    #[test]
    fn fractal_to_pixel_center_maps_to_canvas_center() {
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 128);
        let canvas_size = (800, 600);

        let fx = BigFloat::with_precision(0.0, 128);
        let fy = BigFloat::with_precision(0.0, 128);

        let (px, py) = fractal_to_pixel(&fx, &fy, &viewport, canvas_size);

        assert!((px - 400.0).abs() < 1e-5);
        assert!((py - 300.0).abs() < 1e-5);
    }

    #[test]
    fn fractal_to_pixel_corner_at_half_width() {
        // width=2.0, height=1.5 means corners at (-1.0, -0.75)
        let viewport = Viewport::from_f64(0.0, 0.0, 2.0, 1.5, 128);
        let canvas_size = (800, 600);

        let fx = BigFloat::with_precision(-1.0, 128);
        let fy = BigFloat::with_precision(-0.75, 128);

        let (px, py) = fractal_to_pixel(&fx, &fy, &viewport, canvas_size);

        assert!((px - 0.0).abs() < 1e-5);
        assert!((py - 0.0).abs() < 1e-5);
    }

    #[test]
    fn roundtrip_pixel_to_fractal_to_pixel() {
        let viewport = Viewport::from_f64(-0.5, 0.3, 1.6, 0.9, 128);
        let canvas_size = (1920, 1080);

        let original_px = 1234.0;
        let original_py = 567.0;

        let (fx, fy) = pixel_to_fractal(original_px, original_py, &viewport, canvas_size, 128);
        let (px, py) = fractal_to_pixel(&fx, &fy, &viewport, canvas_size);

        assert!((px - original_px).abs() < 1e-5);
        assert!((py - original_py).abs() < 1e-5);
    }

    // ============================================================================
    // apply_pixel_transform_to_viewport() Tests
    // ============================================================================

    #[test]
    fn apply_transform_zoom_halves_dimensions() {
        // Start with width=4.0, height=3.0
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 128);
        let transform = PixelTransform {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom_factor: 2.0,
            matrix: [[2.0, 0.0, -400.0], [0.0, 2.0, -300.0], [0.0, 0.0, 1.0]],
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size);

        // Width/height should halve: 4.0/2 = 2.0, 3.0/2 = 1.5
        let expected_width = BigFloat::with_precision(2.0, 128);
        let expected_height = BigFloat::with_precision(1.5, 128);
        assert_eq!(new_vp.width, expected_width);
        assert_eq!(new_vp.height, expected_height);

        // Center unchanged (no offset)
        assert_eq!(new_vp.center.0, viewport.center.0);
        assert_eq!(new_vp.center.1, viewport.center.1);
    }

    #[test]
    fn apply_transform_pan_shifts_center() {
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 128);
        let transform = PixelTransform {
            offset_x: 100.0, // Image moved right 100 pixels on screen
            offset_y: -50.0, // Image moved up 50 pixels on screen
            zoom_factor: 1.0,
            matrix: [[1.0, 0.0, 100.0], [0.0, 1.0, -50.0], [0.0, 0.0, 1.0]],
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size);

        // Dimensions unchanged (check numerical equality)
        assert!((new_vp.width.to_f64() - viewport.width.to_f64()).abs() < 1e-10);
        assert!((new_vp.height.to_f64() - viewport.height.to_f64()).abs() < 1e-10);

        // When image moves RIGHT on screen, we see LEFT part of fractal,
        // so viewport center moves LEFT (negative offset applied)
        // x: -100/800 * 4.0 = -0.5, y: +50/600 * 3.0 = +0.25
        let expected_x = -0.5;
        let expected_y = 0.25;

        assert!(
            (new_vp.center.0.to_f64() - expected_x).abs() < 1e-10,
            "Expected center.x ≈ {}, got {}",
            expected_x,
            new_vp.center.0.to_f64()
        );
        assert!(
            (new_vp.center.1.to_f64() - expected_y).abs() < 1e-10,
            "Expected center.y ≈ {}, got {}",
            expected_y,
            new_vp.center.1.to_f64()
        );
    }

    #[test]
    fn apply_transform_combined_zoom_and_pan() {
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 128);
        let transform = PixelTransform {
            offset_x: 100.0, // Image moved right on screen
            offset_y: 50.0,  // Image moved down on screen
            zoom_factor: 2.0,
            matrix: [[2.0, 0.0, -300.0], [0.0, 2.0, -250.0], [0.0, 0.0, 1.0]],
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size);

        // Dimensions should halve
        let expected_width = BigFloat::with_precision(2.0, 128);
        let expected_height = BigFloat::with_precision(1.5, 128);
        assert_eq!(new_vp.width, expected_width);
        assert_eq!(new_vp.height, expected_height);

        // When image moves RIGHT/DOWN on screen, viewport center moves LEFT/UP (negative)
        assert!(new_vp.center.0 < BigFloat::zero(128));
        assert!(new_vp.center.1 < BigFloat::zero(128));
    }

    #[test]
    fn apply_transform_uses_adequate_precision() {
        // At 1x zoom with small canvas, precision is calculated automatically
        let viewport = Viewport::from_f64(0.0, 0.0, 4.0, 3.0, 128);
        let transform = PixelTransform {
            offset_x: 10.0,
            offset_y: 10.0,
            zoom_factor: 1.5,
            matrix: [[1.5, 0.0, -190.0], [0.0, 1.5, -140.0], [0.0, 0.0, 1.0]],
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size);

        // Precision should be at least 64 bits (minimum) and consistent across all fields
        assert!(new_vp.center.0.precision_bits() >= 64);
        assert_eq!(
            new_vp.center.0.precision_bits(),
            new_vp.center.1.precision_bits()
        );
        assert_eq!(
            new_vp.width.precision_bits(),
            new_vp.height.precision_bits()
        );
    }

    #[test]
    fn apply_transform_works_at_extreme_depth() {
        // Start with tiny dimensions (beyond f64 range)
        let tiny_width = BigFloat::from_string("1e-2000", 7000).unwrap();
        let tiny_height = BigFloat::from_string("7.5e-2001", 7000).unwrap();
        let viewport = Viewport::with_bigfloat(
            BigFloat::zero(7000),
            BigFloat::zero(7000),
            tiny_width.clone(),
            tiny_height.clone(),
        );

        let transform = PixelTransform {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom_factor: 2.0,
            matrix: [[2.0, 0.0, -400.0], [0.0, 2.0, -300.0], [0.0, 0.0, 1.0]],
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size);

        // Dimensions should halve
        let expected_width = BigFloat::from_string("5e-2001", 7000).unwrap();
        let expected_height = BigFloat::from_string("3.75e-2001", 7000).unwrap();
        assert_eq!(new_vp.width, expected_width);
        assert_eq!(new_vp.height, expected_height);

        // Center unchanged
        assert_eq!(new_vp.center.0, BigFloat::zero(7000));
        assert_eq!(new_vp.center.1, BigFloat::zero(7000));
    }

    #[test]
    fn apply_transform_pan_at_extreme_depth() {
        // At extreme depth, panning should produce tiny fractal-space changes
        let tiny_width = BigFloat::from_string("1e-2000", 7000).unwrap();
        let tiny_height = BigFloat::from_string("7.5e-2001", 7000).unwrap();
        let viewport = Viewport::with_bigfloat(
            BigFloat::zero(7000),
            BigFloat::zero(7000),
            tiny_width,
            tiny_height,
        );

        let transform = PixelTransform {
            offset_x: 100.0, // Image moved right 100 pixels on screen
            offset_y: 0.0,
            zoom_factor: 1.0,
            matrix: [[1.0, 0.0, 100.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        let canvas_size = (800, 600);

        let new_vp = apply_pixel_transform_to_viewport(&viewport, &transform, canvas_size);

        // When image moves RIGHT on screen, viewport center moves LEFT (negative)
        assert!(new_vp.center.0 < BigFloat::zero(7000));

        // The offset should be extremely tiny (but negative)
        let tiny_threshold = BigFloat::from_string("-1e-300", 7000).unwrap();
        assert!(new_vp.center.0 > tiny_threshold);
    }

    #[test]
    fn test_calculate_aspect_ratio() {
        assert!((calculate_aspect_ratio(1920, 1080) - 1.7777).abs() < 0.001);
        assert!((calculate_aspect_ratio(1080, 1920) - 0.5625).abs() < 0.001);
        assert_eq!(calculate_aspect_ratio(1000, 1000), 1.0);
    }

    // ============================================================================
    // fit_viewport_to_canvas tests
    // ============================================================================

    #[test]
    fn fit_viewport_expands_width_for_landscape_canvas() {
        // Square viewport (4x4) on landscape canvas (1920x1080)
        let natural = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let fitted = fit_viewport_to_canvas(&natural, (1920, 1080));

        // Width should be expanded: new_width = height * (1920/1080) ≈ 7.11
        assert!(fitted.width.to_f64() > 4.0);
        assert!((fitted.height.to_f64() - 4.0).abs() < 0.001);
    }

    #[test]
    fn fit_viewport_expands_height_for_portrait_canvas() {
        // Square viewport (4x4) on portrait canvas (1080x1920)
        let natural = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 128);
        let fitted = fit_viewport_to_canvas(&natural, (1080, 1920));

        // Height should be expanded: new_height = width / (1080/1920) ≈ 7.11
        assert!((fitted.width.to_f64() - 4.0).abs() < 0.001);
        assert!(fitted.height.to_f64() > 4.0);
    }

    #[test]
    fn fit_viewport_preserves_center() {
        let natural = Viewport::from_f64(-0.5, 0.3, 4.0, 4.0, 128);
        let fitted = fit_viewport_to_canvas(&natural, (1920, 1080));

        assert_eq!(fitted.center.0, natural.center.0);
        assert_eq!(fitted.center.1, natural.center.1);
    }

    #[test]
    fn fit_viewport_preserves_precision() {
        let natural = Viewport::from_f64(0.0, 0.0, 4.0, 4.0, 512);
        let fitted = fit_viewport_to_canvas(&natural, (1920, 1080));

        assert_eq!(fitted.precision_bits(), 512);
    }

    #[test]
    fn fit_viewport_unchanged_for_matching_aspect() {
        // Viewport with same aspect as canvas
        let natural = Viewport::from_f64(0.0, 0.0, 16.0, 9.0, 128);
        let fitted = fit_viewport_to_canvas(&natural, (1920, 1080)); // 16:9

        assert!((fitted.width.to_f64() - 16.0).abs() < 0.001);
        assert!((fitted.height.to_f64() - 9.0).abs() < 0.001);
    }

    // ============================================================================
    // calculate_max_iterations tests
    // ============================================================================

    const TEST_MULTIPLIER: f64 = 200.0;
    const TEST_POWER: f64 = 2.5;

    #[test]
    fn max_iterations_at_low_zoom() {
        // zoom_exp = 1 (10^1 zoom)
        let result = calculate_max_iterations(1.0, TEST_MULTIPLIER, TEST_POWER);
        // 200 * 1^2.5 = 200, clamped to 1000
        assert_eq!(result, 1000);
    }

    #[test]
    fn max_iterations_at_10x_zoom() {
        // zoom_exp = 10 (10^10 zoom)
        let result = calculate_max_iterations(10.0, TEST_MULTIPLIER, TEST_POWER);
        // 200 * 10^2.5 ≈ 63,246
        assert!(
            result > 60000 && result < 70000,
            "Expected ~63246, got {}",
            result
        );
    }

    #[test]
    fn max_iterations_at_100x_zoom() {
        // zoom_exp = 100 (10^100 zoom)
        let result = calculate_max_iterations(100.0, TEST_MULTIPLIER, TEST_POWER);
        // 200 * 100^2.5 = 20,000,000 → capped at 10,000,000
        assert_eq!(result, 10_000_000);
    }

    #[test]
    fn max_iterations_at_1000x_zoom() {
        // zoom_exp = 1000 (10^1000 zoom)
        let result = calculate_max_iterations(1000.0, TEST_MULTIPLIER, TEST_POWER);
        // 200 * 1000^2.5 = huge → capped at 10,000,000
        assert_eq!(result, 10_000_000);
    }

    #[test]
    fn max_iterations_at_2000x_zoom() {
        // zoom_exp = 2000 (10^2000 zoom)
        let result = calculate_max_iterations(2000.0, TEST_MULTIPLIER, TEST_POWER);
        // 200 * 2000^2.5 = huge → capped at 10,000,000
        assert_eq!(result, 10_000_000);
    }

    #[test]
    fn max_iterations_capped_at_10m() {
        // Very extreme zoom should cap at 10 million
        let result = calculate_max_iterations(100000.0, TEST_MULTIPLIER, TEST_POWER);
        assert_eq!(result, 10_000_000);
    }

    #[test]
    fn max_iterations_floor_at_1000() {
        // Low/negative zoom should floor at 1000
        let result = calculate_max_iterations(0.5, TEST_MULTIPLIER, TEST_POWER);
        assert_eq!(result, 1000);
    }
}
