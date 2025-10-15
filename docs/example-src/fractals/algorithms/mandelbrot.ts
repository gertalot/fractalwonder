// ABOUTME: Mandelbrot set algorithm implementation
// ABOUTME: Computes escape-time iterations for points in the complex plane

import { FractalAlgorithm, IterationResult } from "./base";

/**
 * Mandelbrot Set algorithm implementation.
 *
 * The Mandelbrot set is defined as the set of complex numbers c for which
 * the function f(z) = z² + c does not diverge when iterated from z = 0.
 *
 * For each point c in the complex plane, we iterate:
 *   z₀ = 0
 *   z_{n+1} = z_n² + c
 *
 * We count how many iterations it takes for |z| to exceed 2 (i.e., |z|² > 4).
 * Points that never escape (reach maxIterations) are considered part of the set.
 */
export class MandelbrotAlgorithm implements FractalAlgorithm {
  readonly name = "Mandelbrot Set";
  readonly description =
    "The classic Mandelbrot set: z → z² + c, starting from z = 0";

  /**
   * Computes the escape-time iteration count for a point in the Mandelbrot set.
   *
   * @param real - Real component of c (x-coordinate in the complex plane)
   * @param imag - Imaginary component of c (y-coordinate in the complex plane)
   * @param maxIterations - Maximum iterations before assuming point is in the set
   * @returns IterationResult with iteration count and final z values
   */
  computePoint(real: number, imag: number, maxIterations: number): IterationResult {
    let zr = 0; // Real component of z
    let zi = 0; // Imaginary component of z
    let iter = 0;

    // Iterate z = z² + c until |z|² > 4 or we reach maxIterations
    // |z|² = zr² + zi² > 4 means the point has escaped
    while (zr * zr + zi * zi < 4 && iter < maxIterations) {
      // Compute z² + c
      // (zr + zi*i)² = zr² - zi² + 2*zr*zi*i
      const newZr = zr * zr - zi * zi + real;
      zi = 2 * zr * zi + imag;
      zr = newZr;
      iter++;
    }

    return { iter, zr, zi };
  }
}

/**
 * Default instance of the Mandelbrot algorithm for convenient importing.
 */
export const mandelbrotAlgorithm = new MandelbrotAlgorithm();

