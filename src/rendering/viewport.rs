use crate::rendering::coords::{ImageCoord, ImageRect};

#[derive(Debug, Clone, PartialEq)]
pub struct Viewport<T> {
    pub center: ImageCoord<T>,
    pub zoom: f64,
    pub natural_bounds: ImageRect<T>,
}

impl<T: Clone> Viewport<T> {
    pub fn new(center: ImageCoord<T>, zoom: f64, natural_bounds: ImageRect<T>) -> Self {
        Self {
            center,
            zoom,
            natural_bounds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_construction() {
        let viewport = Viewport::new(
            ImageCoord::new(0.0, 0.0),
            1.0,
            ImageRect::new(ImageCoord::new(-50.0, -50.0), ImageCoord::new(50.0, 50.0)),
        );
        assert_eq!(*viewport.center.x(), 0.0);
        assert_eq!(*viewport.center.y(), 0.0);
        assert_eq!(viewport.zoom, 1.0);
    }

    #[test]
    fn test_viewport_generic_types() {
        let viewport_f64 = Viewport::new(
            ImageCoord::new(0.0, 0.0),
            1.0,
            ImageRect::new(ImageCoord::new(-1.0, -1.0), ImageCoord::new(1.0, 1.0)),
        );
        let viewport_i32 = Viewport::new(
            ImageCoord::new(0, 0),
            2.0,
            ImageRect::new(ImageCoord::new(-10, -10), ImageCoord::new(10, 10)),
        );
        assert_eq!(viewport_f64.zoom, 1.0);
        assert_eq!(viewport_i32.zoom, 2.0);
    }
}
