use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord<T> {
    x: T,
    y: T,
}

impl<T> Coord<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> &T {
        &self.x
    }

    pub fn y(&self) -> &T {
        &self.y
    }

    pub fn into_parts(self) -> (T, T) {
        (self.x, self.y)
    }

    pub fn add(&self, other: &Self) -> Self
    where
        T: Add<Output = T> + Clone,
    {
        Self {
            x: self.x.clone() + other.x.clone(),
            y: self.y.clone() + other.y.clone(),
        }
    }

    pub fn sub(&self, other: &Self) -> Self
    where
        T: Sub<Output = T> + Clone,
    {
        Self {
            x: self.x.clone() - other.x.clone(),
            y: self.y.clone() - other.y.clone(),
        }
    }

    pub fn mul_scalar(&self, scalar: f64) -> Self
    where
        T: Mul<f64, Output = T> + Clone,
    {
        Self {
            x: self.x.clone() * scalar,
            y: self.y.clone() * scalar,
        }
    }

    pub fn div_scalar(&self, scalar: f64) -> Self
    where
        T: Div<f64, Output = T> + Clone,
    {
        Self {
            x: self.x.clone() / scalar,
            y: self.y.clone() / scalar,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rect<T> {
    pub min: Coord<T>,
    pub max: Coord<T>,
}

impl<T> Rect<T> {
    pub fn new(min: Coord<T>, max: Coord<T>) -> Self {
        Self { min, max }
    }

    pub fn width(&self) -> T
    where
        T: Sub<Output = T> + Clone,
    {
        self.max.x().clone() - self.min.x().clone()
    }

    pub fn height(&self) -> T
    where
        T: Sub<Output = T> + Clone,
    {
        self.max.y().clone() - self.min.y().clone()
    }

    pub fn is_valid(&self) -> bool
    where
        T: PartialOrd + Clone,
    {
        self.min.x() <= self.max.x() && self.min.y() <= self.max.y()
    }
}

impl<T> Rect<T>
where
    T: Clone + std::ops::Add<Output = T> + std::ops::Div<Output = T> + From<f64>,
{
    /// Calculate center point of rectangle
    pub fn center(&self) -> Coord<T> {
        let two = T::from(2.0);
        let center_x = (self.min.x().clone() + self.max.x().clone()) / two.clone();
        let center_y = (self.min.y().clone() + self.max.y().clone()) / two;
        Coord::new(center_x, center_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_construction_and_accessors() {
        let coord = Coord::new(10.5, 20.5);
        assert_eq!(*coord.x(), 10.5);
        assert_eq!(*coord.y(), 20.5);
    }

    #[test]
    fn test_coord_into_parts() {
        let coord = Coord::new(10.5, 20.5);
        let (x, y) = coord.into_parts();
        assert_eq!(x, 10.5);
        assert_eq!(y, 20.5);
    }

    #[test]
    fn test_coord_add() {
        let c1 = Coord::new(1.0, 2.0);
        let c2 = Coord::new(3.0, 4.0);
        let sum = c1.add(&c2);
        assert_eq!(*sum.x(), 4.0);
        assert_eq!(*sum.y(), 6.0);
    }

    #[test]
    fn test_coord_sub() {
        let c1 = Coord::new(5.0, 7.0);
        let c2 = Coord::new(2.0, 3.0);
        let diff = c1.sub(&c2);
        assert_eq!(*diff.x(), 3.0);
        assert_eq!(*diff.y(), 4.0);
    }

    #[test]
    fn test_coord_mul_scalar() {
        let c = Coord::new(2.0, 3.0);
        let scaled = c.mul_scalar(2.5);
        assert_eq!(*scaled.x(), 5.0);
        assert_eq!(*scaled.y(), 7.5);
    }

    #[test]
    fn test_coord_div_scalar() {
        let c = Coord::new(10.0, 20.0);
        let divided = c.div_scalar(2.0);
        assert_eq!(*divided.x(), 5.0);
        assert_eq!(*divided.y(), 10.0);
    }

    #[test]
    fn test_coord_generic_with_i32() {
        let coord_f64 = Coord::new(10.5, 20.5);
        let coord_i32 = Coord::new(10, 20);
        assert_eq!(*coord_f64.x(), 10.5);
        assert_eq!(*coord_i32.x(), 10);
    }

    #[test]
    fn test_coord_precision_maintained() {
        let coord = Coord::new(1.0, 2.0);
        let scaled = coord.mul_scalar(3.0);
        let divided = scaled.div_scalar(3.0);
        assert_eq!(*divided.x(), 1.0);
        assert_eq!(*divided.y(), 2.0);
    }

    #[test]
    fn test_rect_construction() {
        let rect = Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 50.0));
        assert_eq!(*rect.min.x(), 0.0);
        assert_eq!(*rect.max.x(), 100.0);
    }

    #[test]
    fn test_rect_dimensions() {
        let rect = Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 50.0));
        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 50.0);
    }

    #[test]
    fn test_rect_generic_with_i32() {
        let rect_f64 = Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 50.0));
        let rect_i32 = Rect::new(Coord::new(0, 0), Coord::new(100, 50));
        assert_eq!(rect_f64.width(), 100.0);
        assert_eq!(rect_i32.width(), 100);
    }

    #[test]
    fn test_rect_is_valid_for_valid_rect() {
        let rect = Rect::new(Coord::new(0.0, 0.0), Coord::new(100.0, 50.0));
        assert!(rect.is_valid());
    }

    #[test]
    fn test_rect_is_valid_for_inverted_x() {
        let rect = Rect::new(Coord::new(100.0, 0.0), Coord::new(0.0, 50.0));
        assert!(!rect.is_valid());
    }

    #[test]
    fn test_rect_is_valid_for_inverted_y() {
        let rect = Rect::new(Coord::new(0.0, 50.0), Coord::new(100.0, 0.0));
        assert!(!rect.is_valid());
    }

    #[test]
    fn test_rect_is_valid_for_zero_width() {
        let rect = Rect::new(Coord::new(50.0, 0.0), Coord::new(50.0, 100.0));
        assert!(rect.is_valid()); // Zero width is valid (point or line)
    }

    #[test]
    fn test_rect_is_valid_for_negative_coords() {
        let rect = Rect::new(Coord::new(-100.0, -50.0), Coord::new(-10.0, 10.0));
        assert!(rect.is_valid());
    }
}
