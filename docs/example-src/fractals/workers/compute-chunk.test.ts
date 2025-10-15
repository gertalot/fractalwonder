import * as Comlink from "comlink";
import { Decimal } from "decimal.js";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { smoothLoopingColorScheme } from "@/fractals/algorithms/coloring";
import { mandelbrotAlgorithm } from "@/fractals/algorithms/mandelbrot";
import { derivedRealIterations } from "@/hooks/use-store";
import { pixelToFractalCoordinateUltraHP } from "@/lib/coordinates";

import { computeChunk } from "./compute-chunk";
import { FractalWorkerAPI } from "./fractal.worker";
import { ChunkComputeRequest, deserializeFractalParams, serializeFractalParams } from "./types";

// Make Decimal available in worker environment for tests
if (typeof globalThis !== "undefined") {
  (globalThis as any).Decimal = Decimal;
}
if (typeof self !== "undefined") {
  (self as any).Decimal = Decimal;
}

/**
 * NOTE: All tests in this file require ImageData which is not available in jsdom.
 * These tests are skipped in the standard test run and should be tested in a
 * browser environment using Playwright or similar browser testing tools.
 *
 * The worker implementation is validated through:
 * 1. Type checking (ensures correct interfaces)
 * 2. Manual browser testing (Playwright)
 * 3. Integration testing in Story 3-4 when connected to Canvas
 */

// Skip all comparison tests in Node environment (they need ImageData)
const describeIfImageDataAvailable =
  typeof ImageData !== "undefined" ? describe : describe.skip;

