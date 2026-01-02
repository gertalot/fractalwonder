//! Diagnostic test comparing CPU and GPU Mandelbrot renderers.
//!
//! Renders identical pixels through both pipelines and prints MandelbrotData
//! field-by-field to diagnose rendering differences.

#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports, dead_code)] // Staged for subsequent implementation tasks
mod tests {
    use fractalwonder_compute::{render_tile_hdr, BlaTable, ReferenceOrbit, TileConfig};
    use fractalwonder_core::{BigFloat, ComputeData, HDRFloat, MandelbrotData, Viewport};
    use fractalwonder_gpu::{GpuAvailability, GpuContext, ProgressiveGpuRenderer};

    /// Test viewport parameters (extreme deep zoom ~10^-281)
    const CENTER_X: &str =
        "0.273000307495579097715200094310253922494103490187797182966812629706330340783242";
    const CENTER_Y: &str =
        "0.005838718497531293679839354462882728828030188792949767250660666951674130465532";
    const WIDTH: &str = "1.38277278476513331960149825811900065907944121299848E-281";
    const HEIGHT: &str = "7.97822331184022584815185255533429968247789646588334E-282";

    const IMAGE_WIDTH: u32 = 766;
    const IMAGE_HEIGHT: u32 = 432;
    const TEST_ROW: u32 = 350;
    const TEST_COL_START: u32 = 580;
    const TEST_COL_END: u32 = 611; // 32 pixels
    const MAX_ITERATIONS: u32 = 10_000_000;
    const TAU_SQ: f64 = 1e-6;
    const PRECISION_BITS: usize = 1067;

    fn parse_viewport() -> Viewport {
        Viewport::from_strings(CENTER_X, CENTER_Y, WIDTH, HEIGHT, PRECISION_BITS)
            .expect("valid viewport parameters")
    }

    #[test]
    fn compare_cpu_gpu_mandelbrot_output() {
        let viewport = parse_viewport();
        println!("Viewport parsed:");
        println!(
            "  width exponent: ~10^{}",
            viewport.width.log2_approx() as i32 * 301 / 1000
        );
        println!("  precision: {} bits", PRECISION_BITS);
    }
}
