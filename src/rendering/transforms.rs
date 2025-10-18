use crate::rendering::{
    coords::{ImageCoord, ImageRect, PixelCoord},
    viewport::Viewport,
};

pub fn calculate_visible_bounds<T>(
    viewport: &Viewport<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> ImageRect<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Div<f64, Output = T>
        + std::ops::Mul<f64, Output = T>
        + std::ops::Add<Output = T>,
{
    let natural_width =
        viewport.natural_bounds.max.x().clone() - viewport.natural_bounds.min.x().clone();
    let natural_height =
        viewport.natural_bounds.max.y().clone() - viewport.natural_bounds.min.y().clone();

    // Apply zoom (1.0 = show entire natural bounds)
    let view_width = natural_width / viewport.zoom;
    let view_height = natural_height / viewport.zoom;

    // Adjust for canvas aspect ratio - extend the wider dimension
    let canvas_aspect = canvas_width as f64 / canvas_height as f64;

    // Simplified: assume T can be multiplied by f64
    let (final_width, final_height) = if canvas_aspect > 1.0 {
        // Landscape - extend width
        (view_height.clone() * canvas_aspect, view_height)
    } else {
        // Portrait - extend height
        (view_width.clone(), view_width / canvas_aspect)
    };

    // Calculate bounds centered on viewport.center
    ImageRect::new(
        ImageCoord::new(
            viewport.center.x().clone() - final_width.clone() / 2.0,
            viewport.center.y().clone() - final_height.clone() / 2.0,
        ),
        ImageCoord::new(
            viewport.center.x().clone() + final_width / 2.0,
            viewport.center.y().clone() + final_height / 2.0,
        ),
    )
}

pub fn pixel_to_image<T>(
    pixel: PixelCoord,
    visible_bounds: &ImageRect<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> ImageCoord<T>
where
    T: Clone
        + std::ops::Sub<Output = T>
        + std::ops::Mul<f64, Output = T>
        + std::ops::Add<Output = T>,
{
    let bounds_width = visible_bounds.max.x().clone() - visible_bounds.min.x().clone();
    let bounds_height = visible_bounds.max.y().clone() - visible_bounds.min.y().clone();

    ImageCoord::new(
        visible_bounds.min.x().clone() + bounds_width * (pixel.x() / canvas_width as f64),
        visible_bounds.min.y().clone() + bounds_height * (pixel.y() / canvas_height as f64),
    )
}

pub fn image_to_pixel<T>(
    image: &ImageCoord<T>,
    visible_bounds: &ImageRect<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> PixelCoord
where
    T: Clone + std::ops::Sub<Output = T> + std::ops::Div<Output = T>,
    f64: std::ops::Mul<T, Output = f64>,
{
    let bounds_width = visible_bounds.max.x().clone() - visible_bounds.min.x().clone();
    let bounds_height = visible_bounds.max.y().clone() - visible_bounds.min.y().clone();

    let normalized_x = (image.x().clone() - visible_bounds.min.x().clone()) / bounds_width;
    let normalized_y = (image.y().clone() - visible_bounds.min.y().clone()) / bounds_height;

    PixelCoord::new(
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
            ImageCoord::new(0.0, 0.0),
            1.0,
            ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0)),
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
            ImageCoord::new(0.0, 0.0),
            1.0,
            ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0)),
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
            ImageCoord::new(0.0, 0.0),
            2.0, // 2x zoom
            ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0)),
        );

        // Square canvas
        let bounds = calculate_visible_bounds(&viewport, 1000, 1000);

        // At zoom 2.0, should show half the natural area (50 units)
        assert_eq!(bounds.width(), 50.0);
        assert_eq!(bounds.height(), 50.0);
    }

    #[test]
    fn test_pixel_to_image_center() {
        let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
        let pixel = PixelCoord::new(500.0, 500.0); // Center of 1000x1000 canvas
        let image = pixel_to_image(pixel, &bounds, 1000, 1000);

        assert_eq!(*image.x(), 0.0);
        assert_eq!(*image.y(), 0.0);
    }

    #[test]
    fn test_pixel_to_image_corners() {
        let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));

        // Top-left corner
        let image = pixel_to_image(PixelCoord::new(0.0, 0.0), &bounds, 1000, 1000);
        assert_eq!(*image.x(), -50.0);
        assert_eq!(*image.y(), -50.0);

        // Bottom-right corner
        let image = pixel_to_image(PixelCoord::new(1000.0, 1000.0), &bounds, 1000, 1000);
        assert_eq!(*image.x(), 50.0);
        assert_eq!(*image.y(), 50.0);
    }

    #[test]
    fn test_image_to_pixel_center() {
        let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
        let image = ImageCoord::new(0.0, 0.0);
        let pixel = image_to_pixel(&image, &bounds, 1000, 1000);

        assert_eq!(pixel.x(), 500.0);
        assert_eq!(pixel.y(), 500.0);
    }

    #[test]
    fn test_round_trip_pixel_image_pixel() {
        let bounds = ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0));
        let original = PixelCoord::new(123.0, 456.0);

        let image = pixel_to_image(original, &bounds, 1000, 1000);
        let result = image_to_pixel(&image, &bounds, 1000, 1000);

        assert!((result.x() - original.x()).abs() < 0.001);
        assert!((result.y() - original.y()).abs() < 0.001);
    }
}
