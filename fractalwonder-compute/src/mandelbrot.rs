use crate::Renderer;
use fractalwonder_core::{pixel_to_fractal, MandelbrotData, Viewport};

/// Simple Mandelbrot set renderer using escape-time algorithm with f64 arithmetic.
///
/// This renderer is suitable for shallow zoom levels where f64 precision is sufficient.
/// For deep zoom (beyond ~10^14), use perturbation-based rendering instead.
///
/// NOTE: BigFloat is intentionally NOT used here. Pixel calculations should only use
/// f64 or HDRFloat. BigFloat should only be used for reference orbit computation.
pub struct MandelbrotRenderer {
    max_iterations: u32,
}

impl MandelbrotRenderer {
    pub fn new(max_iterations: u32) -> Self {
        Self { max_iterations }
    }
}

impl Renderer for MandelbrotRenderer {
    type Data = MandelbrotData;

    fn render(&self, viewport: &Viewport, canvas_size: (u32, u32)) -> Vec<MandelbrotData> {
        let (width, height) = canvas_size;
        let precision = viewport.precision_bits();

        (0..height)
            .flat_map(|py| {
                (0..width).map(move |px| {
                    let (cx_bf, cy_bf) =
                        pixel_to_fractal(px as f64, py as f64, viewport, canvas_size, precision);
                    // Convert to f64 for pixel iteration - this renderer is for shallow zoom only
                    let cx = cx_bf.to_f64();
                    let cy = cy_bf.to_f64();
                    self.compute_point(cx, cy)
                })
            })
            .collect()
    }
}

impl MandelbrotRenderer {
    /// Compute Mandelbrot iteration for a single point using f64 arithmetic.
    fn compute_point(&self, cx: f64, cy: f64) -> MandelbrotData {
        let mut zx = 0.0_f64;
        let mut zy = 0.0_f64;
        const ESCAPE_RADIUS_SQ: f64 = 65536.0;

        for i in 0..self.max_iterations {
            let zx_sq = zx * zx;
            let zy_sq = zy * zy;

            // Escape check: |z|^2 > 65536
            let z_norm_sq = zx_sq + zy_sq;
            if z_norm_sq > ESCAPE_RADIUS_SQ {
                // No derivative tracking in simple Mandelbrot renderer
                return MandelbrotData::new(
                    i,
                    self.max_iterations,
                    true,
                    false,
                    z_norm_sq as f32,
                    0.0,
                    0.0,
                );
            }

            // z = z^2 + c
            // new_zx = zx^2 - zy^2 + cx
            // new_zy = 2*zx*zy + cy
            let new_zx = zx_sq - zy_sq + cx;
            let new_zy = 2.0 * zx * zy + cy;
            zx = new_zx;
            zy = new_zy;
        }

        // Interior point - no surface normal
        MandelbrotData {
            iterations: self.max_iterations,
            max_iterations: self.max_iterations,
            escaped: false,
            glitched: false,
            final_z_norm_sq: 0.0,
            surface_normal_re: 0.0,
            surface_normal_im: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_produces_correct_size() {
        let renderer = MandelbrotRenderer::new(100);
        let vp = Viewport::from_f64(-0.5, 0.0, 4.0, 4.0, 128);
        let result = renderer.render(&vp, (100, 50));
        assert_eq!(result.len(), 100 * 50);
    }

    #[test]
    fn origin_is_in_set() {
        // Point (0, 0) is in the Mandelbrot set
        let renderer = MandelbrotRenderer::new(100);
        let result = renderer.compute_point(0.0, 0.0);
        assert!(!result.escaped, "Origin should be in set");
        assert_eq!(result.iterations, 100);
        assert_eq!(result.max_iterations, 100);
    }

    #[test]
    fn point_outside_escapes_quickly() {
        // Point (2, 0) escapes: z0=0, z1=2, z2=6, z3=38, z4=1446, ... |z6|^2 > 65536
        let renderer = MandelbrotRenderer::new(100);
        let result = renderer.compute_point(2.0, 0.0);
        assert!(result.escaped, "Point (2,0) should escape");
        assert!(result.iterations < 10, "Should escape quickly");
    }

    #[test]
    fn point_far_outside_escapes_at_zero() {
        // Point (10, 0): |c|^2 = 100, escapes quickly
        let renderer = MandelbrotRenderer::new(100);
        let result = renderer.compute_point(10.0, 0.0);
        assert!(result.escaped);
        // z0=0, z1=10, z2=110, z3=12110, |z4|^2 > 65536
        assert!(result.iterations < 5, "Should escape very quickly");
    }

    #[test]
    fn point_on_boundary_high_iterations() {
        // Point (-0.75, 0.1) is near the boundary, should take many iterations
        let renderer = MandelbrotRenderer::new(1000);
        let result = renderer.compute_point(-0.75, 0.1);
        // This point eventually escapes but takes many iterations
        assert!(result.escaped);
        assert!(
            result.iterations > 10,
            "Boundary point should take many iterations"
        );
    }

    #[test]
    fn main_cardioid_point_in_set() {
        // Point (-0.5, 0) is in the main cardioid
        let renderer = MandelbrotRenderer::new(500);
        let result = renderer.compute_point(-0.5, 0.0);
        assert!(!result.escaped, "Point (-0.5, 0) should be in set");
    }

    #[test]
    fn max_iterations_stored_in_result() {
        let renderer = MandelbrotRenderer::new(500);
        let result = renderer.compute_point(0.0, 0.0);
        assert_eq!(result.max_iterations, 500);
    }
}
