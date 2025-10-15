// ABOUTME: Orchestrates parallel fractal rendering using Web Workers
// ABOUTME: Manages worker pool and distributes chunks for progressive rendering

import * as Comlink from "comlink";
import { Decimal } from "decimal.js";

import { FractalParams } from "@/hooks/use-store";
import { PerformanceMonitor } from "@/lib/performance-monitor";

import type { FractalWorkerAPI } from "../workers/fractal.worker";
import { ChunkComputeRequest, ChunkComputeResult, serializeFractalParams } from "../workers/types";
import { ChunkCache, createCacheKey } from "./chunk-cache";
import { calculateAdaptiveChunkCount, createChunks, RenderChunk } from "./chunks";

/**
 * Options for controlling render behavior
 */
export interface RenderOptions {
  /** Callback invoked with progress percentage (0-100) as chunks complete */
  onProgress?: (percent: number) => void;
  /** Signal to cancel the render operation */
  signal?: AbortSignal;
}

/**
 * ParallelRenderer orchestrates parallel fractal computation across multiple Web Workers.
 * 
 * Features:
 * - Progressive rendering with center-out spiral pattern
 * - Automatic worker pool management
 * - Cancellable render operations
 * - Progress tracking
 * 
 * Usage:
 * ```typescript
 * const renderer = new ParallelRenderer(4); // 4 workers
 * await renderer.init();
 * await renderer.render(canvas, params, {
 *   onProgress: (pct) => console.log(`${pct}% complete`),
 *   signal: abortController.signal
 * });
 * ```
 */
export class ParallelRenderer {
  private workers: Array<{ worker: Worker; api: Comlink.Remote<FractalWorkerAPI> }> = [];
  private workerCount: number;
  private isInitialized = false;
  private chunkCache: ChunkCache;
  private performanceMonitor: PerformanceMonitor;
  private lastRenderParams: {
    center: { x: Decimal; y: Decimal };
    zoom: Decimal;
    maxIterations: number;
    algorithmName: string;
  } | null = null;
  private currentSessionId: string | null = null;

  /**
   * Creates a new ParallelRenderer.
   * 
   * @param workerCount - Number of workers to create (defaults to 75% of CPU cores)
   * @param cacheSize - Maximum number of chunks to cache (default: 200)
   */
  constructor(workerCount?: number, cacheSize = 200) {
    this.workerCount = workerCount ?? this.getOptimalWorkerCount();
    this.chunkCache = new ChunkCache(cacheSize);
    this.performanceMonitor = new PerformanceMonitor();
  }

  /**
   * Calculates the optimal number of workers based on available CPU cores.
   * Uses 75% of cores, with a minimum of 2 and maximum of 16.
   */
  private getOptimalWorkerCount(): number {
    const cpuCount = navigator.hardwareConcurrency || 4;
    return Math.max(2, Math.min(16, Math.floor(cpuCount * 0.75)));
  }

  /**
   * Initializes the worker pool. Must be called before render().
   * Creates workers and wraps them with Comlink for RPC communication.
   */
  async init(): Promise<void> {
    if (this.isInitialized) {
      return;
    }

    // Dynamically import the worker
    const FractalWorker = await import("../workers/fractal.worker?worker").then(
      (module) => module.default
    );

    // Create and wrap workers
    for (let i = 0; i < this.workerCount; i++) {
      const worker = new FractalWorker();
      const api = Comlink.wrap<FractalWorkerAPI>(worker);

      // Test connectivity with ping
      const response = await api.ping();
      if (response !== "pong") {
        throw new Error(`Worker ${i} failed to respond to ping`);
      }

      this.workers.push({ worker, api });
    }

    this.isInitialized = true;
    console.log(`ParallelRenderer initialized with ${this.workerCount} workers`);
  }

