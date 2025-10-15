// ABOUTME: Core chunk computation logic for Web Workers
// ABOUTME: Converts a canvas chunk into computed fractal pixel data

import type { FractalAlgorithm } from "@/fractals/algorithms/base";
import { smoothLoopingColorScheme } from "@/fractals/algorithms/coloring";
import { mandelbrotAlgorithm } from "@/fractals/algorithms/mandelbrot";
import { perturbationMandelbrotAlgorithm } from "@/fractals/algorithms/perturbation-mandelbrot";
import { derivedRealIterations } from "@/hooks/use-store";
import { pixelToFractalCoordinateUltraHP } from "@/lib/coordinates";
import { Decimal } from "decimal.js";

import { ChunkComputeRequest, ChunkComputeResult, deserializeFractalParams } from "./types";

/**
 * Computes fractal pixel data for a specific rectangular chunk of the canvas.
 *
 * This function runs in a Web Worker and performs the following:
 * 1. Creates a pixel buffer for the chunk
 * 2. For each pixel in the chunk:
 *    - Converts canvas coordinates to fractal coordinates
 *    - Computes iteration count using the selected algorithm
 *    - Applies color scheme to get RGB values
 *    - Writes RGBA values to the buffer
 * 3. Returns ImageData ready to be drawn on the main thread canvas
 *
 * @param request - Chunk computation parameters
 * @returns Result containing chunk bounds and computed ImageData
 */
export function computeChunk(request: ChunkComputeRequest): ChunkComputeResult {
  const { chunk, params: serializedParams, canvasWidth, canvasHeight, algorithmName } = request;
  const { startX, startY, width, height } = chunk;
  
  // Deserialize params to restore Decimal objects for high-precision computation
  const params = deserializeFractalParams(serializedParams);

  // Select algorithm based on algorithmName
  let algorithm: FractalAlgorithm;
  if (algorithmName === "Perturbation Mandelbrot") {
    algorithm = perturbationMandelbrotAlgorithm;
  } else {
    algorithm = mandelbrotAlgorithm;
  }

  // Create buffer for pixel data (4 bytes per pixel: RGBA)
  const buffer = new Uint8ClampedArray(width * height * 4);
  const maxIter = derivedRealIterations(params);

  // Prepare algorithm if it supports prepareForRender (e.g., perturbation theory)
  if (algorithm.prepareForRender) {
    algorithm.prepareForRender({
      center: params.center,
      zoom: params.zoom,
      maxIterations: maxIter,
    });
  }

  // Compute each pixel in the chunk
  for (let x = 0; x < width; x++) {
    for (let y = 0; y < height; y++) {
      let iter: number;

      // Use precision-preserving method if available (for perturbation algorithm)
      if (algorithm.computePointFromOffset) {
        // Calculate pixel offset from canvas center
        const pixelX = startX + x;
        const pixelY = startY + y;
        const offsetX = pixelX - canvasWidth / 2;
        const offsetY = pixelY - canvasHeight / 2;
        
        // Calculate scale factor (fractal units per pixel) using Decimal arithmetic
        const scale = new Decimal(4).div(canvasHeight).div(params.zoom).toNumber(); // Same as pixelToFractalCoordinate
        
        // Use precision-preserving method
        const result = algorithm.computePointFromOffset(offsetX, offsetY, scale, maxIter);
        iter = result.iter;
      } else {
        // Fallback to precision-preserving method for standard algorithm
        const { x: realDecimal, y: imagDecimal } = pixelToFractalCoordinateUltraHP(
          { x: startX + x, y: startY + y },
          canvasWidth,
          canvasHeight,
          params.center,
          params.zoom
        );

        const result = algorithm.computePoint(realDecimal.toNumber(), imagDecimal.toNumber(), maxIter);
        iter = result.iter;
      }

      // Apply color scheme
      const [r, g, b] = smoothLoopingColorScheme(iter, maxIter);

      // Write RGBA values to buffer
      const index = (y * width + x) * 4;
      buffer[index] = r;
      buffer[index + 1] = g;
      buffer[index + 2] = b;
      buffer[index + 3] = 255; // Alpha: fully opaque
    }
  }

  // Create ImageData from buffer
  const imageData = new ImageData(buffer, width, height);

  return {
    chunk,
    imageData,
  };
}

