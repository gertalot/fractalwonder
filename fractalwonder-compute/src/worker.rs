// fractalwonder-compute/src/worker.rs
use crate::{render_tile_f64, render_tile_hdr, BlaTable, ReferenceOrbit, TileConfig};
use fractalwonder_core::{BigFloat, HDRFloat, MainToWorker, WorkerToMain};
use js_sys::Date;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

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

/// Worker state for orbit cache.
struct WorkerState {
    orbit_cache: HashMap<u32, CachedOrbit>,
}

impl WorkerState {
    fn new() -> Self {
        Self {
            orbit_cache: HashMap::new(),
        }
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
            // dc_max is now HDRFloat to prevent underflow at deep zoom
            let dc_max_log2 = if dc_max.is_zero() {
                f64::NEG_INFINITY
            } else {
                // log2(head * 2^exp) = log2(head) + exp
                (dc_max.head as f64).log2() + dc_max.exp as f64
            };
            let bla_useful = dc_max_log2 < -80.0; // Roughly 10^-25 (scale > 1e25)

            let bla_table = if bla_enabled && bla_useful {
                let ref_orbit = ReferenceOrbit {
                    c_ref,
                    orbit: orbit.clone(),
                    derivative: derivative.clone(),
                    escaped_at,
                };
                let table = BlaTable::compute(&ref_orbit, &dc_max);
                web_sys::console::log_1(
                    &format!(
                        "[Worker] Built BLA table: {} entries, {} levels (dc_max: head={:.2e}, exp={})",
                        table.entries.len(),
                        table.num_levels,
                        dc_max.head,
                        dc_max.exp
                    )
                    .into(),
                );
                Some(table)
            } else {
                if bla_enabled && !bla_useful {
                    web_sys::console::log_1(
                        &format!(
                            "[Worker] Skipping BLA table: dc_max (head={:.2e}, exp={}) too large (log2={:.0})",
                            dc_max.head, dc_max.exp, dc_max_log2
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
            bigfloat_threshold_bits: _,
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

            let config = TileConfig {
                size: (tile.width, tile.height),
                max_iterations,
                tau_sq,
                bla_enabled,
            };

            // Dispatch based on delta magnitude: use f64 when deltas fit, HDRFloat otherwise
            let delta_log2 = delta_c_origin
                .0
                .log2_approx()
                .max(delta_c_origin.1.log2_approx());
            let use_f64 = !force_hdr_float && delta_log2 > -900.0 && delta_log2 < 900.0;

            let result = if use_f64 {
                let delta_origin = (delta_c_origin.0.to_f64(), delta_c_origin.1.to_f64());
                let delta_step = (delta_c_step.0.to_f64(), delta_c_step.1.to_f64());
                render_tile_f64(
                    &orbit,
                    cached.bla_table.as_ref(),
                    delta_origin,
                    delta_step,
                    &config,
                )
            } else {
                let delta_origin = (
                    HDRFloat::from_bigfloat(&delta_c_origin.0),
                    HDRFloat::from_bigfloat(&delta_c_origin.1),
                );
                let delta_step = (
                    HDRFloat::from_bigfloat(&delta_c_step.0),
                    HDRFloat::from_bigfloat(&delta_c_step.1),
                );
                render_tile_hdr(
                    &orbit,
                    cached.bla_table.as_ref(),
                    delta_origin,
                    delta_step,
                    &config,
                )
            };

            let compute_time_ms = Date::now() - start_time;

            post_message(&WorkerToMain::TileComplete {
                render_id,
                tile,
                data: result.data,
                compute_time_ms,
                bla_iterations: result.stats.bla_iterations,
                total_iterations: result.stats.total_iterations,
                rebase_count: result.stats.rebase_count,
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