  /**
   * Renders a fractal onto the canvas using parallel computation.
   * 
   * The canvas is divided into chunks that are distributed to workers.
   * Chunks render progressively in a center-out spiral pattern for better UX.
   * 
   * @param canvas - Target canvas element
   * @param params - Fractal parameters (center, zoom, iterations)
   * @param algorithmName - Name of the fractal algorithm to use
   * @param options - Render options (progress callback, abort signal)
   * @throws Error if not initialized or if render fails
   */
  async render(
    canvas: HTMLCanvasElement,
    params: FractalParams,
    algorithmName: string,
    options?: RenderOptions
  ): Promise<void> {
    if (!this.isInitialized) {
      throw new Error("ParallelRenderer not initialized. Call init() first.");
    }

    if (canvas.width === 0 || canvas.height === 0) {
      console.warn("Skipping render: Canvas dimensions are zero.");
      return;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Failed to get 2D context from canvas");
    }

    // Check if already aborted
    if (options?.signal?.aborted) {
      throw new Error("Render cancelled before starting");
    }

    const startTime = performance.now();
    console.log(
      `Starting parallel render: ${canvas.width}x${canvas.height}, ` +
        `center=(${params.center.x}, ${params.center.y}), zoom=${params.zoom}`
    );

    // Invalidate cache if parameters changed significantly
    this.invalidateCacheIfNeeded(params, algorithmName);

    // Create chunks in spiral order (center-out) with adaptive sizing based on zoom
    const adaptiveChunkCount = calculateAdaptiveChunkCount(params.zoom);
    const chunks = createChunks(canvas.width, canvas.height, {
      preferredNumber: adaptiveChunkCount,
      minSize: 20,
      maxSize: 1000,
    });
    const totalChunks = chunks.length;
    let completedChunks = 0;

    console.log(
      `Created ${totalChunks} chunks for parallel rendering (adaptive count: ${adaptiveChunkCount} for zoom ${params.zoom})`
    );

    // Start performance monitoring
    this.currentSessionId = this.performanceMonitor.startRender(
      totalChunks,
      canvas.width * canvas.height
    );

    // Process chunks as they complete (progressive rendering)
    try {
      // Start all chunk computations
      const chunkPromises = chunks.map((chunk, index) =>
        this.computeChunkWithWorker(
          chunk,
          index,
          canvas.width,
          canvas.height,
          params,
          algorithmName,
          options?.signal
        ).then((result) => {
          // Draw immediately as each chunk completes
          if (options?.signal?.aborted) {
            throw new Error("Render cancelled");
          }

          ctx.putImageData(result.imageData, result.chunk.startX, result.chunk.startY);

          // Update progress
          completedChunks++;
          const progress = (completedChunks / totalChunks) * 100;
          options?.onProgress?.(progress);

          return result;
        })
      );

      // Wait for all to complete (but drawing happens as each completes above)
      await Promise.all(chunkPromises);

      const duration = performance.now() - startTime;

      // End performance monitoring
      if (this.currentSessionId) {
        const metrics = this.performanceMonitor.endRender(this.currentSessionId);
        console.log(
          `Parallel render complete: ${totalChunks} chunks in ${duration.toFixed(1)}ms ` +
            `(${metrics.pixelsPerSecond.toFixed(0)} pixels/s, ` +
            `cache hit rate: ${(metrics.cacheHitRate * 100).toFixed(1)}%)`
        );
        this.currentSessionId = null;
      } else {
        console.log(
          `Parallel render complete: ${totalChunks} chunks in ${duration.toFixed(1)}ms ` +
            `(${((canvas.width * canvas.height) / duration).toFixed(0)} pixels/ms)`
        );
      }
    } catch (error) {
      if (options?.signal?.aborted || (error instanceof Error && error.message === "Render cancelled")) {
        console.log("Render cancelled by user");
        throw error;
      } else {
        console.error("Render failed:", error);
        throw error;
      }
    }
  }

