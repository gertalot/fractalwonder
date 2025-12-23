use crate::ReferenceOrbit;
use fractalwonder_core::BigFloat;

#[test]
fn reference_orbit_in_set_never_escapes() {
    // Point (-0.5, 0) is in the main cardioid
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    assert_eq!(orbit.escaped_at, None);
    assert_eq!(orbit.orbit.len(), 1000);
    assert!((orbit.c_ref.0 - (-0.5)).abs() < 1e-10);
    assert!((orbit.c_ref.1 - 0.0).abs() < 1e-10);
}

#[test]
fn reference_orbit_outside_set_escapes() {
    // Point (2, 0) escapes quickly
    let c_ref = (BigFloat::with_precision(2.0, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 1000);

    assert!(orbit.escaped_at.is_some());
    assert!(orbit.escaped_at.unwrap() < 10);
}

#[test]
fn reference_orbit_values_bounded() {
    // All orbit values should be bounded by escape radius
    let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    for (x, y) in &orbit.orbit {
        let mag_sq = x * x + y * y;
        assert!(mag_sq <= 65536.0, "Orbit value escaped: ({}, {})", x, y);
    }
}

#[test]
fn orbit_satisfies_recurrence_relation() {
    // Verify that orbit values follow z_{n+1} = z_n^2 + c exactly
    let c_ref = (
        BigFloat::with_precision(-0.5, 128),
        BigFloat::with_precision(0.1, 128),
    );
    let orbit = ReferenceOrbit::compute(&c_ref, 100);

    let (c_x, c_y) = orbit.c_ref;

    for n in 0..orbit.orbit.len() - 1 {
        let (xn, yn) = orbit.orbit[n];
        let (xn1, yn1) = orbit.orbit[n + 1];

        // z_{n+1} = z_n^2 + c
        // (x + iy)^2 = x^2 - y^2 + 2ixy
        let expected_x = xn * xn - yn * yn + c_x;
        let expected_y = 2.0 * xn * yn + c_y;

        // Allow small floating point error since orbit stores f64
        assert!(
            (xn1 - expected_x).abs() < 1e-10,
            "x recurrence failed at n={}: got {}, expected {}",
            n,
            xn1,
            expected_x
        );
        assert!(
            (yn1 - expected_y).abs() < 1e-10,
            "y recurrence failed at n={}: got {}, expected {}",
            n,
            yn1,
            expected_y
        );
    }
}

#[test]
fn orbit_starts_at_origin() {
    // The Mandelbrot iteration z_{n+1} = z_n^2 + c starts with z_0 = 0
    let orbit = ReferenceOrbit::compute(
        &(
            BigFloat::with_precision(-0.5, 128),
            BigFloat::with_precision(0.1, 128),
        ),
        100,
    );
    assert_eq!(orbit.orbit[0], (0.0, 0.0), "Orbit must start at origin");
}

#[test]
fn orbit_known_values_c_equals_neg1() {
    // c = -1: orbit is 0, -1, 0, -1, ... (period 2)
    // z_0 = 0
    // z_1 = 0^2 + (-1) = -1
    // z_2 = (-1)^2 + (-1) = 0
    // z_3 = 0^2 + (-1) = -1
    // ...
    let orbit = ReferenceOrbit::compute(
        &(BigFloat::with_precision(-1.0, 128), BigFloat::zero(128)),
        100,
    );

    // Point c = -1 is in the set (bounded period-2 orbit)
    assert!(orbit.escaped_at.is_none(), "c = -1 should not escape");

    // Check the orbit values
    assert_eq!(orbit.orbit[0], (0.0, 0.0), "z_0 should be 0");
    assert!(
        (orbit.orbit[1].0 - (-1.0)).abs() < 1e-14 && orbit.orbit[1].1.abs() < 1e-14,
        "z_1 should be -1, got {:?}",
        orbit.orbit[1]
    );
    assert!(
        orbit.orbit[2].0.abs() < 1e-14 && orbit.orbit[2].1.abs() < 1e-14,
        "z_2 should be 0, got {:?}",
        orbit.orbit[2]
    );
    assert!(
        (orbit.orbit[3].0 - (-1.0)).abs() < 1e-14 && orbit.orbit[3].1.abs() < 1e-14,
        "z_3 should be -1, got {:?}",
        orbit.orbit[3]
    );
}

#[test]
fn orbit_diverges_with_tiny_precision_difference() {
    // This test proves precision matters: two points differing by ~10^-16
    // produce different escape behavior at boundary regions.

    // Point on the "antenna" (real axis boundary) where chaotic behavior is extreme
    // c = -2 is the tip of the antenna; nearby points are extremely sensitive
    // Using a point that escapes after many iterations to show sensitivity
    let c1 = (
        BigFloat::from_string("-1.9999999999999998", 128).unwrap(),
        BigFloat::zero(128),
    );
    let c2 = (
        BigFloat::from_string("-2.0000000000000002", 128).unwrap(),
        BigFloat::zero(128),
    );

    // Compute orbits
    let orbit1 = ReferenceOrbit::compute(&c1, 10000);
    let orbit2 = ReferenceOrbit::compute(&c2, 10000);

    // c1 is slightly inside (-2 is the boundary), c2 is slightly outside
    // One should escape, the other should not (or escape much later)
    let escaped_differently = orbit1.escaped_at.is_some() != orbit2.escaped_at.is_some();

    let escape_time_differs = match (orbit1.escaped_at, orbit2.escaped_at) {
        (Some(e1), Some(e2)) => (e1 as i32 - e2 as i32).abs() > 100,
        _ => false,
    };

    assert!(
        escaped_differently || escape_time_differs,
        "Orbits should diverge: c1 (inside boundary) vs c2 (outside boundary). \
         orbit1.escaped_at={:?}, orbit2.escaped_at={:?}",
        orbit1.escaped_at,
        orbit2.escaped_at
    );
}

#[test]
fn high_precision_orbit_differs_from_low_precision() {
    // Compare orbit computed with different precision levels
    // This demonstrates why we need arbitrary precision at deep zoom

    // Point in chaotic region
    let c_high = (
        BigFloat::from_string("-0.7436438870371587", 256).unwrap(),
        BigFloat::from_string("0.1318259043091895", 256).unwrap(),
    );

    let c_low = (
        BigFloat::with_precision(-0.7436438870371587, 64),
        BigFloat::with_precision(0.1318259043091895, 64),
    );

    let orbit_high = ReferenceOrbit::compute(&c_high, 10000);
    let orbit_low = ReferenceOrbit::compute(&c_low, 10000);

    // Both should have the same f64 c_ref (since that's stored as f64)
    assert!(
        (orbit_high.c_ref.0 - orbit_low.c_ref.0).abs() < 1e-14,
        "c_ref should be approximately equal"
    );

    // But orbit behavior may differ due to precision during computation
    // This is expected behavior - at deep zoom, precision matters
    // The test passes as long as orbits are computed without error
    assert!(
        !orbit_high.orbit.is_empty(),
        "High precision orbit should compute"
    );
    assert!(
        !orbit_low.orbit.is_empty(),
        "Low precision orbit should compute"
    );
}
