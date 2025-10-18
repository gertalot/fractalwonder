/**
 * Metrics for a single chunk computation.
 */
export interface ChunkMetrics {
  chunkIndex: number;
  computeTime: number; // milliseconds
  fromCache: boolean;
}

/**
 * Metrics for a complete render session.
 */
export interface RenderSessionMetrics {
  sessionId: string;
  startTime: number;
  endTime: number;
  duration: number; // milliseconds
  totalChunks: number;
  completedChunks: number;
  cachedChunks: number;
  computedChunks: number;
  totalPixels: number;
  pixelsPerSecond: number;
  averageChunkTime: number;
  cacheHitRate: number;
}

/**
 * Active render session tracking.
 */
interface RenderSession {
  sessionId: string;
  startTime: number;
  totalChunks: number;
  completedChunks: number;
  cachedChunks: number;
  chunkMetrics: ChunkMetrics[];
  totalPixels: number;
}

/**
 * Performance monitor for tracking fractal rendering metrics.
 *
 * Features:
 * - Track multiple concurrent render sessions
 * - Per-chunk timing
 * - Cache hit rate tracking
 * - Throughput calculations (pixels/second)
 * - Session statistics and history
 *
 * Usage:
 * ```typescript
 * const monitor = new PerformanceMonitor();
 * const sessionId = monitor.startRender(totalChunks, totalPixels);
 *
 * // For each chunk completion:
 * monitor.recordChunk(sessionId, chunkIndex, computeTime, fromCache);
 *
 * const metrics = monitor.endRender(sessionId);
 * console.log(`Render took ${metrics.duration}ms at ${metrics.pixelsPerSecond} px/s`);
 * ```
 */
export class PerformanceMonitor {
  private activeSessions = new Map<string, RenderSession>();
  private completedSessions: RenderSessionMetrics[] = [];
  private maxHistorySize = 50; // Keep last 50 sessions

  /**
   * Starts a new render session.
   *
   * @param totalChunks - Total number of chunks to render
   * @param totalPixels - Total number of pixels (width * height)
   * @returns Session ID for tracking
   */
  startRender(totalChunks: number, totalPixels: number): string {
    const sessionId = `render-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

    this.activeSessions.set(sessionId, {
      sessionId,
      startTime: performance.now(),
      totalChunks,
      completedChunks: 0,
      cachedChunks: 0,
      chunkMetrics: [],
      totalPixels,
    });

    return sessionId;
  }

  /**
   * Records completion of a chunk.
   *
   * @param sessionId - Session ID from startRender
   * @param chunkIndex - Index of the completed chunk
   * @param computeTime - Time taken to compute (0 if from cache)
   * @param fromCache - Whether this chunk was retrieved from cache
   */
  recordChunk(sessionId: string, chunkIndex: number, computeTime: number, fromCache: boolean): void {
    const session = this.activeSessions.get(sessionId);
    if (!session) {
      console.warn(`PerformanceMonitor: Unknown session ${sessionId}`);
      return;
    }

    session.completedChunks++;
    if (fromCache) {
      session.cachedChunks++;
    }

    session.chunkMetrics.push({
      chunkIndex,
      computeTime,
      fromCache,
    });
  }

  /**
   * Ends a render session and calculates final metrics.
   *
   * @param sessionId - Session ID from startRender
   * @returns Final render metrics
   */
  endRender(sessionId: string): RenderSessionMetrics {
    const session = this.activeSessions.get(sessionId);
    if (!session) {
      throw new Error(`PerformanceMonitor: Unknown session ${sessionId}`);
    }

    const endTime = performance.now();
    const duration = endTime - session.startTime;
    const computedChunks = session.completedChunks - session.cachedChunks;

    // Calculate average chunk time (excluding cached chunks)
    const computedChunkTimes = session.chunkMetrics.filter((m) => !m.fromCache).map((m) => m.computeTime);
    const averageChunkTime =
      computedChunkTimes.length > 0 ? computedChunkTimes.reduce((a, b) => a + b, 0) / computedChunkTimes.length : 0;

    const cacheHitRate = session.completedChunks > 0 ? session.cachedChunks / session.completedChunks : 0;

    const pixelsPerSecond = duration > 0 ? (session.totalPixels / duration) * 1000 : 0;

    const metrics: RenderSessionMetrics = {
      sessionId,
      startTime: session.startTime,
      endTime,
      duration,
      totalChunks: session.totalChunks,
      completedChunks: session.completedChunks,
      cachedChunks: session.cachedChunks,
      computedChunks,
      totalPixels: session.totalPixels,
      pixelsPerSecond,
      averageChunkTime,
      cacheHitRate,
    };

    // Move to history
    this.completedSessions.push(metrics);
    if (this.completedSessions.length > this.maxHistorySize) {
      this.completedSessions.shift();
    }

    this.activeSessions.delete(sessionId);

    return metrics;
  }

  /**
   * Gets current progress of an active session.
   *
   * @param sessionId - Session ID from startRender
   * @returns Progress percentage (0-100) or null if session not found
   */
  getProgress(sessionId: string): number | null {
    const session = this.activeSessions.get(sessionId);
    if (!session) {
      return null;
    }

    return session.totalChunks > 0 ? (session.completedChunks / session.totalChunks) * 100 : 0;
  }

  /**
   * Gets metrics for the last completed render.
   *
   * @returns Last render metrics or null if no renders completed
   */
  getLastRenderMetrics(): RenderSessionMetrics | null {
    if (this.completedSessions.length === 0) {
      return null;
    }
    return this.completedSessions[this.completedSessions.length - 1];
  }

  /**
   * Gets summary statistics across all completed renders.
   */
  getStats(): {
    totalRenders: number;
    averageDuration: number;
    averagePixelsPerSecond: number;
    averageCacheHitRate: number;
  } {
    if (this.completedSessions.length === 0) {
      return {
        totalRenders: 0,
        averageDuration: 0,
        averagePixelsPerSecond: 0,
        averageCacheHitRate: 0,
      };
    }

    const totalDuration = this.completedSessions.reduce((sum, m) => sum + m.duration, 0);
    const totalPixelsPerSecond = this.completedSessions.reduce((sum, m) => sum + m.pixelsPerSecond, 0);
    const totalCacheHitRate = this.completedSessions.reduce((sum, m) => sum + m.cacheHitRate, 0);

    return {
      totalRenders: this.completedSessions.length,
      averageDuration: totalDuration / this.completedSessions.length,
      averagePixelsPerSecond: totalPixelsPerSecond / this.completedSessions.length,
      averageCacheHitRate: totalCacheHitRate / this.completedSessions.length,
    };
  }

  /**
   * Gets all completed render metrics.
   */
  getHistory(): RenderSessionMetrics[] {
    return [...this.completedSessions];
  }

  /**
   * Clears all history.
   */
  clearHistory(): void {
    this.completedSessions = [];
  }

  /**
   * Sets the maximum number of sessions to keep in history.
   */
  setMaxHistorySize(size: number): void {
    this.maxHistorySize = size;
    while (this.completedSessions.length > this.maxHistorySize) {
      this.completedSessions.shift();
    }
  }
}
