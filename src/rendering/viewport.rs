use crate::rendering::coords::{Coord, Rect};

#[derive(Debug, Clone, PartialEq)]
pub struct Viewport<T> {
    pub center: Coord<T>,
    pub zoom: f64,
    pub natural_bounds: Rect<T>,
}

impl<T: Clone> Viewport<T> {
    pub fn new(center: Coord<T>, zoom: f64, natural_bounds: Rect<T>) -> Self {
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
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-50.0, -50.0), Coord::new(50.0, 50.0)),
        );
        assert_eq!(*viewport.center.x(), 0.0);
        assert_eq!(*viewport.center.y(), 0.0);
        assert_eq!(viewport.zoom, 1.0);
    }

    #[test]
    fn test_viewport_generic_types() {
        let viewport_f64 = Viewport::new(
            Coord::new(0.0, 0.0),
            1.0,
            Rect::new(Coord::new(-1.0, -1.0), Coord::new(1.0, 1.0)),
        );
        let viewport_i32 = Viewport::new(
            Coord::new(0, 0),
            2.0,
            Rect::new(Coord::new(-10, -10), Coord::new(10, 10)),
        );
        assert_eq!(viewport_f64.zoom, 1.0);
        assert_eq!(viewport_i32.zoom, 2.0);
    }
}
