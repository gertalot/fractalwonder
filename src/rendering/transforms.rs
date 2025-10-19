use crate::rendering::{
    coords::{Coord, Rect},
    viewport::Viewport,
};

pub fn calculate_visible_bounds<T>(
    viewport: &Viewport<T>,
    canvas_width: u32,
    canvas_height: u32,
) -> Rect<T>
where
    T: Clone + std::ops::Sub<Output = T> + std::ops::Add<Output = T> + std::ops::Div<f64, Output = T> + std::ops::Mul<f64, Output = T>,
{
    let natural_width = viewport.natural_bounds.width();
    let natural_height = viewport.natural_bounds.height();

    // Apply zoom (1.0 = show entire natural bounds)
    let view_width = natural_width / viewport.zoom;
    let view_height = natural_height / viewport.zoom;

    // Adjust for canvas aspect ratio - extend the wider dimension
    let canvas_aspect = canvas_width as f64 / canvas_height as f64;

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
    T: Clone + std::ops::Sub<Output = T> + std::ops::Add<Output = T> + std::ops::Mul<f64, Output = T>,
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
}
