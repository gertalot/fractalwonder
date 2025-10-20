use crate::rendering::points::Point;

#[derive(Debug, Clone, PartialEq)]
pub struct Viewport<T> {
    pub center: Point<T>,
    pub zoom: f64,
}

impl<T: Clone> Viewport<T> {
    pub fn new(center: Point<T>, zoom: f64) -> Self {
        Self { center, zoom }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_construction() {
        let viewport = Viewport::new(Point::new(0.0, 0.0), 1.0);
        assert_eq!(*viewport.center.x(), 0.0);
        assert_eq!(*viewport.center.y(), 0.0);
        assert_eq!(viewport.zoom, 1.0);
    }

    #[test]
    fn test_viewport_generic_types() {
        let viewport_f64 = Viewport::new(Point::new(0.0, 0.0), 1.0);
        let viewport_i32 = Viewport::new(Point::new(0, 0), 2.0);
        assert_eq!(viewport_f64.zoom, 1.0);
        assert_eq!(viewport_i32.zoom, 2.0);
    }
}
