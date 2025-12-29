// fractalwonder-compute/src/worker.rs
use crate::{
    compute_pixel_perturbation, compute_pixel_perturbation_hdr_bla, BlaTable, MandelbrotRenderer,
    ReferenceOrbit, Renderer, TestImageRenderer,
};
use fractalwonder_core::{
    BigFloat, ComplexDelta, ComputeData, F64Complex, HDRComplex, HDRFloat, MainToWorker, Viewport,
    WorkerToMain,
};
use js_sys::Date;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Boxed renderer trait object for dynamic dispatch.
type BoxedRenderer = Box<dyn Renderer<Data = ComputeData>>;

/// Cached reference orbit for perturbation rendering.
struct CachedOrbit {
    c_ref: (f64, f64),
    orbit: Vec<(f64, f64)>,
    derivative: Vec<(f64, f64)>,
    escaped_at: Option<u32>,
    bla_table: Option<BlaTable>,
}

impl CachedOrbit {
    fn to_reference_orbit(&self) -> ReferenceOrbit {
        ReferenceOrbit {
            c_ref: self.c_ref,
            orbit: self.orbit.clone(),
            derivative: self.derivative.clone(),
            escaped_at: self.escaped_at,
        }
    }
}

/// Worker state including renderer and orbit cache.
struct WorkerState {
    renderer: Option<BoxedRenderer>,
    orbit_cache: HashMap<u32, CachedOrbit>,
}

impl WorkerState {
    fn new() -> Self {
        Self {
            renderer: None,
            orbit_cache: HashMap::new(),
        }
    }
}

fn create_renderer(renderer_id: &str) -> Option<BoxedRenderer> {
    match renderer_id {
        "test_image" => Some(Box::new(TestImageRendererWrapper)),
        "mandelbrot" => Some(Box::new(MandelbrotRendererWrapper {
            max_iterations: 1000,
        })),
        _ => None,
    }
}

// Wrapper to unify renderer output types
struct TestImageRendererWrapper;

impl Renderer for TestImageRendererWrapper {
    type Data = ComputeData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data> {
        TestImageRenderer
            .render(viewport, canvas_size)
            .into_iter()
            .map(ComputeData::TestImage)
            .collect()
    }
}

struct MandelbrotRendererWrapper {
    max_iterations: u32,
}

impl Renderer for MandelbrotRendererWrapper {
    type Data = ComputeData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<Self::Data> {
        MandelbrotRenderer::new(self.max_iterations)
            .render(viewport, canvas_size)
            .into_iter()
            .map(ComputeData::Mandelbrot)
            .collect()
    }
}

fn post_message(msg: &WorkerToMain) {
    match serde_json::to_string(msg) {
        Ok(json) => {
            let global: web_sys::DedicatedWorkerGlobalScope =
                js_sys::global().dyn_into().expect("Not in worker context");
            let _ = global.post_message(&JsValue::from_str(&json));
        }
        Err(e) => {
            web_sys::console::error_1(
                &format!("[Worker] Failed to serialize message: {}", e).into(),
            );
        }
    }
}

