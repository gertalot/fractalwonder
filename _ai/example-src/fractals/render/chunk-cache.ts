import { FractalParams } from "@/hooks/use-store";
import { Decimal } from "decimal.js";
import { RenderChunk } from "./chunks";

/**
 * Cache key that uniquely identifies a chunk's computation parameters.
 * Combines chunk position/size with fractal parameters.
 */
export interface ChunkCacheKey {
  // Chunk bounds
  startX: number;
  startY: number;
  width: number;
  height: number;

  // Fractal parameters
  centerX: Decimal;
  centerY: Decimal;
  zoom: Decimal;
  maxIterations: number;

  // Canvas size (affects coordinate transformation)
  canvasWidth: number;
  canvasHeight: number;

  // Algorithm
  algorithmName: string;
}

/**
 * Serializes a cache key to a string for use in Map.
 * Uses fixed-precision for floating-point values to avoid cache misses from rounding errors.
 */
function serializeKey(key: ChunkCacheKey): string {
  return (
    `${key.startX},${key.startY},${key.width},${key.height}|` +
    `${key.centerX.toFixed(10)},${key.centerY.toFixed(10)}|` +
    `${key.zoom.toFixed(6)}|${key.maxIterations}|` +
    `${key.canvasWidth},${key.canvasHeight}|${key.algorithmName}`
  );
}

/**
 * Creates a cache key from render parameters.
 */
export function createCacheKey(
  chunk: RenderChunk,
  params: FractalParams,
  canvasWidth: number,
  canvasHeight: number,
  algorithmName: string
): ChunkCacheKey {
  return {
    startX: chunk.startX,
    startY: chunk.startY,
    width: chunk.width,
    height: chunk.height,
    centerX: params.center.x,
    centerY: params.center.y,
    zoom: params.zoom,
    maxIterations: params.maxIterations,
    canvasWidth,
    canvasHeight,
    algorithmName,
  };
}

/**
 * Entry in the cache, tracking access order for LRU eviction.
 */
interface CacheEntry {
  imageData: ImageData;
  accessTime: number;
}

/**
 * LRU cache for fractal chunk ImageData.
 *
 * Features:
 * - Least-Recently-Used eviction when max size exceeded
 * - Hit/miss statistics tracking
 * - Region-based invalidation for partial cache clears
 *
 * Usage:
 * ```typescript
 * const cache = new ChunkCache(100); // cache up to 100 chunks
 * const key = createCacheKey(chunk, params, width, height, algorithm);
 *
 * if (cache.has(key)) {
 *   const imageData = cache.get(key)!;
 *   // use cached data
 * } else {
 *   const imageData = await computeChunk(...);
 *   cache.set(key, imageData);
 * }
 * ```
 */
export class ChunkCache {
  private cache = new Map<string, CacheEntry>();
  private maxSize: number;
  private hits = 0;
  private misses = 0;

  /**
   * Creates a new chunk cache.
   * @param maxSize - Maximum number of chunks to cache (default: 100)
   */
  constructor(maxSize = 100) {
    this.maxSize = maxSize;
  }

  /**
   * Retrieves cached ImageData for the given key.
   * Updates access time for LRU tracking.
   * @returns ImageData if cached, undefined otherwise
   */
  get(key: ChunkCacheKey): ImageData | undefined {
    const serialized = serializeKey(key);
    const entry = this.cache.get(serialized);

    if (entry) {
      // Update access time
      entry.accessTime = Date.now();
      this.hits++;
      return entry.imageData;
    }

    this.misses++;
    return undefined;
  }

  /**
   * Stores ImageData in the cache.
   * If cache is full, evicts the least recently used entry.
   */
  set(key: ChunkCacheKey, imageData: ImageData): void {
    const serialized = serializeKey(key);

    // Evict LRU entry if cache is full
    if (this.cache.size >= this.maxSize && !this.cache.has(serialized)) {
      this.evictLRU();
    }

    this.cache.set(serialized, {
      imageData,
      accessTime: Date.now(),
    });
  }

  /**
   * Checks if a key is in the cache without updating access time.
   */
  has(key: ChunkCacheKey): boolean {
    return this.cache.has(serializeKey(key));
  }

  /**
   * Clears the entire cache.
   */
  clear(): void {
    this.cache.clear();
    this.hits = 0;
    this.misses = 0;
  }

  /**
   * Invalidates chunks that overlap with the given region.
   * Useful for partial cache invalidation during pan operations.
   */
  invalidateRegion(startX: number, startY: number, width: number, height: number): void {
    const endX = startX + width;
    const endY = startY + height;

    // Find keys to remove (we need to parse the serialized keys)
    const keysToRemove: string[] = [];

    for (const [serialized] of this.cache) {
      // Parse the serialized key to check overlap
      const parts = serialized.split("|")[0].split(",");
      const chunkStartX = parseInt(parts[0], 10);
      const chunkStartY = parseInt(parts[1], 10);
      const chunkWidth = parseInt(parts[2], 10);
      const chunkHeight = parseInt(parts[3], 10);
      const chunkEndX = chunkStartX + chunkWidth;
      const chunkEndY = chunkStartY + chunkHeight;

      // Check for overlap
      const overlaps = !(chunkEndX <= startX || chunkStartX >= endX || chunkEndY <= startY || chunkStartY >= endY);

      if (overlaps) {
        keysToRemove.push(serialized);
      }
    }

    for (const key of keysToRemove) {
      this.cache.delete(key);
    }
  }

  /**
   * Evicts the least recently used entry from the cache.
   */
  private evictLRU(): void {
    let oldestKey: string | null = null;
    let oldestTime = Infinity;

    for (const [key, entry] of this.cache) {
      if (entry.accessTime < oldestTime) {
        oldestTime = entry.accessTime;
        oldestKey = key;
      }
    }

    if (oldestKey) {
      this.cache.delete(oldestKey);
    }
  }

  /**
   * Gets cache statistics.
   */
  getStats(): { hits: number; misses: number; size: number; hitRate: number } {
    const total = this.hits + this.misses;
    return {
      hits: this.hits,
      misses: this.misses,
      size: this.cache.size,
      hitRate: total > 0 ? this.hits / total : 0,
    };
  }

  /**
   * Gets the current size of the cache.
   */
  getSize(): number {
    return this.cache.size;
  }

  /**
   * Gets the maximum size of the cache.
   */
  getMaxSize(): number {
    return this.maxSize;
  }

  /**
   * Sets a new maximum size for the cache.
   * If the new size is smaller, evicts LRU entries until within limit.
   */
  setMaxSize(maxSize: number): void {
    this.maxSize = maxSize;
    while (this.cache.size > this.maxSize) {
      this.evictLRU();
    }
  }
}
