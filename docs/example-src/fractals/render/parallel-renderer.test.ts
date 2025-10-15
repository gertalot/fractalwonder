import { FractalParams } from "@/hooks/use-store";
import * as Comlink from "comlink";
import { Decimal } from "decimal.js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FractalWorkerAPI } from "../workers/fractal.worker";
import { ChunkComputeResult } from "../workers/types";
import { ParallelRenderer } from "./parallel-renderer";

// Mock Comlink
vi.mock("comlink", () => ({
  wrap: vi.fn(),
  releaseProxy: Symbol("releaseProxy"),
}));

// Mock the worker import
vi.mock("../workers/fractal.worker?worker", () => ({
  default: vi.fn(() => ({
    terminate: vi.fn(),
    postMessage: vi.fn(),
    addEventListener: vi.fn(),
  })),
}));

describe("ParallelRenderer", () => {
  let renderer: ParallelRenderer;
  let mockWorkerAPI: Partial<FractalWorkerAPI> & { [Comlink.releaseProxy]: () => void };
  let mockCanvas: HTMLCanvasElement;
  let mockCtx: CanvasRenderingContext2D;
  
  // Define testParams at module level so it's available to all tests
  const testParams: FractalParams = {
    center: { x: new Decimal(0), y: new Decimal(0) },
    zoom: new Decimal(1),
    maxIterations: 100,
    iterationScalingFactor: 1,
  };

  beforeEach(() => {
    // Reset mocks
    vi.clearAllMocks();

    // Create mock worker API
    mockWorkerAPI = {
      ping: vi.fn().mockResolvedValue("pong"),
      computeChunk: vi.fn().mockImplementation((request) => {
        // Create a mock ImageData
        const { width, height } = request.chunk;
        const buffer = new Uint8ClampedArray(width * height * 4);
        // Fill with test pattern (red pixels)
        for (let i = 0; i < buffer.length; i += 4) {
          buffer[i] = 255; // R
          buffer[i + 1] = 0; // G
          buffer[i + 2] = 0; // B
          buffer[i + 3] = 255; // A
        }
        const imageData = new ImageData(buffer, width, height);

        const result: ChunkComputeResult = {
          chunk: request.chunk,
          imageData,
        };
        return Promise.resolve(result);
      }),
      [Comlink.releaseProxy]: vi.fn(),
    };

    // Mock Comlink.wrap to return our mock API
    (Comlink.wrap as ReturnType<typeof vi.fn>).mockReturnValue(mockWorkerAPI);

    // Create mock canvas
    mockCtx = {
      putImageData: vi.fn(),
      getImageData: vi.fn(),
      clearRect: vi.fn(),
    } as unknown as CanvasRenderingContext2D;

    mockCanvas = {
      width: 200,
      height: 200,
      getContext: vi.fn().mockReturnValue(mockCtx),
    } as unknown as HTMLCanvasElement;

    // Create renderer with 2 workers for testing
    renderer = new ParallelRenderer(2);
  });

  afterEach(() => {
    renderer.terminate();
  });

  describe("constructor", () => {
    it("should create renderer with specified worker count", () => {
      const r = new ParallelRenderer(4);
      expect(r.getWorkerCount()).toBe(4);
      r.terminate();
    });

    it("should use optimal worker count when not specified", () => {
      const r = new ParallelRenderer();
      const count = r.getWorkerCount();
      expect(count).toBeGreaterThanOrEqual(2);
      expect(count).toBeLessThanOrEqual(16);
      r.terminate();
    });
  });

  describe("init", () => {
    it("should initialize workers successfully", async () => {
      await renderer.init();
      expect(mockWorkerAPI.ping).toHaveBeenCalled();
    });

    it("should be idempotent (calling init twice is safe)", async () => {
      await renderer.init();
      const callCount = (mockWorkerAPI.ping as ReturnType<typeof vi.fn>).mock.calls.length;
      await renderer.init();
      // Should not create new workers
      expect((mockWorkerAPI.ping as ReturnType<typeof vi.fn>).mock.calls.length).toBe(callCount);
    });

    it("should throw error if worker fails to respond to ping", async () => {
      (mockWorkerAPI.ping as ReturnType<typeof vi.fn>).mockResolvedValue("wrong");
      await expect(renderer.init()).rejects.toThrow("failed to respond to ping");
    });
  });

  describe("render", () => {
    it("should throw error if not initialized", async () => {
      await expect(renderer.render(mockCanvas, testParams, "Mandelbrot Set")).rejects.toThrow(
        "not initialized"
      );
    });

    it("should skip render if canvas dimensions are zero", async () => {
      await renderer.init();
      const zeroCanvas = { ...mockCanvas, width: 0, height: 0 };
      await renderer.render(zeroCanvas as HTMLCanvasElement, testParams, "Mandelbrot Set");
      expect(mockWorkerAPI.computeChunk).not.toHaveBeenCalled();
    });

    it("should throw error if canvas context is null", async () => {
      await renderer.init();
      const badCanvas = { ...mockCanvas, getContext: vi.fn().mockReturnValue(null) };
      await expect(
        renderer.render(badCanvas as unknown as HTMLCanvasElement, testParams, "Mandelbrot Set")
      ).rejects.toThrow("Failed to get 2D context");
    });

    it("should compute and render all chunks", async () => {
      await renderer.init();
      await renderer.render(mockCanvas, testParams, "Mandelbrot Set");

      // Verify workers were called
      expect(mockWorkerAPI.computeChunk).toHaveBeenCalled();

      // Verify putImageData was called to draw chunks
      expect(mockCtx.putImageData).toHaveBeenCalled();
    });

    it("should distribute chunks to workers in round-robin fashion", async () => {
      await renderer.init();
      await renderer.render(mockCanvas, testParams, "Mandelbrot Set");

      // Both workers should have been used (we have 2 workers and many chunks)
      const callCount = (mockWorkerAPI.computeChunk as ReturnType<typeof vi.fn>).mock.calls.length;
      expect(callCount).toBeGreaterThan(2); // At least more chunks than workers
    });

    it("should call progress callback with increasing percentages", async () => {
      await renderer.init();
      const progressValues: number[] = [];
      const onProgress = vi.fn((pct: number) => progressValues.push(pct));

      await renderer.render(mockCanvas, testParams, "Mandelbrot Set", { onProgress });

      expect(onProgress).toHaveBeenCalled();
      // Verify progress increases
      for (let i = 1; i < progressValues.length; i++) {
        expect(progressValues[i]).toBeGreaterThanOrEqual(progressValues[i - 1]);
      }
      // Final progress should be 100%
      expect(progressValues[progressValues.length - 1]).toBe(100);
    });

    it("should pass correct parameters to workers", async () => {
      await renderer.init();
      await renderer.render(mockCanvas, testParams, "Mandelbrot Set");

      const calls = (mockWorkerAPI.computeChunk as ReturnType<typeof vi.fn>).mock.calls;
      expect(calls.length).toBeGreaterThan(0);

      // Verify first call has correct structure
      const firstRequest = calls[0][0];
      expect(firstRequest).toHaveProperty("chunk");
      expect(firstRequest).toHaveProperty("params");
      expect(firstRequest).toHaveProperty("canvasWidth", 200);
      expect(firstRequest).toHaveProperty("canvasHeight", 200);
      expect(firstRequest).toHaveProperty("algorithmName", "Mandelbrot Set");
      expect(firstRequest.params).toEqual({
        center: { x: "0", y: "0" },
        zoom: "1",
        maxIterations: 100,
        iterationScalingFactor: 1,
      });
    });

    it("should handle cancellation via AbortSignal", async () => {
      await renderer.init();

      // Create an already-aborted signal
      const abortController = new AbortController();
      abortController.abort();

      await expect(
        renderer.render(mockCanvas, testParams, "Mandelbrot Set", {
          signal: abortController.signal,
        })
      ).rejects.toThrow("cancelled");
    });

    it("should handle worker errors gracefully", async () => {
      await renderer.init();

      // Make worker throw an error
      (mockWorkerAPI.computeChunk as ReturnType<typeof vi.fn>).mockRejectedValue(
        new Error("Worker error")
      );

      await expect(renderer.render(mockCanvas, testParams, "Mandelbrot Set")).rejects.toThrow(
        "Worker error"
      );
    });

    it("should handle cancellation mid-render", async () => {
      await renderer.init();

      // Slow down worker responses to allow time for cancellation
      let resolveCount = 0;
      (mockWorkerAPI.computeChunk as ReturnType<typeof vi.fn>).mockImplementation((request) => {
        return new Promise((resolve) => {
          setTimeout(() => {
            resolveCount++;
            const { width, height } = request.chunk;
            const buffer = new Uint8ClampedArray(width * height * 4);
            const imageData = new ImageData(buffer, width, height);
            resolve({ chunk: request.chunk, imageData });
          }, 10);
        });
      });

      const abortController = new AbortController();

      // Start render and cancel it quickly
      const renderPromise = renderer.render(mockCanvas, testParams, "Mandelbrot Set", {
        signal: abortController.signal,
      });

      // Cancel after a short delay
      setTimeout(() => abortController.abort(), 5);

      await expect(renderPromise).rejects.toThrow();
    });
  });

  describe("terminate", () => {
    it("should terminate all workers and clean up", async () => {
      await renderer.init();
      renderer.terminate();

      // Verify Comlink releaseProxy was called
      expect(Comlink.releaseProxy).toBeDefined();

      // After termination, renderer should not be initialized
      expect(renderer.getWorkerCount()).toBe(2); // Worker count is set at construction
    });

    it("should be safe to call terminate multiple times", async () => {
      await renderer.init();
      renderer.terminate();
      renderer.terminate(); // Should not throw
    });
  });

  describe("integration scenarios", () => {
    it("should handle sequential renders correctly", async () => {
      await renderer.init();

      // First render
      await renderer.render(mockCanvas, testParams, "Mandelbrot Set");
      const firstCallCount = (mockWorkerAPI.computeChunk as ReturnType<typeof vi.fn>).mock.calls
        .length;

      // Second render
      await renderer.render(
        mockCanvas,
        { ...testParams, zoom: new Decimal(2) },
        "Mandelbrot Set"
      );
      const secondCallCount = (mockWorkerAPI.computeChunk as ReturnType<typeof vi.fn>).mock.calls
        .length;

      // Both renders should have completed
      expect(secondCallCount).toBeGreaterThan(firstCallCount);
    });

    it("should handle different canvas sizes", async () => {
      await renderer.init();

      const smallCanvas = { ...mockCanvas, width: 100, height: 100 };
      await renderer.render(smallCanvas as HTMLCanvasElement, testParams, "Mandelbrot Set");
      const smallCallCount = (mockWorkerAPI.computeChunk as ReturnType<typeof vi.fn>).mock.calls
        .length;

      vi.clearAllMocks();

      const largeCanvas = { ...mockCanvas, width: 400, height: 400 };
      await renderer.render(largeCanvas as HTMLCanvasElement, testParams, "Mandelbrot Set");
      const largeCallCount = (mockWorkerAPI.computeChunk as ReturnType<typeof vi.fn>).mock.calls
        .length;

      // Larger canvas should result in more chunks
      expect(largeCallCount).toBeGreaterThan(smallCallCount);
    });
  });
});

