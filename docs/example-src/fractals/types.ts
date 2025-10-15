import { Decimal } from "decimal.js";
import React from "react";

export type Point = {
  x: Decimal;
  y: Decimal;
};

// --- Parameter Types ---
// Base type - might include common view parameters later if needed
export type BaseFractalParams = {
  /* Common fields if any */
};

// Specific parameter types using a discriminated union
export type MandelbrotParams = BaseFractalParams & {
  type: "mandelbrot";
  center: Point;
  zoom: Decimal;
  maxIterations: number;
  iterationScalingFactor: number;
};

export type JuliaParams = BaseFractalParams & {
  type: "julia";
  center: Point;
  zoom: Decimal;
  maxIterations: number;
  iterationScalingFactor: number;
  cReal: number;
  cImag: number;
};

// Add more parameter types here as needed
// export type LSystemParams = ...

// Union of all possible parameter sets
export type FractalParams = MandelbrotParams | JuliaParams /* | LSystemParams | ... */;

// --- Raw Data Types (Flexible) ---
// Example for iteration-based fractals like Mandelbrot/Julia
// Stores [iteration, final z_real, final z_imag] per pixel
export type IterationData = {
  width: number;
  height: number;
  values: Float32Array; // Size = width * height * 3
};

// You might define other raw data types later
// export type VectorFieldData = { dataType: 'vectorField', ... };

// Union of possible raw data structures produced by calculations
export type RawFractalData = IterationData /* | VectorFieldData | ... */;

// --- Color Scheme Function ---
// A function that maps raw data points to RGBA colors
export type ColorSchemeFunction = (
  // Provides data for a single point/pixel
  dataPoint: { iter: number; zr: number; zi: number; maxIter: number }
  // Optionally provide the full raw dataset for context (e.g., normalization)
  // fullRawData?: RawFractalData
) => [r: number, g: number, b: number, a: number]; // 0-255

// --- Rendering Progress ---
export type RenderProgress = {
  percentComplete: number;
  // Could potentially include intermediate ImageData for previews
  // previewImageData?: ImageData;
};

// --- The Main Fractal Interface ---
// Generic over the specific parameter type P
export interface FractalInterface<P extends FractalParams = FractalParams> {
  // === Metadata ===
  name: string; // Display name (e.g., "Mandelbrot Set")
  type: P["type"]; // Unique identifier (e.g., "mandelbrot")
  description?: string;

  // === Parameters ===
  defaultParameters: P; // Default values for this fractal type

  // Optional: A React component to render controls for this fractal's specific parameters.
  // The main UI can dynamically render this component when this fractal is selected.
  ParameterUI?: React.FC<{
    params: P;
    onChange: (newPartialParams: Partial<P>) => void;
  }>;

  // === Rendering Methods & Capabilities ===

  // --- Method 1: Chunked Calculation (for parallelizable CPU/Worker tasks) ---
  // Calculates raw data for a specific region (chunk) of the canvas.
  // If this method exists, the renderer can assume chunking/parallelization is possible.
  calculateChunk?: (
    params: P,
    // The specific rectangle (in canvas pixels) this chunk should calculate
    chunk: { x: number; y: number; width: number; height: number },
    signal?: AbortSignal // For cancellation
  ) => Promise<RawFractalData> | RawFractalData; // Return raw data for the chunk

  // --- Method 2: Applying Post-Processing / Coloring ---
  // Takes assembled raw data (from chunks) and applies final processing (like coloring)
  // to draw onto the canvas. Required if `calculateChunk` is used.
  applyPostProcessing?: (
    ctx: CanvasRenderingContext2D,
    params: P, // Needed for context (e.g., maxIterations for color scaling)
    rawData: RawFractalData, // Assembled raw data for the *entire* view
    // Pass the currently selected color scheme function
    colorScheme: ColorSchemeFunction
  ) => void;

  // --- Method 3: Direct Rendering (for non-chunkable or simple fractals) ---
  // Renders directly to the canvas context. Used if `calculateChunk` is not suitable.
  // Could be sync, async (to allow yielding), or return a generator for progress.
  renderDirectly?: (
    ctx: CanvasRenderingContext2D,
    params: P,
    signal?: AbortSignal
    // Could return a Promise for async yielding, or an AsyncGenerator for progress updates
  ) => Promise<void> | void | AsyncGenerator<RenderProgress, void, void>;

  // --- Method 4: GPU Rendering (Placeholder) ---
  // renderGPU?: (gl: WebGLRenderingContext | WebGL2RenderingContext, ...) => void;
}
