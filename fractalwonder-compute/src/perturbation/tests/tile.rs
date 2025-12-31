//! Tests for tile rendering functions.

use crate::perturbation::tile::{render_tile_f64, TileConfig};
use crate::ReferenceOrbit;
use fractalwonder_core::{BigFloat, ComputeData};

#[test]
fn render_tile_f64_produces_correct_pixel_count() {
    // Create a simple reference orbit at c = -0.5
    let c_ref = (BigFloat::with_precision(-0.5, 64), BigFloat::zero(64));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    let config = TileConfig {
        size: (4, 4),
        max_iterations: 100,
        tau_sq: 1e-6,
        bla_enabled: false,
    };

    // Delta origin and step for a 4x4 tile
    let delta_origin = (0.1, 0.1);
    let delta_step = (0.01, 0.01);

    let result = render_tile_f64(&orbit, delta_origin, delta_step, &config);

    assert_eq!(result.data.len(), 16, "4x4 tile should produce 16 pixels");
    assert!(
        result
            .data
            .iter()
            .all(|d| matches!(d, ComputeData::Mandelbrot(_))),
        "All pixels should be Mandelbrot data"
    );
}

#[test]
fn render_tile_f64_escapes_outside_set() {
    // Reference at origin
    let c_ref = (BigFloat::zero(64), BigFloat::zero(64));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    let config = TileConfig {
        size: (2, 2),
        max_iterations: 100,
        tau_sq: 1e-6,
        bla_enabled: false,
    };

    // Delta puts pixels outside the set (|c| > 2)
    let delta_origin = (2.5, 2.5);
    let delta_step = (0.1, 0.1);

    let result = render_tile_f64(&orbit, delta_origin, delta_step, &config);

    // All pixels should escape quickly
    for pixel in &result.data {
        let ComputeData::Mandelbrot(m) = pixel;
        assert!(m.escaped, "Pixels at |c| > 2 should escape");
        assert!(m.iterations < 10, "Should escape within few iterations");
    }
}
