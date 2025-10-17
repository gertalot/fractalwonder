import * as Comlink from "comlink";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { FractalWorkerAPI } from "./fractal.worker";
import { ChunkComputeRequest } from "./types";

// Skip all tests in this file if Worker is not available (e.g., in jsdom/Node environment)
// These tests should be run in a browser environment (e.g., with Playwright)
const describeIfWorkerAvailable =
  typeof Worker !== "undefined" ? describe : describe.skip;

describeIfWorkerAvailable("FractalWorker (requires browser environment)", () => {
  let worker: Worker;
  let workerAPI: Comlink.Remote<FractalWorkerAPI>;

  beforeEach(() => {
    // Import the worker using Vite's ?worker suffix
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

  describe("connectivity", () => {
    it("should respond to ping", async () => {
      const response = await workerAPI.ping();
      expect(response).toBe("pong");
    });
  });

  describe("computeChunk", () => {
    it("should compute a 10x10 chunk correctly", async () => {
      const request: ChunkComputeRequest = {
        chunk: { startX: 0, startY: 0, width: 10, height: 10 },
        params: {
          center: { x: -0.5, y: 0 },
          zoom: 1,
          maxIterations: 100,
          iterationScalingFactor: 100,
        },
        canvasWidth: 100,
        canvasHeight: 100,
        algorithmName: "Mandelbrot Set",
      };

      const result = await workerAPI.computeChunk(request);

      expect(result).toBeDefined();
      expect(result.chunk).toEqual(request.chunk);
      expect(result.imageData).toBeDefined();
      expect(result.imageData.width).toBe(10);
      expect(result.imageData.height).toBe(10);
      expect(result.imageData.data.length).toBe(10 * 10 * 4); // RGBA
    });

    it("should return ImageData with correct dimensions for rectangular chunk", async () => {
      const request: ChunkComputeRequest = {
        chunk: { startX: 50, startY: 50, width: 20, height: 15 },
        params: {
          center: { x: -1, y: 0 },
          zoom: 1,
          maxIterations: 100,
          iterationScalingFactor: 100,
        },
        canvasWidth: 200,
        canvasHeight: 200,
        algorithmName: "Mandelbrot Set",
      };

      const result = await workerAPI.computeChunk(request);

      expect(result.imageData.width).toBe(20);
      expect(result.imageData.height).toBe(15);
      expect(result.imageData.data.length).toBe(20 * 15 * 4);
    });

    it("should compute pixels with valid RGBA values", async () => {
      const request: ChunkComputeRequest = {
        chunk: { startX: 0, startY: 0, width: 5, height: 5 },
        params: {
          center: { x: 0, y: 0 },
          zoom: 1,
          maxIterations: 50,
          iterationScalingFactor: 50,
        },
        canvasWidth: 50,
        canvasHeight: 50,
        algorithmName: "Mandelbrot Set",
      };

      const result = await workerAPI.computeChunk(request);
      const data = result.imageData.data;

      // Check every pixel has valid RGBA values (0-255)
      for (let i = 0; i < data.length; i += 4) {
        const r = data[i];
        const g = data[i + 1];
        const b = data[i + 2];
        const a = data[i + 3];

        expect(r).toBeGreaterThanOrEqual(0);
        expect(r).toBeLessThanOrEqual(255);
        expect(g).toBeGreaterThanOrEqual(0);
        expect(g).toBeLessThanOrEqual(255);
        expect(b).toBeGreaterThanOrEqual(0);
        expect(b).toBeLessThanOrEqual(255);
        expect(a).toBe(255); // Alpha should always be 255 (opaque)
      }
    });

    it("should handle multiple sequential requests correctly", async () => {
      const baseRequest: ChunkComputeRequest = {
        chunk: { startX: 0, startY: 0, width: 5, height: 5 },
        params: {
          center: { x: -1, y: 0 },
          zoom: 1,
          maxIterations: 50,
          iterationScalingFactor: 50,
        },
        canvasWidth: 50,
        canvasHeight: 50,
        algorithmName: "Mandelbrot Set",
      };

      // Send three requests sequentially
      const result1 = await workerAPI.computeChunk(baseRequest);
      const result2 = await workerAPI.computeChunk({
        ...baseRequest,
        chunk: { startX: 5, startY: 5, width: 5, height: 5 },
      });
      const result3 = await workerAPI.computeChunk({
        ...baseRequest,
        chunk: { startX: 10, startY: 10, width: 5, height: 5 },
      });

      expect(result1.imageData.width).toBe(5);
      expect(result2.imageData.width).toBe(5);
      expect(result3.imageData.width).toBe(5);

      expect(result1.chunk.startX).toBe(0);
      expect(result2.chunk.startX).toBe(5);
      expect(result3.chunk.startX).toBe(10);
    });
  });
});

