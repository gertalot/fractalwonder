use crate::{ComputeData, HDRFloat, PixelRect};
use serde::{Deserialize, Serialize};

/// Messages sent from main thread to worker.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum MainToWorker {
    /// Initialize worker with specified renderer type.
    Initialize { renderer_id: String },

    /// Render a tile. viewport_json is JSON-serialized Viewport to preserve BigFloat precision.
    RenderTile {
        render_id: u32,
        viewport_json: String,
        tile: PixelRect,
    },

    /// No work available - worker should idle.
    NoWork,

    /// Terminate worker.
    Terminate,

    /// Compute a reference orbit at high precision.
    ComputeReferenceOrbit {
        render_id: u32,
        orbit_id: u32,
        c_ref_json: String,
        max_iterations: u32,
    },

    /// Store a reference orbit for use in tile rendering.
    StoreReferenceOrbit {
        orbit_id: u32,
        c_ref: (f64, f64),
        orbit: Vec<(f64, f64)>,
        derivative: Vec<(f64, f64)>,
        escaped_at: Option<u32>,
        /// Maximum |δc| for any pixel in viewport (HDRFloat to avoid underflow at extreme zoom)
        dc_max: HDRFloat,
        /// Whether to build BLA table for this orbit
        bla_enabled: bool,
    },

    /// Render a tile using perturbation with extended precision deltas.
    RenderTilePerturbation {
        render_id: u32,
        tile: PixelRect,
        orbit_id: u32,
        /// JSON-serialized (BigFloat, BigFloat) for delta_c at tile origin
        delta_c_origin_json: String,
        /// JSON-serialized (BigFloat, BigFloat) for delta_c step per pixel
        delta_c_step_json: String,
        max_iterations: u32,
        /// Glitch detection threshold squared (τ²).
        tau_sq: f64,
        /// Precision threshold for BigFloat arithmetic (bits).
        /// Below this, use fast f64; above, use BigFloat.
        bigfloat_threshold_bits: usize,
        /// Enable BLA (Bivariate Linear Approximation) for iteration skipping.
        bla_enabled: bool,
        /// Force HDRFloat for all calculations (debug option).
        force_hdr_float: bool,
    },

    /// Discard a cached orbit.
    DiscardOrbit { orbit_id: u32 },
}

