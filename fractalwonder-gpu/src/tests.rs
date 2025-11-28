//! Tests for GPU renderer.

use crate::{GpuAvailability, GpuContext, GpuRenderer};

/// Test that GPU initialization doesn't panic.
#[test]
fn gpu_init_does_not_panic() {
    // This test verifies the initialization code path runs without panic.
    // On systems without GPU, it should return Unavailable gracefully.
    pollster::block_on(async {
        let result = GpuContext::try_init().await;
        match result {
            GpuAvailability::Available(_) => {
                println!("GPU available");
            }
            GpuAvailability::Unavailable(reason) => {
                println!("GPU unavailable: {reason}");
            }
        }
    });
}

/// Test basic render on GPU (if available).
#[test]
fn gpu_render_basic() {
    pollster::block_on(async {
        let GpuAvailability::Available(ctx) = GpuContext::try_init().await else {
            println!("Skipping test: no GPU available");
            return;
        };

        let mut renderer = GpuRenderer::new(ctx);

        // Simple reference orbit: z=0 -> z=c -> z=c^2+c -> ...
        // For c = (0, 0), orbit is all zeros
        let orbit = vec![(0.0_f64, 0.0_f64); 100];

        let result = renderer
            .render(
                &orbit,
                1,            // orbit_id
                (-2.0, -1.5), // dc_origin
                (0.01, 0.01), // dc_step
                100,          // width
                100,          // height
                100,          // max_iterations
                1e-6,         // tau_sq
            )
            .await
            .expect("Render should succeed");

        assert_eq!(result.iterations.len(), 100 * 100);
        assert_eq!(result.glitch_flags.len(), 100 * 100);
        println!("GPU render completed in {:.2}ms", result.compute_time_ms);
    });
}
