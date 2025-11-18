import { FractalParams } from "@/hooks/use-store";
import { ParallelRenderer, RenderOptions } from "./parallel-renderer";

type FractalAlgorithm = (canvas: HTMLCanvasElement, params: FractalParams) => void;

/**
 * Render mode types
 */
export type RenderMode = "parallel" | "single-threaded";

/**
 * Detects if the browser supports Web Workers.
 * @returns true if workers are supported, false otherwise
 */
export function detectWorkerSupport(): boolean {
  try {
    return typeof Worker !== "undefined";
  } catch {
    return false;
  }
}

/**
 * Renders a fractal onto a canvas using a provided algorithm (single-threaded).
 *
 * @param canvas - The HTMLCanvasElement to draw on.
 * @param params - The fractal parameters (center, zoom, etc.).
 * @param fractalAlgorithm - The function that performs the actual fractal calculation and drawing (e.g., mandelbrot).
 * @returns The ImageData of the rendered fractal, or null if rendering fails.
 */
export const renderFractal = (canvas: HTMLCanvasElement, params: FractalParams, fractalAlgorithm: FractalAlgorithm) => {
  if (canvas.width === 0 || canvas.height === 0) {
    console.warn("Skipping render: Canvas dimensions are zero.");
    return null;
  }
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    console.error("Failed to get 2D context from canvas.");
    return null;
  }

  // Execute the specific fractal drawing logic
  fractalAlgorithm(canvas, params);
};

/**
 * Renders a fractal onto a canvas using parallel computation with Web Workers.
 *
 * @param renderer - The ParallelRenderer instance to use
 * @param canvas - The HTMLCanvasElement to draw on
 * @param params - The fractal parameters (center, zoom, etc.)
 * @param algorithmName - The name of the algorithm to use (e.g., "Mandelbrot Set")
 * @param options - Render options (progress callback, abort signal)
 * @returns Promise that resolves when rendering is complete
 */
export async function renderFractalParallel(
  renderer: ParallelRenderer,
  canvas: HTMLCanvasElement,
  params: FractalParams,
  algorithmName: string,
  options?: RenderOptions
): Promise<void> {
  await renderer.render(canvas, params, algorithmName, options);
}
