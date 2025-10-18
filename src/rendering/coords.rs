#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelCoord {
    x: f64,
    y: f64,
}

impl PixelCoord {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageCoord<T> {
    x: T,
    y: T,
}

impl<T> ImageCoord<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> &T {
        &self.x
    }

    pub fn y(&self) -> &T {
        &self.y
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageRect<T> {
    pub min: ImageCoord<T>,
    pub max: ImageCoord<T>,
}

impl<T: Clone> ImageRect<T> {
    pub fn new(min: ImageCoord<T>, max: ImageCoord<T>) -> Self {
        Self { min, max }
    }

    pub fn width(&self) -> T
    where
        T: std::ops::Sub<Output = T>,
    {
        self.max.x().clone() - self.min.x().clone()
    }

    pub fn height(&self) -> T
    where
        T: std::ops::Sub<Output = T>,
    {
        self.max.y().clone() - self.min.y().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_coord_construction() {
        let coord = PixelCoord::new(10.0, 20.0);
        assert_eq!(coord.x(), 10.0);
        assert_eq!(coord.y(), 20.0);
    }

    #[test]
    fn test_pixel_coord_equality() {
        let coord1 = PixelCoord::new(10.0, 20.0);
        let coord2 = PixelCoord::new(10.0, 20.0);
        let coord3 = PixelCoord::new(10.0, 21.0);
        assert_eq!(coord1, coord2);
        assert_ne!(coord1, coord3);
    }

    #[test]
    fn test_image_coord_f64() {
        let coord = ImageCoord::new(10.5, 20.5);
        assert_eq!(*coord.x(), 10.5);
        assert_eq!(*coord.y(), 20.5);
    }

    #[test]
    fn test_image_coord_generic() {
        let coord_f64 = ImageCoord::new(10.5, 20.5);
        let coord_i32 = ImageCoord::new(10, 20);
        assert_eq!(*coord_f64.x(), 10.5);
        assert_eq!(*coord_i32.x(), 10);
    }

    #[test]
    fn test_image_rect_dimensions() {
        let rect = ImageRect::new(ImageCoord::new(0.0, 0.0), ImageCoord::new(100.0, 50.0));
        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 50.0);
    }
}