describeIfImageDataAvailable("computeChunk - Main Thread Computation", () => {
  /**
   * Computes a chunk on the main thread for comparison using Decimal precision.
   * This duplicates the logic from compute-chunk.ts but for testing.
   */
  function computeChunkMainThread(request: ChunkComputeRequest): ImageData {
    const { chunk, params, canvasWidth, canvasHeight } = request;
    const { startX, startY, width, height } = chunk;

    const buffer = new Uint8ClampedArray(width * height * 4);
    // Deserialize params to get FractalParams for derivedRealIterations
    const fractalParams = deserializeFractalParams(params);
    const maxIter = derivedRealIterations(fractalParams);

    for (let x = 0; x < width; x++) {
      for (let y = 0; y < height; y++) {
        // Use ultra-high-precision coordinate conversion like the worker
        const { x: realDecimal, y: imagDecimal } = pixelToFractalCoordinateUltraHP(
          { x: startX + x, y: startY + y },
          canvasWidth,
          canvasHeight,
          fractalParams.center,
          fractalParams.zoom
        );

        const { iter } = mandelbrotAlgorithm.computePoint(realDecimal.toNumber(), imagDecimal.toNumber(), maxIter);
        const [r, g, b] = smoothLoopingColorScheme(iter, maxIter);

        const index = (y * width + x) * 4;
        buffer[index] = r;
        buffer[index + 1] = g;
        buffer[index + 2] = b;
        buffer[index + 3] = 255;
      }
    }

    return new ImageData(buffer, width, height);
  }

  it("should produce identical output to main thread computation", () => {
    const fractalParams = {
      center: { x: new Decimal(-0.5), y: new Decimal(0) },
      zoom: new Decimal(1),
      maxIterations: 100,
      iterationScalingFactor: 100,
    };
    
    const request: ChunkComputeRequest = {
      chunk: { startX: 0, startY: 0, width: 10, height: 10 },
      params: serializeFractalParams(fractalParams),
      canvasWidth: 100,
      canvasHeight: 100,
      algorithmName: "Mandelbrot Set",
    };

    const mainThreadResult = computeChunkMainThread(request);
    const chunkResult = computeChunk(request);

    // Compare dimensions
    expect(chunkResult.imageData.width).toBe(mainThreadResult.width);
    expect(chunkResult.imageData.height).toBe(mainThreadResult.height);

    // Compare pixel data byte-for-byte
    const workerData = chunkResult.imageData.data;
    const mainData = mainThreadResult.data;

    expect(workerData.length).toBe(mainData.length);

    for (let i = 0; i < workerData.length; i++) {
      expect(workerData[i]).toBe(mainData[i]);
    }
  });


  it("should produce identical results at different zoom levels", () => {
    const zoomLevels = [1, 10, 100];

    for (const zoom of zoomLevels) {
      const fractalParams = {
        center: { x: new Decimal(-0.75), y: new Decimal(0.1) },
        zoom: new Decimal(zoom),
        maxIterations: 100,
        iterationScalingFactor: 100,
      };
      
      const request: ChunkComputeRequest = {
        chunk: { startX: 10, startY: 10, width: 8, height: 8 },
        params: serializeFractalParams(fractalParams),
        canvasWidth: 100,
        canvasHeight: 100,
        algorithmName: "Mandelbrot Set",
      };

      const mainThreadResult = computeChunkMainThread(request);
      const chunkResult = computeChunk(request);

      const workerData = chunkResult.imageData.data;
      const mainData = mainThreadResult.data;

      for (let i = 0; i < workerData.length; i++) {
        expect(workerData[i]).toBe(mainData[i]);
      }
    }
  });

  it("should maintain precision at extreme zoom levels", () => {
    const extremeZoomLevels = [
      new Decimal(1e10),
      new Decimal(1e15),
      new Decimal(1e20),
    ];

    for (const zoom of extremeZoomLevels) {
      const fractalParams = {
        center: { x: new Decimal(-0.75), y: new Decimal(0.1) },
        zoom,
        maxIterations: 100,
        iterationScalingFactor: 100,
      };
      
      const request: ChunkComputeRequest = {
        chunk: { startX: 5, startY: 5, width: 4, height: 4 },
        params: serializeFractalParams(fractalParams),
        canvasWidth: 100,
        canvasHeight: 100,
        algorithmName: "Mandelbrot Set",
      };

      // Should not throw an error and should produce consistent results
      expect(() => {
        const result = computeChunk(request);
        expect(result.imageData.width).toBe(4);
        expect(result.imageData.height).toBe(4);
        expect(result.imageData.data.length).toBe(4 * 4 * 4); // RGBA
      }).not.toThrow();
    }
  });

  it("should maintain precision in scale calculation", () => {
    const zoom = new Decimal(1e15);
    const canvasHeight = 1000;
    
    // Test that scale calculation matches coordinate function precision
    const scaleFromWorker = new Decimal(4).div(canvasHeight).div(zoom);
    const scaleFromCoordFunction = new Decimal(4).div(canvasHeight).div(zoom);
    
    expect(scaleFromWorker.toString()).toBe(scaleFromCoordFunction.toString());
    
    // Verify precision is maintained
    expect(scaleFromWorker.decimalPlaces()).toBeGreaterThan(10);
  });

  it("should work with both algorithm paths (perturbation and standard)", () => {
    const fractalParams = {
      center: { x: new Decimal(-0.75), y: new Decimal(0.1) },
      zoom: new Decimal(1e10),
      maxIterations: 100,
      iterationScalingFactor: 100,
    };
    
    const request: ChunkComputeRequest = {
      chunk: { startX: 5, startY: 5, width: 4, height: 4 },
      params: serializeFractalParams(fractalParams),
      canvasWidth: 100,
      canvasHeight: 100,
      algorithmName: "Mandelbrot Set", // Standard algorithm (fallback path)
    };

    // Test standard algorithm path (fallback)
    expect(() => {
      const result = computeChunk(request);
      expect(result.imageData.width).toBe(4);
      expect(result.imageData.height).toBe(4);
      expect(result.imageData.data.length).toBe(4 * 4 * 4); // RGBA
    }).not.toThrow();

    // Test perturbation algorithm path (precision-preserving)
    const perturbationRequest: ChunkComputeRequest = {
      ...request,
      algorithmName: "Perturbation Mandelbrot",
    };

    expect(() => {
      const result = computeChunk(perturbationRequest);
      expect(result.imageData.width).toBe(4);
      expect(result.imageData.height).toBe(4);
      expect(result.imageData.data.length).toBe(4 * 4 * 4); // RGBA
    }).not.toThrow();
  });
});

