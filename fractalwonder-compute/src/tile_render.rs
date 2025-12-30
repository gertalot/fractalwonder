//! Tile rendering logic extracted for testability.
//!
//! This module contains the core tile computation logic used by the worker.
//! By extracting it here, we can test the exact production code path.

use crate::bla::BlaTable;
use crate::perturbation::{compute_pixel_perturbation, compute_pixel_perturbation_hdr_bla};
use crate::ReferenceOrbit;
use fractalwonder_core::{BigFloat, ComplexDelta, ComputeData, F64Complex, HDRComplex, HDRFloat};

/// Input parameters for tile rendering.
/// These match exactly what the worker receives.
pub struct TileRenderInput {
    pub delta_c_origin: (BigFloat, BigFloat),
    pub delta_c_step: (BigFloat, BigFloat),
    pub tile_width: u32,
    pub tile_height: u32,
    pub max_iterations: u32,
    pub tau_sq: f64,
    pub bla_enabled: bool,
    pub force_hdr_float: bool,
}

/// Render a tile using the exact production code path.
///
/// This function contains the same logic as worker.rs lines 378-465.
/// It dispatches between f64 and HDRFloat paths based on delta magnitude.
pub fn render_tile(
    orbit: &ReferenceOrbit,
    bla_table: Option<&BlaTable>,
    input: &TileRenderInput,
) -> Vec<ComputeData> {
    // Check if deltas fit in f64 range (roughly 10^-300 to 10^300)
    // log2 of ~10^-300 is about -1000, so we use -900 as safe threshold
    // force_hdr_float overrides this check for debugging deep zoom issues
    let delta_log2 = input
        .delta_c_origin
        .0
        .log2_approx()
        .max(input.delta_c_origin.1.log2_approx());
    let deltas_fit_f64 = !input.force_hdr_float && delta_log2 > -900.0 && delta_log2 < 900.0;

    let mut data = Vec::with_capacity((input.tile_width * input.tile_height) as usize);

    // Two-tier dispatch based on delta magnitude:
    // 1. Deltas fit in f64 range: Use fast f64 path (most common case)
    // 2. Otherwise: Use HDRFloat (handles arbitrary exponent range)
    //
    // NOTE: BigFloat is intentionally NOT used for pixel calculations.
    // BigFloat should ONLY be used for reference orbit computation.
    // HDRFloat provides sufficient precision for pixel deltas at any zoom level.

    if deltas_fit_f64 {
        // Fast path: f64 arithmetic
        // Note: BLA is disabled for f64 path because at zoom levels where f64
        // deltas are valid, the BLA validity radius (r_sq) becomes too small
        // after merging, providing no iteration skipping benefit.
        let delta_origin = (
            input.delta_c_origin.0.to_f64(),
            input.delta_c_origin.1.to_f64(),
        );
        let delta_step = (input.delta_c_step.0.to_f64(), input.delta_c_step.1.to_f64());

        let mut delta_c_row = delta_origin;

        for _py in 0..input.tile_height {
            let mut delta_c = delta_c_row;

            for _px in 0..input.tile_width {
                let result = compute_pixel_perturbation(
                    orbit,
                    F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                    input.max_iterations,
                    input.tau_sq,
                );
                data.push(ComputeData::Mandelbrot(result));

                delta_c.0 += delta_step.0;
            }

            delta_c_row.1 += delta_step.1;
        }
    } else {
        // HDRFloat path: handles arbitrary exponent range with ~48-bit mantissa
        // This is sufficient for pixel calculations at any zoom depth.
        let delta_origin = HDRComplex {
            re: HDRFloat::from_bigfloat(&input.delta_c_origin.0),
            im: HDRFloat::from_bigfloat(&input.delta_c_origin.1),
        };
        let delta_step = HDRComplex {
            re: HDRFloat::from_bigfloat(&input.delta_c_step.0),
            im: HDRFloat::from_bigfloat(&input.delta_c_step.1),
        };

        let mut delta_c_row = delta_origin;

        for _py in 0..input.tile_height {
            let mut delta_c = delta_c_row;

            for _px in 0..input.tile_width {
                let result = if input.bla_enabled {
                    if let Some(bla_table) = bla_table {
                        compute_pixel_perturbation_hdr_bla(
                            orbit,
                            bla_table,
                            delta_c,
                            input.max_iterations,
                            input.tau_sq,
                        )
                    } else {
                        // Fallback if table wasn't built
                        compute_pixel_perturbation(orbit, delta_c, input.max_iterations, input.tau_sq)
                    }
                } else {
                    compute_pixel_perturbation(orbit, delta_c, input.max_iterations, input.tau_sq)
                };
                data.push(ComputeData::Mandelbrot(result));

                delta_c.re = delta_c.re.add(&delta_step.re);
            }

            delta_c_row.im = delta_c_row.im.add(&delta_step.im);
        }
    }

    data
}