  /**
   * Invalidates cache if render parameters changed significantly.
   * 
   * Cache invalidation rules:
   * - Zoom change: Clear entire cache (different scale)
   * - Algorithm change: Clear entire cache (different computation)
   * - maxIterations change: Clear entire cache (different detail level)
   * - Pan only (center change): Keep cache (chunks are position-independent in fractal space)
   */
  private invalidateCacheIfNeeded(params: FractalParams, algorithmName: string): void {
    if (!this.lastRenderParams) {
      // First render, nothing to invalidate
      this.lastRenderParams = {
        center: { ...params.center },
        zoom: params.zoom,
        maxIterations: params.maxIterations,
        algorithmName,
      };
      return;
    }

    // Check for parameter changes that require cache invalidation
    const zoomChanged = this.lastRenderParams.zoom instanceof Decimal && params.zoom instanceof Decimal 
      ? !this.lastRenderParams.zoom.equals(params.zoom)
      : this.lastRenderParams.zoom !== params.zoom;
    const algorithmChanged = this.lastRenderParams.algorithmName !== algorithmName;
    const iterationsChanged = this.lastRenderParams.maxIterations !== params.maxIterations;

    if (zoomChanged || algorithmChanged || iterationsChanged) {
      console.log(
        `Cache invalidated (zoom: ${zoomChanged}, algorithm: ${algorithmChanged}, iterations: ${iterationsChanged})`
      );
      this.chunkCache.clear();
    }

    // Update last render params
    this.lastRenderParams = {
      center: { ...params.center },
      zoom: params.zoom,
      maxIterations: params.maxIterations,
      algorithmName,
    };
  }

  /**
   * Computes a single chunk using the next available worker.
   * Workers are distributed in a round-robin fashion.
   * Uses cache if available to avoid recomputation.
   */
  private async computeChunkWithWorker(
    chunk: RenderChunk,
    chunkIndex: number,
    canvasWidth: number,
    canvasHeight: number,
    params: FractalParams,
    algorithmName: string,
    signal?: AbortSignal
  ): Promise<ChunkComputeResult> {
    // Check for cancellation
    if (signal?.aborted) {
      throw new Error("Render cancelled");
    }

    const chunkStartTime = performance.now();

    // Check cache first
    const cacheKey = createCacheKey(chunk, params, canvasWidth, canvasHeight, algorithmName);
    const cachedImageData = this.chunkCache.get(cacheKey);
    
    if (cachedImageData) {
      // Cache hit! Return immediately
      if (this.currentSessionId) {
        this.performanceMonitor.recordChunk(this.currentSessionId, chunkIndex, 0, true);
      }

      return {
        chunk: {
          startX: chunk.startX,
          startY: chunk.startY,
          width: chunk.width,
          height: chunk.height,
        },
        imageData: cachedImageData,
      };
    }

    // Cache miss - compute the chunk
    // Select worker in round-robin fashion
    const workerIndex = chunkIndex % this.workers.length;
    const { api } = this.workers[workerIndex];

    // Create request with serialized params to avoid DataCloneError
    const request: ChunkComputeRequest = {
      chunk: {
        startX: chunk.startX,
        startY: chunk.startY,
        width: chunk.width,
        height: chunk.height,
      },
      params: serializeFractalParams(params),
      canvasWidth,
      canvasHeight,
      algorithmName,
    };

    // Compute the chunk
    try {
      const result = await api.computeChunk(request);
      const chunkTime = performance.now() - chunkStartTime;

      // Store in cache for future use
      this.chunkCache.set(cacheKey, result.imageData);

      // Record metrics
      if (this.currentSessionId) {
        this.performanceMonitor.recordChunk(this.currentSessionId, chunkIndex, chunkTime, false);
      }
      
      return result;
    } catch (error) {
      console.error(`Worker ${workerIndex} failed to compute chunk ${chunkIndex}:`, error);
      throw error;
    }
  }

  /**
   * Terminates all workers and cleans up resources.
   * Should be called when the renderer is no longer needed.
   */
  terminate(): void {
    for (const { worker, api } of this.workers) {
      api[Comlink.releaseProxy]();
      worker.terminate();
    }
    this.workers = [];
    this.isInitialized = false;
    console.log("ParallelRenderer terminated");
  }

  /**
   * Gets the number of workers in the pool.
   */
  getWorkerCount(): number {
    return this.workerCount;
  }

  /**
   * Gets cache statistics (hits, misses, hit rate).
   */
  getCacheStats(): { hits: number; misses: number; size: number; hitRate: number } {
    return this.chunkCache.getStats();
  }

  /**
   * Clears the chunk cache.
   */
  clearCache(): void {
    this.chunkCache.clear();
  }

  /**
   * Gets the chunk cache instance (for advanced operations).
   */
  getCache(): ChunkCache {
    return this.chunkCache;
  }

  /**
   * Gets the performance monitor instance.
   */
  getPerformanceMonitor(): PerformanceMonitor {
    return this.performanceMonitor;
  }
}

