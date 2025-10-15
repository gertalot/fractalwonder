// ABOUTME: Type definitions for Web Worker communication
// ABOUTME: Defines request/response interfaces for chunk-based fractal computation

import { FractalParams } from "@/hooks/use-store";

/**
 * Serializable version of Point where Decimal objects are converted to strings
 * for safe transmission via postMessage (structured clone algorithm).
 */
export interface SerializablePoint {
  x: string;
  y: string;
}

/**
 * Serializable version of FractalParams where Decimal objects are converted to strings
 * for safe transmission via postMessage (structured clone algorithm).
 */
export interface SerializableFractalParams {
  center: SerializablePoint;
  zoom: string;
  maxIterations: number;
  iterationScalingFactor: number;
}

/**
 * Rectangle defining a chunk of the canvas to compute.
 */
export interface ChunkBounds {
  /** X coordinate of chunk's top-left corner (in canvas pixels) */
  startX: number;
  /** Y coordinate of chunk's top-left corner (in canvas pixels) */
  startY: number;
  /** Width of chunk in pixels */
  width: number;
  /** Height of chunk in pixels */
  height: number;
}

/**
 * Request sent to worker to compute a fractal chunk.
 * Uses SerializableFractalParams to avoid DataCloneError with Decimal objects.
 */
export interface ChunkComputeRequest {
  /** Bounds of the chunk to compute */
  chunk: ChunkBounds;
  /** Fractal parameters (center, zoom, iterations, etc.) - serialized for worker */
  params: SerializableFractalParams;
  /** Total canvas width (needed for coordinate transformations) */
  canvasWidth: number;
  /** Total canvas height (needed for coordinate transformations) */
  canvasHeight: number;
  /** Name of the algorithm to use (e.g., "Mandelbrot Set") */
  algorithmName: string;
  /**
   * Render ID for abort checking. If this changes mid-computation,
   * the worker should abort by throwing an error.
   */
  renderId?: string;
}

/**
 * Result returned from worker after computing a chunk.
 */
export interface ChunkComputeResult {
  /** Bounds of the computed chunk (matches request) */
  chunk: ChunkBounds;
  /** Computed pixel data ready to be drawn on canvas */
  imageData: ImageData;
}

/**
 * Serializes FractalParams by converting Decimal objects to strings.
 * This enables safe transmission via postMessage (structured clone algorithm).
 */
export function serializeFractalParams(params: FractalParams): SerializableFractalParams {
  return {
    center: {
      x: params.center.x.toString(),
      y: params.center.y.toString(),
    },
    zoom: params.zoom.toString(),
    maxIterations: params.maxIterations,
    iterationScalingFactor: params.iterationScalingFactor,
  };
}

/**
 * Deserializes SerializableFractalParams by converting strings back to Decimal objects.
 * This restores the original high-precision values in the worker environment.
 */
export function deserializeFractalParams(serialized: SerializableFractalParams): FractalParams {
  // Import Decimal dynamically since workers don't have require
  const Decimal = (globalThis as any).Decimal || (self as any).Decimal;
  if (!Decimal) {
    throw new Error("Decimal.js not available in worker environment");
  }
  
  return {
    center: {
      x: new Decimal(serialized.center.x),
      y: new Decimal(serialized.center.y),
    },
    zoom: new Decimal(serialized.zoom),
    maxIterations: serialized.maxIterations,
    iterationScalingFactor: serialized.iterationScalingFactor,
  };
}