/// Messages sent from worker to main thread.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WorkerToMain {
    /// Worker is loaded and ready for initialization.
    Ready,

    /// Worker requests work assignment.
    /// render_id is None after Initialize, Some(id) after completing work for that render.
    RequestWork { render_id: Option<u32> },

    /// Worker completed a tile.
    TileComplete {
        render_id: u32,
        tile: PixelRect,
        data: Vec<ComputeData>,
        compute_time_ms: f64,
    },

    /// Worker encountered an error.
    Error { message: String },

    /// Reference orbit computation complete.
    ReferenceOrbitComplete {
        render_id: u32,
        orbit_id: u32,
        c_ref: (f64, f64),
        orbit: Vec<(f64, f64)>,
        derivative: Vec<(f64, f64)>,
        escaped_at: Option<u32>,
    },

    /// Orbit stored and ready.
    OrbitStored { orbit_id: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_to_worker_initialize_roundtrip() {
        let msg = MainToWorker::Initialize {
            renderer_id: "mandelbrot".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Initialize""#));
        assert!(json.contains(r#""renderer_id":"mandelbrot""#));

        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::Initialize { renderer_id } => assert_eq!(renderer_id, "mandelbrot"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn main_to_worker_render_tile_roundtrip() {
        let msg = MainToWorker::RenderTile {
            render_id: 42,
            viewport_json: r#"{"center":...}"#.to_string(),
            tile: PixelRect::new(10, 20, 64, 64),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::RenderTile {
                render_id, tile, ..
            } => {
                assert_eq!(render_id, 42);
                assert_eq!(tile.x, 10);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn worker_to_main_ready_roundtrip() {
        let msg = WorkerToMain::Ready;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Ready""#));

        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, WorkerToMain::Ready));
    }

    #[test]
    fn worker_to_main_tile_complete_roundtrip() {
        use crate::MandelbrotData;

        let msg = WorkerToMain::TileComplete {
            render_id: 1,
            tile: PixelRect::new(0, 0, 64, 64),
            data: vec![ComputeData::Mandelbrot(MandelbrotData {
                iterations: 100,
                max_iterations: 1000,
                escaped: true,
                glitched: false,
                final_z_norm_sq: 0.0,
                surface_normal_re: 0.0,
                surface_normal_im: 0.0,
            })],
            compute_time_ms: 12.5,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        match parsed {
            WorkerToMain::TileComplete {
                render_id, data, ..
            } => {
                assert_eq!(render_id, 1);
                assert_eq!(data.len(), 1);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn compute_reference_orbit_roundtrip() {
        let msg = MainToWorker::ComputeReferenceOrbit {
            render_id: 1,
            orbit_id: 42,
            c_ref_json: r#"{"x":"-0.5","y":"0.0"}"#.to_string(),
            max_iterations: 10000,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::ComputeReferenceOrbit {
                orbit_id,
                max_iterations,
                ..
            } => {
                assert_eq!(orbit_id, 42);
                assert_eq!(max_iterations, 10000);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn store_reference_orbit_roundtrip() {
        let msg = MainToWorker::StoreReferenceOrbit {
            orbit_id: 1,
            c_ref: (-0.5, 0.0),
            orbit: vec![(0.0, 0.0), (-0.5, 0.0), (-0.25, 0.0)],
            derivative: vec![(0.0, 0.0), (1.0, 0.0), (1.5, 0.0)],
            escaped_at: None,
            dc_max: HDRFloat::from_f64(0.01),
            bla_enabled: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::StoreReferenceOrbit {
                orbit_id,
                orbit,
                derivative,
                ..
            } => {
                assert_eq!(orbit_id, 1);
                assert_eq!(orbit.len(), 3);
                assert_eq!(derivative.len(), 3);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn render_tile_perturbation_roundtrip() {
        use crate::BigFloat;

        let delta_origin = (
            BigFloat::from_string("1e-500", 2048).unwrap(),
            BigFloat::from_string("-2e-500", 2048).unwrap(),
        );
        let delta_step = (
            BigFloat::from_string("1e-503", 2048).unwrap(),
            BigFloat::from_string("1e-503", 2048).unwrap(),
        );

        let msg = MainToWorker::RenderTilePerturbation {
            render_id: 1,
            tile: PixelRect::new(0, 0, 64, 64),
            orbit_id: 42,
            delta_c_origin_json: serde_json::to_string(&delta_origin).unwrap(),
            delta_c_step_json: serde_json::to_string(&delta_step).unwrap(),
            max_iterations: 10000,
            tau_sq: 1e-6,
            bigfloat_threshold_bits: 1024,
            bla_enabled: true,
            force_hdr_float: false,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::RenderTilePerturbation {
                orbit_id,
                delta_c_origin_json,
                tau_sq,
                ..
            } => {
                assert_eq!(orbit_id, 42);
                assert!((tau_sq - 1e-6).abs() < 1e-12);

                // Verify BigFloat survives roundtrip
                let parsed_origin: (BigFloat, BigFloat) =
                    serde_json::from_str(&delta_c_origin_json).unwrap();
                assert_eq!(parsed_origin.0.precision_bits(), 2048);

                // Verify extreme value preserved
                let log2 = parsed_origin.0.log2_approx();
                assert!(log2 < -1600.0, "Delta should be ~10^-500");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn reference_orbit_complete_roundtrip() {
        let msg = WorkerToMain::ReferenceOrbitComplete {
            render_id: 1,
            orbit_id: 42,
            c_ref: (-0.5, 0.0),
            orbit: vec![(0.0, 0.0), (-0.5, 0.0)],
            derivative: vec![(0.0, 0.0), (1.0, 0.0)],
            escaped_at: Some(1000),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        match parsed {
            WorkerToMain::ReferenceOrbitComplete {
                orbit_id,
                escaped_at,
                ..
            } => {
                assert_eq!(orbit_id, 42);
                assert_eq!(escaped_at, Some(1000));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn orbit_stored_roundtrip() {
        let msg = WorkerToMain::OrbitStored { orbit_id: 42 };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WorkerToMain = serde_json::from_str(&json).unwrap();
        match parsed {
            WorkerToMain::OrbitStored { orbit_id } => assert_eq!(orbit_id, 42),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn store_reference_orbit_with_dc_max_roundtrip() {
        let msg = MainToWorker::StoreReferenceOrbit {
            orbit_id: 1,
            c_ref: (-0.5, 0.0),
            orbit: vec![(0.0, 0.0), (-0.5, 0.0)],
            derivative: vec![(0.0, 0.0), (1.0, 0.0)],
            escaped_at: None,
            dc_max: HDRFloat::from_f64(0.001),
            bla_enabled: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: MainToWorker = serde_json::from_str(&json).unwrap();
        match parsed {
            MainToWorker::StoreReferenceOrbit { dc_max, .. } => {
                assert!((dc_max.to_f64() - 0.001).abs() < 1e-12);
            }
            _ => panic!("Wrong variant"),
        }
    }

    // =========================================================================
    // Phase 3: Precision Preservation Tests
    // =========================================================================

    use crate::BigFloat;

    #[test]
    fn bigfloat_json_roundtrip_preserves_precision() {
        // Test with high-precision value that would lose precision in f64
        let original = BigFloat::from_string(
            "-1.10000101110000011001011000111011011110110100100101010010110010101111001",
            128,
        )
        .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let parsed: BigFloat = serde_json::from_str(&json).unwrap();

        // Precision bits must be preserved
        assert_eq!(
            original.precision_bits(),
            parsed.precision_bits(),
            "Precision bits should be preserved"
        );

        // Values must be equal
        assert_eq!(
            original, parsed,
            "Value should be preserved through roundtrip"
        );
    }

    #[test]
    fn bigfloat_json_roundtrip_preserves_extreme_precision() {
        // Test with extreme precision (beyond f64 range)
        let original = BigFloat::from_string("1.23456789e-500", 512).unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let parsed: BigFloat = serde_json::from_str(&json).unwrap();

        assert_eq!(original.precision_bits(), parsed.precision_bits());
        assert_eq!(original, parsed);
    }

    #[test]
    fn bigfloat_json_format_is_human_readable() {
        let bf = BigFloat::from_string("-0.75", 128).unwrap();
        let json = serde_json::to_string(&bf).unwrap();

        // JSON should contain readable fields
        assert!(json.contains("value"), "JSON should contain 'value' field");
        assert!(
            json.contains("precision_bits"),
            "JSON should contain 'precision_bits' field"
        );
        assert!(json.contains("128"), "JSON should contain precision value");
    }

    #[test]
    fn viewport_coordinates_survive_roundtrip() {
        // Real-world deep zoom coordinates (like from the glitchy tile data in the plan)
        // These coordinates represent a point at ~10^14 zoom depth
        let x = BigFloat::from_string(
            "-1.1000010111000001100101100011101101111011010010010101001011001010111100100000011110010",
            128,
        )
        .unwrap();
        let y = BigFloat::from_string("0.23456789012345678901234567890123456789", 128).unwrap();

        // Serialize as tuple (like c_ref in messages)
        let coords = (x.clone(), y.clone());
        let json = serde_json::to_string(&coords).unwrap();
        let parsed: (BigFloat, BigFloat) = serde_json::from_str(&json).unwrap();

        assert_eq!(coords.0, parsed.0, "X coordinate should survive roundtrip");
        assert_eq!(coords.1, parsed.1, "Y coordinate should survive roundtrip");
        assert_eq!(
            coords.0.precision_bits(),
            parsed.0.precision_bits(),
            "X precision should be preserved"
        );
        assert_eq!(
            coords.1.precision_bits(),
            parsed.1.precision_bits(),
            "Y precision should be preserved"
        );
    }

    #[test]
    fn viewport_json_field_roundtrip() {
        // Test the actual message pattern used: viewport_json is a JSON string
        // containing BigFloat coordinates
        use crate::Viewport;

        let viewport = Viewport::from_strings(
            "-1.10000101110000011001011000111011011110110100100101010010110010101111001",
            "0.23456789",
            "0.0000001",
            "0.0000001",
            128,
        )
        .unwrap();

        // Serialize viewport to JSON string (as done in RenderTile message)
        let viewport_json = serde_json::to_string(&viewport).unwrap();

        // Put in message
        let msg = MainToWorker::RenderTile {
            render_id: 1,
            viewport_json: viewport_json.clone(),
            tile: PixelRect::new(0, 0, 32, 32),
        };

        // Roundtrip the message
        let msg_json = serde_json::to_string(&msg).unwrap();
        let parsed_msg: MainToWorker = serde_json::from_str(&msg_json).unwrap();

        match parsed_msg {
            MainToWorker::RenderTile {
                viewport_json: parsed_vp_json,
                ..
            } => {
                // Parse the inner viewport JSON
                let parsed_viewport: Viewport = serde_json::from_str(&parsed_vp_json).unwrap();

                // Center coordinates precision should match
                assert_eq!(
                    viewport.center.0.precision_bits(),
                    parsed_viewport.center.0.precision_bits(),
                    "Viewport center_x precision should be preserved"
                );

                // Values should match
                assert_eq!(
                    viewport.center.0, parsed_viewport.center.0,
                    "Viewport center_x should be preserved"
                );
            }
            _ => panic!("Wrong variant"),
        }
    }
}
