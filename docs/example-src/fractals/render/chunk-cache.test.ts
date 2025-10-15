import { beforeEach, describe, expect, it } from "vitest";

import { ChunkCache, createCacheKey } from "./chunk-cache";

describe("ChunkCache", () => {
  let cache: ChunkCache;

  beforeEach(() => {
    cache = new ChunkCache(3); // Small cache for testing
  });

  it("should store and retrieve ImageData", () => {
    const key = createCacheKey(
      { startX: 0, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    const imageData = new ImageData(100, 100);
    cache.set(key, imageData);

    expect(cache.has(key)).toBe(true);
    const retrieved = cache.get(key);
    expect(retrieved).toBe(imageData);
  });

  it("should return undefined for non-existent keys", () => {
    const key = createCacheKey(
      { startX: 0, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    expect(cache.get(key)).toBeUndefined();
    expect(cache.has(key)).toBe(false);
  });

  it("should track cache hits and misses", () => {
    const key = createCacheKey(
      { startX: 0, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    const imageData = new ImageData(100, 100);
    
    // Miss
    cache.get(key);
    expect(cache.getStats().misses).toBe(1);
    expect(cache.getStats().hits).toBe(0);

    // Store
    cache.set(key, imageData);

    // Hit
    cache.get(key);
    expect(cache.getStats().hits).toBe(1);
    expect(cache.getStats().misses).toBe(1);
    expect(cache.getStats().hitRate).toBeCloseTo(0.5);
  });

  it("should evict LRU entry when cache is full", () => {
    const key1 = createCacheKey(
      { startX: 0, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    const key2 = createCacheKey(
      { startX: 100, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    const key3 = createCacheKey(
      { startX: 200, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    const key4 = createCacheKey(
      { startX: 300, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    const imageData = new ImageData(100, 100);

    // Fill cache to max
    cache.set(key1, imageData);
    cache.set(key2, imageData);
    cache.set(key3, imageData);
    expect(cache.getSize()).toBe(3);

    // Access key2 to make it more recently used
    cache.get(key2);

    // Add key4, should evict key1 (least recently used)
    cache.set(key4, imageData);
    expect(cache.getSize()).toBe(3);
    expect(cache.has(key1)).toBe(false);
    expect(cache.has(key2)).toBe(true);
    expect(cache.has(key3)).toBe(true);
    expect(cache.has(key4)).toBe(true);
  });

  it("should clear all entries", () => {
    const key = createCacheKey(
      { startX: 0, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    cache.set(key, new ImageData(100, 100));
    expect(cache.getSize()).toBe(1);

    cache.clear();
    expect(cache.getSize()).toBe(0);
    expect(cache.has(key)).toBe(false);
    expect(cache.getStats().hits).toBe(0);
    expect(cache.getStats().misses).toBe(0);
  });

  it("should invalidate region", () => {
    const key1 = createCacheKey(
      { startX: 0, startY: 0, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    const key2 = createCacheKey(
      { startX: 200, startY: 200, width: 100, height: 100 },
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      800,
      600,
      "Mandelbrot Set"
    );

    const imageData = new ImageData(100, 100);
    cache.set(key1, imageData);
    cache.set(key2, imageData);

    // Invalidate region that overlaps with key1 but not key2
    cache.invalidateRegion(50, 50, 100, 100);

    expect(cache.has(key1)).toBe(false); // Should be invalidated (overlaps)
    expect(cache.has(key2)).toBe(true);  // Should remain (no overlap)
  });

  it("should differentiate keys with different parameters", () => {
    const chunk = { startX: 0, startY: 0, width: 100, height: 100 };
    const canvas = { width: 800, height: 600 };
    
    const key1 = createCacheKey(
      chunk,
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
      canvas.width,
      canvas.height,
      "Mandelbrot Set"
    );

    const key2 = createCacheKey(
      chunk,
      { center: { x: -1, y: 0 }, zoom: 2, maxIterations: 1000, iterationScalingFactor: 1000 }, // Different zoom
      canvas.width,
      canvas.height,
      "Mandelbrot Set"
    );

    const key3 = createCacheKey(
      chunk,
      { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 2000, iterationScalingFactor: 1000 }, // Different maxIter
      canvas.width,
      canvas.height,
      "Mandelbrot Set"
    );

    const imageData = new ImageData(100, 100);
    cache.set(key1, imageData);

    expect(cache.has(key1)).toBe(true);
    expect(cache.has(key2)).toBe(false);
    expect(cache.has(key3)).toBe(false);
  });

  it("should update max size and evict excess entries", () => {
    const imageData = new ImageData(100, 100);
    
    // Fill cache with 3 entries
    for (let i = 0; i < 3; i++) {
      const key = createCacheKey(
        { startX: i * 100, startY: 0, width: 100, height: 100 },
        { center: { x: -1, y: 0 }, zoom: 1, maxIterations: 1000, iterationScalingFactor: 1000 },
        800,
        600,
        "Mandelbrot Set"
      );
      cache.set(key, imageData);
    }

    expect(cache.getSize()).toBe(3);

    // Reduce max size
    cache.setMaxSize(2);
    expect(cache.getSize()).toBe(2);
    expect(cache.getMaxSize()).toBe(2);
  });
});