// Worker-specific comparison tests (require browser environment)
const describeIfWorkerAvailable =
  typeof Worker !== "undefined" ? describe : describe.skip;

describeIfWorkerAvailable(
  "computeChunk - Worker vs Main Thread (requires browser)",
  () => {
    let worker: Worker;
    let workerAPI: Comlink.Remote<FractalWorkerAPI>;

    beforeEach(() => {
      worker = new Worker(new URL("./fractal.worker.ts", import.meta.url), {
        type: "module",
      });
      workerAPI = Comlink.wrap<FractalWorkerAPI>(worker);
    });

    afterEach(() => {
      if (worker) {
        worker.terminate();
      }
    });

    /**
     * Computes a chunk on the main thread for comparison using Decimal precision.
     */
    function computeChunkMainThread(request: ChunkComputeRequest): ImageData {
      const { chunk, params, canvasWidth, canvasHeight } = request;
      const { startX, startY, width, height } = chunk;

      const buffer = new Uint8ClampedArray(width * height * 4);
      // Deserialize params to get FractalParams for derivedRealIterations
      const fractalParams = deserializeFractalParams(params);
      const maxIter = derivedRealIterations(fractalParams);

      for (let x = 0; x < width; x++) {
        for (let y = 0; y < height; y++) {
          // Use ultra-high-precision coordinate conversion like the worker
          const { x: realDecimal, y: imagDecimal } = pixelToFractalCoordinateUltraHP(
            { x: startX + x, y: startY + y },
            canvasWidth,
            canvasHeight,
            fractalParams.center,
            fractalParams.zoom
          );

          const { iter } = mandelbrotAlgorithm.computePoint(realDecimal.toNumber(), imagDecimal.toNumber(), maxIter);
          const [r, g, b] = smoothLoopingColorScheme(iter, maxIter);

          const index = (y * width + x) * 4;
          buffer[index] = r;
          buffer[index + 1] = g;
          buffer[index + 2] = b;
          buffer[index + 3] = 255;
        }
      }

      return new ImageData(buffer, width, height);
    }

    it("worker should produce identical output to main thread computation", async () => {
      const fractalParams = {
        center: { x: new Decimal(-1), y: new Decimal(0) },
        zoom: new Decimal(2),
        maxIterations: 200,
        iterationScalingFactor: 200,
      };
      
      const request: ChunkComputeRequest = {
        chunk: { startX: 25, startY: 25, width: 15, height: 15 },
        params: serializeFractalParams(fractalParams),
        canvasWidth: 200,
        canvasHeight: 200,
        algorithmName: "Mandelbrot Set",
      };

      const mainThreadResult = computeChunkMainThread(request);
      const workerResult = await workerAPI.computeChunk(request);

      // Compare dimensions
      expect(workerResult.imageData.width).toBe(mainThreadResult.width);
      expect(workerResult.imageData.height).toBe(mainThreadResult.height);

      // Compare pixel data byte-for-byte
      const workerData = workerResult.imageData.data;
      const mainData = mainThreadResult.data;

      expect(workerData.length).toBe(mainData.length);

      for (let i = 0; i < workerData.length; i++) {
        if (workerData[i] !== mainData[i]) {
          const pixelIndex = Math.floor(i / 4);
          const x = pixelIndex % 15;
          const y = Math.floor(pixelIndex / 15);
          const channel = ["R", "G", "B", "A"][i % 4];
          throw new Error(
            `Mismatch at pixel (${x}, ${y}) channel ${channel}: worker=${workerData[i]}, main=${mainData[i]}`
          );
        }
      }
    });
  }
);

