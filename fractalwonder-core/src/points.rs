use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point<T> {
    x: T,
    y: T,
}

impl<T> Point<T> {
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

    pub fn mul_scalar(&self, scalar: &T) -> Self
    where
        T: Mul<Output = T> + Clone,
    {
        Self {
            x: self.x.clone() * scalar.clone(),
            y: self.y.clone() * scalar.clone(),
        }
    }

    pub fn div_scalar(&self, scalar: &T) -> Self
    where
        T: Div<Output = T> + Clone,
    {
        Self {
            x: self.x.clone() / scalar.clone(),
            y: self.y.clone() / scalar.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rect<T> {
    pub min: Point<T>,
    pub max: Point<T>,
}

impl<T> Rect<T> {
    pub fn new(min: Point<T>, max: Point<T>) -> Self {
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
    pub fn center(&self) -> Point<T> {
        let two = T::from(2.0);
        let center_x = (self.min.x().clone() + self.max.x().clone()) / two.clone();
        let center_y = (self.min.y().clone() + self.max.y().clone()) / two;
        Point::new(center_x, center_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_construction_and_accessors() {
        let point = Point::new(10.5, 20.5);
        assert_eq!(*point.x(), 10.5);
        assert_eq!(*point.y(), 20.5);
    }

    #[test]
    fn test_point_into_parts() {
        let point = Point::new(10.5, 20.5);
        let (x, y) = point.into_parts();
        assert_eq!(x, 10.5);
        assert_eq!(y, 20.5);
    }

    #[test]
    fn test_point_add() {
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(3.0, 4.0);
        let sum = p1.add(&p2);
        assert_eq!(*sum.x(), 4.0);
        assert_eq!(*sum.y(), 6.0);
    }

    #[test]
    fn test_point_sub() {
        let p1 = Point::new(5.0, 7.0);
        let p2 = Point::new(2.0, 3.0);
        let diff = p1.sub(&p2);
        assert_eq!(*diff.x(), 3.0);
        assert_eq!(*diff.y(), 4.0);
    }

    #[test]
    fn test_point_mul_scalar() {
        let p = Point::new(2.0, 3.0);
        let scaled = p.mul_scalar(&2.5);
        assert_eq!(*scaled.x(), 5.0);
        assert_eq!(*scaled.y(), 7.5);
    }

    #[test]
    fn test_point_div_scalar() {
        let p = Point::new(10.0, 20.0);
        let divided = p.div_scalar(&2.0);
        assert_eq!(*divided.x(), 5.0);
        assert_eq!(*divided.y(), 10.0);
    }

    #[test]
    fn test_point_generic_with_i32() {
        let point_f64 = Point::new(10.5, 20.5);
        let point_i32 = Point::new(10, 20);
        assert_eq!(*point_f64.x(), 10.5);
        assert_eq!(*point_i32.x(), 10);
    }

    #[test]
    fn test_point_precision_maintained() {
        let point = Point::new(1.0, 2.0);
        let scaled = point.mul_scalar(&3.0);
        let divided = scaled.div_scalar(&3.0);
        assert_eq!(*divided.x(), 1.0);
        assert_eq!(*divided.y(), 2.0);
    }

    #[test]
    fn test_rect_construction() {
        let rect = Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        assert_eq!(*rect.min.x(), 0.0);
        assert_eq!(*rect.max.x(), 100.0);
    }

    #[test]
    fn test_rect_dimensions() {
        let rect = Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        assert_eq!(rect.width(), 100.0);
        assert_eq!(rect.height(), 50.0);
    }

    #[test]
    fn test_rect_generic_with_i32() {
        let rect_f64 = Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let rect_i32 = Rect::new(Point::new(0, 0), Point::new(100, 50));
        assert_eq!(rect_f64.width(), 100.0);
        assert_eq!(rect_i32.width(), 100);
    }

    #[test]
    fn test_rect_is_valid_for_valid_rect() {
        let rect = Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        assert!(rect.is_valid());
    }

    #[test]
    fn test_rect_is_valid_for_inverted_x() {
        let rect = Rect::new(Point::new(100.0, 0.0), Point::new(0.0, 50.0));
        assert!(!rect.is_valid());
    }

    #[test]
    fn test_rect_is_valid_for_inverted_y() {
        let rect = Rect::new(Point::new(0.0, 50.0), Point::new(100.0, 0.0));
        assert!(!rect.is_valid());
    }

    #[test]
    fn test_rect_is_valid_for_zero_width() {
        let rect = Rect::new(Point::new(50.0, 0.0), Point::new(50.0, 100.0));
        assert!(rect.is_valid()); // Zero width is valid (point or line)
    }

    #[test]
    fn test_rect_is_valid_for_negative_coords() {
        let rect = Rect::new(Point::new(-100.0, -50.0), Point::new(-10.0, 10.0));
        assert!(rect.is_valid());
    }
}
