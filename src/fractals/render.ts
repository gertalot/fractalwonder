import { FractalParams } from "@/hooks/use-store";

type FractalAlgorithm = (canvas: HTMLCanvasElement, params: FractalParams) => void;

/**
 * Renders a fractal onto a canvas using a provided algorithm.
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

  console.log("Rendering fractal... center, zoom: ", params.center, params.zoom);

  // Execute the specific fractal drawing logic
  fractalAlgorithm(canvas, params);

  console.log("External fractal rendering done.");
};
