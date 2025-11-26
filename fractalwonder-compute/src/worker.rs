// fractalwonder-compute/src/worker.rs
use crate::{MandelbrotRenderer, Renderer, TestImageRenderer};
use fractalwonder_core::{ComputeData, MainToWorker, Viewport, WorkerToMain};
use js_sys::Date;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Boxed renderer trait object for dynamic dispatch.
type BoxedRenderer = Box<dyn Renderer<Data = ComputeData>>;

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
    if let Ok(json) = serde_json::to_string(msg) {
        let global: web_sys::DedicatedWorkerGlobalScope =
            js_sys::global().dyn_into().expect("Not in worker context");
        let _ = global.post_message(&JsValue::from_str(&json));
    }
}

fn handle_message(renderer: &Rc<RefCell<Option<BoxedRenderer>>>, data: JsValue) {
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
                    *renderer.borrow_mut() = Some(r);
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
            let borrowed = renderer.borrow();
            let Some(r) = borrowed.as_ref() else {
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

        // Perturbation theory messages - will be implemented in Task 6-8
        MainToWorker::ComputeReferenceOrbit { .. } => {
            post_message(&WorkerToMain::Error {
                message: "ComputeReferenceOrbit not yet implemented".to_string(),
            });
        }

        MainToWorker::StoreReferenceOrbit { .. } => {
            post_message(&WorkerToMain::Error {
                message: "StoreReferenceOrbit not yet implemented".to_string(),
            });
        }

        MainToWorker::RenderTilePerturbation { .. } => {
            post_message(&WorkerToMain::Error {
                message: "RenderTilePerturbation not yet implemented".to_string(),
            });
        }

        MainToWorker::DiscardOrbit { .. } => {
            // Silently ignore for now
        }
    }
}

/// Entry point called by worker JS loader.
#[wasm_bindgen]
pub fn init_message_worker() {
    console_error_panic_hook::set_once();

    web_sys::console::log_1(&"[Worker] Started".into());

    let renderer: Rc<RefCell<Option<BoxedRenderer>>> = Rc::new(RefCell::new(None));

    let renderer_clone = Rc::clone(&renderer);
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        handle_message(&renderer_clone, e.data());
    }) as Box<dyn FnMut(_)>);

    let global: web_sys::DedicatedWorkerGlobalScope =
        js_sys::global().dyn_into().expect("Not in worker context");

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // Signal ready for initialization
    post_message(&WorkerToMain::Ready);
}