fn handle_message(state: &mut WorkerState, data: JsValue) {
    let Some(msg_str) = data.as_string() else {
        post_message(&WorkerToMain::Error {
            message: "Message is not a string".to_string(),
        });
        return;
    };

    let msg: MainToWorker = match serde_json::from_str(&msg_str) {
        Ok(m) => m,
        Err(e) => {
            post_message(&WorkerToMain::Error {
                message: format!("Failed to parse message: {}", e),
            });
            return;
        }
    };

    match msg {
        MainToWorker::Initialize { renderer_id } => {
            web_sys::console::log_1(
                &format!("[Worker] Initialize with renderer: {}", renderer_id).into(),
            );
            match create_renderer(&renderer_id) {
                Some(r) => {
                    state.renderer = Some(r);
                    // Signal ready for work
                    post_message(&WorkerToMain::RequestWork { render_id: None });
                }
                None => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Unknown renderer: {}", renderer_id),
                    });
                }
            }
        }

        MainToWorker::RenderTile {
            render_id,
            viewport_json,
            tile,
        } => {
            let Some(r) = state.renderer.as_ref() else {
                post_message(&WorkerToMain::Error {
                    message: "Renderer not initialized".to_string(),
                });
                return;
            };

            // Parse viewport
            let viewport: Viewport = match serde_json::from_str(&viewport_json) {
                Ok(v) => v,
                Err(e) => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Failed to parse viewport: {}", e),
                    });
                    return;
                }
            };

            let start_time = Date::now();

            // Render tile
            let data = r.render(&viewport, (tile.width, tile.height));

            let compute_time_ms = Date::now() - start_time;

            // Detect all-black tiles (all points in set = potential rendering bug)
            let in_set_count = data
                .iter()
                .filter(|d| match d {
                    fractalwonder_core::ComputeData::Mandelbrot(m) => !m.escaped,
                    _ => false,
                })
                .count();
            let total_pixels = data.len();
            if in_set_count == total_pixels && total_pixels > 0 {
                web_sys::console::warn_1(
                    &format!(
                        "[Worker] ALL-BLACK tile at ({},{}) {}x{}: {}/{} in set. viewport center=({}, {}), width={}",
                        tile.x, tile.y, tile.width, tile.height,
                        in_set_count, total_pixels,
                        viewport.center.0.to_f64(),
                        viewport.center.1.to_f64(),
                        viewport.width.to_f64()
                    )
                    .into(),
                );
            }

            // Send result
            post_message(&WorkerToMain::TileComplete {
                render_id,
                tile,
                data,
                compute_time_ms,
            });

            // Request next work
            post_message(&WorkerToMain::RequestWork {
                render_id: Some(render_id),
            });
        }

        MainToWorker::NoWork => {
            // Idle - wait for next message
        }

        MainToWorker::Terminate => {
            web_sys::console::log_1(&"[Worker] Terminating".into());
            let global: web_sys::DedicatedWorkerGlobalScope =
                js_sys::global().dyn_into().expect("Not in worker context");
            global.close();
        }

        MainToWorker::ComputeReferenceOrbit {
            render_id,
            orbit_id,
            c_ref_json,
            max_iterations,
        } => {
            // Parse c_ref from JSON (BigFloat coordinates)
            let c_ref: (BigFloat, BigFloat) = match serde_json::from_str(&c_ref_json) {
                Ok(c) => c,
                Err(e) => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Failed to parse c_ref: {}", e),
                    });
                    return;
                }
            };

            let start_time = Date::now();

            // Compute reference orbit
            let orbit = ReferenceOrbit::compute(&c_ref, max_iterations);

            let compute_time = Date::now() - start_time;
            web_sys::console::log_1(
                &format!(
                    "[Worker] Reference orbit computed: {} iterations in {:.0}ms, escaped_at={:?}",
                    orbit.orbit.len(),
                    compute_time,
                    orbit.escaped_at
                )
                .into(),
            );

            // Send result back
            post_message(&WorkerToMain::ReferenceOrbitComplete {
                render_id,
                orbit_id,
                c_ref: orbit.c_ref,
                orbit: orbit.orbit,
                derivative: orbit.derivative,
                escaped_at: orbit.escaped_at,
            });
        }

        MainToWorker::StoreReferenceOrbit {
            orbit_id,
            c_ref,
            orbit,
            derivative,
            escaped_at,
            dc_max,
            bla_enabled,
        } => {
            // BLA helps at deep zoom where iteration counts are high.
            // Phil Thompson enables BLA at scale > 1e25 (dc_max < ~1e-25).
            // Reference: https://philthompson.me/2023/Faster-Mandelbrot-Set-Rendering-with-BLA-Bivariate-Linear-Approximation.html
            let dc_max_log2 = dc_max.log2();
            let bla_useful = dc_max_log2 < -80.0; // Roughly 10^-25 (scale > 1e25)

            let bla_table = if bla_enabled && bla_useful {
                let ref_orbit = ReferenceOrbit {
                    c_ref,
                    orbit: orbit.clone(),
                    derivative: derivative.clone(),
                    escaped_at,
                };
                let table = BlaTable::compute(&ref_orbit, dc_max);
                web_sys::console::log_1(
                    &format!(
                        "[Worker] Built BLA table: {} entries, {} levels (dc_max={:.2e})",
                        table.entries.len(),
                        table.num_levels,
                        dc_max
                    )
                    .into(),
                );
                Some(table)
            } else {
                if bla_enabled && !bla_useful {
                    web_sys::console::log_1(
                        &format!(
                            "[Worker] Skipping BLA table: dc_max={:.2e} too large (log2={:.0})",
                            dc_max, dc_max_log2
                        )
                        .into(),
                    );
                }
                None
            };

            state.orbit_cache.insert(
                orbit_id,
                CachedOrbit {
                    c_ref,
                    orbit,
                    derivative,
                    escaped_at,
                    bla_table,
                },
            );
            post_message(&WorkerToMain::OrbitStored { orbit_id });
        }

        MainToWorker::RenderTilePerturbation {
            render_id,
            tile,
            orbit_id,
            delta_c_origin_json,
            delta_c_step_json,
            max_iterations,
            tau_sq,
            bigfloat_threshold_bits: _, // No longer used: pixel calculations use HDRFloat, not BigFloat
            bla_enabled,
            force_hdr_float,
        } => {
            // Parse BigFloat deltas from JSON
            let delta_c_origin: (BigFloat, BigFloat) =
                match serde_json::from_str(&delta_c_origin_json) {
                    Ok(d) => d,
                    Err(e) => {
                        post_message(&WorkerToMain::Error {
                            message: format!("Failed to parse delta_c_origin: {}", e),
                        });
                        return;
                    }
                };

            let delta_c_step: (BigFloat, BigFloat) = match serde_json::from_str(&delta_c_step_json)
            {
                Ok(d) => d,
                Err(e) => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Failed to parse delta_c_step: {}", e),
                    });
                    return;
                }
            };

            // Get cached orbit
            let cached = match state.orbit_cache.get(&orbit_id) {
                Some(c) => c,
                None => {
                    post_message(&WorkerToMain::Error {
                        message: format!("Orbit {} not found in cache", orbit_id),
                    });
                    return;
                }
            };

            let orbit = cached.to_reference_orbit();
            let start_time = Date::now();

            // Check if deltas fit in f64 range (roughly 10^-300 to 10^300)
            // log2 of ~10^-300 is about -1000, so we use -900 as safe threshold
            // force_hdr_float overrides this check for debugging deep zoom issues
            let delta_log2 = delta_c_origin
                .0
                .log2_approx()
                .max(delta_c_origin.1.log2_approx());
            let deltas_fit_f64 = !force_hdr_float && delta_log2 > -900.0 && delta_log2 < 900.0;

            let mut data = Vec::with_capacity((tile.width * tile.height) as usize);

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
                let delta_origin = (delta_c_origin.0.to_f64(), delta_c_origin.1.to_f64());
                let delta_step = (delta_c_step.0.to_f64(), delta_c_step.1.to_f64());

                let mut delta_c_row = delta_origin;

                for _py in 0..tile.height {
                    let mut delta_c = delta_c_row;

                    for _px in 0..tile.width {
                        let result = compute_pixel_perturbation(
                            &orbit,
                            F64Complex::from_f64_pair(delta_c.0, delta_c.1),
                            max_iterations,
                            tau_sq,
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
                    re: HDRFloat::from_bigfloat(&delta_c_origin.0),
                    im: HDRFloat::from_bigfloat(&delta_c_origin.1),
                };
                let delta_step = HDRComplex {
                    re: HDRFloat::from_bigfloat(&delta_c_step.0),
                    im: HDRFloat::from_bigfloat(&delta_c_step.1),
                };

                let mut delta_c_row = delta_origin;

                for _py in 0..tile.height {
                    let mut delta_c = delta_c_row;

                    for _px in 0..tile.width {
                        let result = if bla_enabled {
                            if let Some(ref bla_table) = cached.bla_table {
                                compute_pixel_perturbation_hdr_bla(
                                    &orbit,
                                    bla_table,
                                    delta_c,
                                    max_iterations,
                                    tau_sq,
                                )
                            } else {
                                // Fallback if table wasn't built
                                compute_pixel_perturbation(&orbit, delta_c, max_iterations, tau_sq)
                            }
                        } else {
                            compute_pixel_perturbation(&orbit, delta_c, max_iterations, tau_sq)
                        };
                        data.push(ComputeData::Mandelbrot(result));

                        delta_c.re = delta_c.re.add(&delta_step.re);
                    }

                    delta_c_row.im = delta_c_row.im.add(&delta_step.im);
                }
            }

            let compute_time_ms = Date::now() - start_time;

            post_message(&WorkerToMain::TileComplete {
                render_id,
                tile,
                data,
                compute_time_ms,
            });

            post_message(&WorkerToMain::RequestWork {
                render_id: Some(render_id),
            });
        }

        MainToWorker::DiscardOrbit { orbit_id } => {
            state.orbit_cache.remove(&orbit_id);
        }
    }
}

/// Entry point called by worker JS loader.
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();

    web_sys::console::log_1(&"[Worker] Started".into());

    let state = Rc::new(RefCell::new(WorkerState::new()));

    let state_clone = Rc::clone(&state);
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        handle_message(&mut state_clone.borrow_mut(), e.data());
    }) as Box<dyn FnMut(_)>);

    let global: web_sys::DedicatedWorkerGlobalScope =
        js_sys::global().dyn_into().expect("Not in worker context");

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Signal ready for initialization
    post_message(&WorkerToMain::Ready);
}
