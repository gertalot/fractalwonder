import { Decimal } from "decimal.js";

/**
 * Result of computing iterations for a single point in the complex plane.
 * Used by fractal algorithms to return iteration count and final z values.
 */
export interface IterationResult {
  /** Number of iterations before escape (or maxIterations if point is in the set) */
  iter: number;
  /** Real component of final z value */
  zr: number;
  /** Imaginary component of final z value */
  zi: number;
}

/**
 * Context for preparing an algorithm before rendering.
 * Used by advanced algorithms (e.g., perturbation theory) that need per-frame setup.
 */
export interface AlgorithmContext {
  /** Center point of the viewport in fractal coordinates */
  center: { x: Decimal; y: Decimal };
  /** Zoom level */
  zoom: Decimal;
  /** Maximum iterations for this render */
  maxIterations: number;
}

/**
 * Abstract interface that all fractal algorithms must implement.
 * This enables algorithm swapping and parallel execution via Web Workers.
 */
export interface FractalAlgorithm {
  /** Human-readable name of the algorithm (e.g., "Mandelbrot Set") */
  readonly name: string;

  /** Optional description explaining the algorithm */
  readonly description?: string;

  /**
   * Optional preparation method called before rendering a frame.
   * Advanced algorithms can use this to perform one-time calculations
   * (e.g., computing a reference orbit for perturbation theory).
   *
   * @param context - Rendering context with viewport parameters
   */
  prepareForRender?(context: AlgorithmContext): void;

  /**
   * Computes the iteration count and final z values for a point in the complex plane.
   *
   * @param real - Real component of the complex number (x-coordinate)
   * @param imag - Imaginary component of the complex number (y-coordinate)
   * @param maxIterations - Maximum number of iterations to compute
   * @returns IterationResult containing iteration count and final z values
   */
  computePoint(real: number, imag: number, maxIterations: number): IterationResult;

  /**
   * Optional precision-preserving method for algorithms that suffer from catastrophic cancellation.
   * This method calculates deltaC directly from pixel offsets, avoiding precision loss in the
   * pixel → world coordinate conversion pipeline.
   *
   * @param offsetX - Pixel offset from canvas center (integer)
   * @param offsetY - Pixel offset from canvas center (integer)
   * @param scale - Fractal units per pixel (calculated from zoom level)
   * @param maxIterations - Maximum number of iterations to compute
   * @returns IterationResult containing iteration count and final z values
   */
  computePointFromOffset?(offsetX: number, offsetY: number, scale: number, maxIterations: number): IterationResult;
}
