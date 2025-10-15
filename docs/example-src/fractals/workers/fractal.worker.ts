// ABOUTME: Web Worker for parallel fractal computation using Comlink RPC
// ABOUTME: Exposes computeChunk function for main thread to call via Comlink

import * as Comlink from "comlink";
import { Decimal } from "decimal.js";

import { computeChunk } from "./compute-chunk";
import { ChunkComputeRequest } from "./types";

// Make Decimal available globally for deserialization functions
(globalThis as any).Decimal = Decimal;

/**
 * Worker API exposed to the main thread via Comlink.
 * All methods can be called as if they were async functions on the main thread.
 */
const workerAPI = {
  /**
   * Computes a fractal chunk and returns the pixel data.
   * @param request - Chunk computation parameters
   * @returns Promise resolving to computed ImageData
   */
  computeChunk: (request: ChunkComputeRequest) => {
    return computeChunk(request);
  },

  /**
   * Simple ping method for testing worker connectivity.
   * @returns "pong" string
   */
  ping: () => "pong" as const,
};

// Expose the API to the main thread via Comlink
Comlink.expose(workerAPI);

// Export the type for use on the main thread
export type FractalWorkerAPI = typeof workerAPI;

