use fractalwonder::rendering::*;

#[test]
fn test_mandelbrot_bigfloat_compute() {
    let computer = MandelbrotComputer::<BigFloat>::new();

    // High zoom viewport
    let precision_bits = 256;
    let viewport = Viewport::new(
        Point::new(
            BigFloat::with_precision(-0.5, precision_bits),
            BigFloat::with_precision(0.0, precision_bits),
        ),
        1e15,
    );

    // Compute a point
    let point = Point::new(
        BigFloat::with_precision(-0.5, precision_bits),
        BigFloat::with_precision(0.0, precision_bits),
    );

    let result = computer.compute(point, &viewport);

    // Should compute without panicking
    assert!(result.iterations > 0);
}

#[test]
fn test_precision_calculator_scaling() {
    let bits_1 = PrecisionCalculator::calculate_precision_bits(1.0);
    let bits_20 = PrecisionCalculator::calculate_precision_bits(1e20);
    let bits_50 = PrecisionCalculator::calculate_precision_bits(1e50);
    let bits_120 = PrecisionCalculator::calculate_precision_bits(1e120);

    // Should scale
    assert!(bits_20 > bits_1);
    assert!(bits_50 > bits_20);
    assert!(bits_120 > bits_50);

    // All should be powers of 2
    assert_eq!(bits_1.count_ones(), 1);
    assert_eq!(bits_20.count_ones(), 1);
    assert_eq!(bits_50.count_ones(), 1);
    assert_eq!(bits_120.count_ones(), 1);
}

#[test]
fn test_bigfloat_to_f64_conversion() {
    let val = BigFloat::with_precision(42.5, 128);
    assert!((val.to_f64() - 42.5).abs() < 1e-10);

    let val2 = BigFloat::with_precision(-0.123456789, 256);
    assert!((val2.to_f64() - (-0.123456789)).abs() < 1e-10);
}

#[test]
fn test_pixel_renderer_with_bigfloat() {
    let computer = MandelbrotComputer::<BigFloat>::new();
    let renderer = PixelRenderer::new(computer);

    let precision_bits = 128;
    let viewport = Viewport::new(
        Point::new(
            BigFloat::with_precision(0.0, precision_bits),
            BigFloat::with_precision(0.0, precision_bits),
        ),
        1.0,
    );

    let pixel_rect = PixelRect::full_canvas(10, 10);
    let data = renderer.render(&viewport, pixel_rect, (10, 10));

    // Should render full canvas
    assert_eq!(data.len(), 100);

    // All data points should be valid
    assert!(data.iter().all(|d| d.iterations <= 256));
}

#[test]
fn test_bigfloat_arithmetic_high_precision() {
    let a = BigFloat::with_precision(0.1, 256);
    let b = BigFloat::with_precision(0.2, 256);
    let c = a.clone() + b.clone();

    // High precision addition should work
    let result = c.to_f64();
    assert!((result - 0.3).abs() < 1e-10);
}
