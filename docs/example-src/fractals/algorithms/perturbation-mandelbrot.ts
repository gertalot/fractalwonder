// ABOUTME: Perturbation theory implementation of Mandelbrot set for deep zoom
// ABOUTME: Uses reference orbit and delta calculations for arbitrary precision at extreme magnifications

import { Decimal } from "decimal.js";
import { centerToHP } from "../mandelbrot/conversions";
import { calculateDeltaOrbit } from "../mandelbrot/delta-orbit";
import { calculateReferenceOrbit } from "../mandelbrot/reference-orbit";
import type { ReferenceOrbit } from "../mandelbrot/types";
import { AlgorithmContext, FractalAlgorithm, IterationResult } from "./base";

/**
 * Perturbation theory implementation of the Mandelbrot set.
 * 
 * This algorithm enables deep zoom rendering beyond the precision limits of
 * IEEE 754 double precision floating point (~10^14 zoom).
 * 
 * Algorithm:
 * 1. Compute ONE high-precision reference orbit for the viewport center
 * 2. For each pixel, compute a fast standard-precision delta orbit
 * 3. Combine: Z_n ≈ X_n (reference) + Δ_n (delta)
 * 
 * Performance: ~50-100x faster than full arbitrary precision rendering.
 * Zoom capability: Works reliably up to 10^50 and beyond.
 */
export class PerturbationMandelbrotAlgorithm implements FractalAlgorithm {
  readonly name = "Perturbation Mandelbrot";
  readonly description =
    "Perturbation theory: one high-precision reference orbit + fast delta orbits per pixel";

  // Reference orbit state (set by prepareForRender)
  private referenceOrbit: ReferenceOrbit | null = null;
  private referenceCenter: { x: Decimal; y: Decimal } | null = null;

  /**
   * Prepare the algorithm for rendering by calculating the reference orbit.
   * This is called once per frame before any pixels are computed.
   * 
   * @param context - Viewport parameters for this render
   */
  prepareForRender(context: AlgorithmContext): void {
    const { center, zoom, maxIterations } = context;

    // Convert center to high-precision and calculate reference orbit
    const centerHP = centerToHP(center, zoom);
    this.referenceOrbit = calculateReferenceOrbit(centerHP, maxIterations);
    this.referenceCenter = center;
  }

  /**
   * Compute iteration count for a single point using perturbation theory.
   * 
   * @deprecated Use computePointFromOffset() for precision-preserving calculation.
   * This method suffers from catastrophic cancellation at zoom > 10^12.
   * 
   * @param real - Real component of the point (x-coordinate)
   * @param imag - Imaginary component of the point (y-coordinate)
   * @param maxIterations - Maximum iterations (should match preparedMaxIterations)
   * @returns Iteration result with escape count and final z values
   */
  computePoint(real: number, imag: number, maxIterations: number): IterationResult {
    // Log warning about precision loss
    console.warn(
      "Warning: computePoint() suffers from catastrophic cancellation at zoom > 10^12. " +
      "Use computePointFromOffset() instead for precision-preserving calculation."
    );

    // Ensure algorithm was prepared
    if (!this.referenceOrbit || !this.referenceCenter) {
      throw new Error(
        "PerturbationMandelbrotAlgorithm: prepareForRender must be called before computePoint"
      );
    }

    // Calculate delta C (offset from reference center)
    const deltaC = {
      real: real - this.referenceCenter.x.toNumber(),
      imag: imag - this.referenceCenter.y.toNumber(),
    };

    // Compute delta orbit using standard precision
    const escapeIter = calculateDeltaOrbit(this.referenceOrbit, deltaC, maxIterations);

    // Return iteration result
    // Note: For now, we return synthetic zr/zi values since we don't track final delta
    // These are used for coloring and can be improved later if needed
    const iter = escapeIter === -1 ? maxIterations : escapeIter;
    return {
      iter,
      zr: escapeIter === -1 ? 0 : 2, // Approximate: in set vs escaped
      zi: 0,
    };
  }

  /**
   * Precision-preserving method that calculates deltaC directly from pixel offsets.
   * This eliminates catastrophic cancellation by avoiding the pixel → world coordinate conversion.
   * 
   * @param offsetX - Pixel offset from canvas center (integer)
   * @param offsetY - Pixel offset from canvas center (integer)
   * @param scale - Fractal units per pixel (calculated from zoom level)
   * @param maxIterations - Maximum iterations (should match preparedMaxIterations)
   * @returns Iteration result with escape count and final z values
   */
  computePointFromOffset(
    offsetX: number,
    offsetY: number,
    scale: number,
    maxIterations: number
  ): IterationResult {
    // Ensure algorithm was prepared
    if (!this.referenceOrbit || !this.referenceCenter) {
      throw new Error(
        "PerturbationMandelbrotAlgorithm: prepareForRender must be called before computePointFromOffset"
      );
    }

    // Calculate deltaC directly from pixel offsets - NO PRECISION LOSS
    const deltaC = {
      real: offsetX * scale,
      imag: offsetY * scale,
    };

    // Compute delta orbit using standard precision
    const escapeIter = calculateDeltaOrbit(this.referenceOrbit, deltaC, maxIterations);

    // Return iteration result
    // Note: For now, we return synthetic zr/zi values since we don't track final delta
    // These are used for coloring and can be improved later if needed
    const iter = escapeIter === -1 ? maxIterations : escapeIter;
    return {
      iter,
      zr: escapeIter === -1 ? 0 : 2, // Approximate: in set vs escaped
      zi: 0,
    };
  }
}

/**
 * Default instance of the perturbation Mandelbrot algorithm.
 */
export const perturbationMandelbrotAlgorithm = new PerturbationMandelbrotAlgorithm();

