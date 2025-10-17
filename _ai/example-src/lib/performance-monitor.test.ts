import { beforeEach, describe, expect, it } from "vitest";

import { PerformanceMonitor } from "./performance-monitor";

describe("PerformanceMonitor", () => {
  let monitor: PerformanceMonitor;

  beforeEach(() => {
    monitor = new PerformanceMonitor();
  });

  it("should start and end a render session", () => {
    const sessionId = monitor.startRender(100, 1920 * 1080);
    expect(sessionId).toBeTruthy();

    const metrics = monitor.endRender(sessionId);
    expect(metrics.sessionId).toBe(sessionId);
    expect(metrics.totalChunks).toBe(100);
    expect(metrics.totalPixels).toBe(1920 * 1080);
    expect(metrics.duration).toBeGreaterThanOrEqual(0);
  });

  it("should record chunk completions", () => {
    const sessionId = monitor.startRender(10, 10000);

    monitor.recordChunk(sessionId, 0, 10, false);
    monitor.recordChunk(sessionId, 1, 15, false);
    monitor.recordChunk(sessionId, 2, 0, true); // from cache

    const metrics = monitor.endRender(sessionId);
    expect(metrics.completedChunks).toBe(3);
    expect(metrics.cachedChunks).toBe(1);
    expect(metrics.computedChunks).toBe(2);
    expect(metrics.cacheHitRate).toBeCloseTo(1 / 3);
  });

  it("should calculate average chunk time excluding cached chunks", () => {
    const sessionId = monitor.startRender(10, 10000);

    monitor.recordChunk(sessionId, 0, 10, false);
    monitor.recordChunk(sessionId, 1, 20, false);
    monitor.recordChunk(sessionId, 2, 0, true); // from cache, should not affect average

    const metrics = monitor.endRender(sessionId);
    expect(metrics.averageChunkTime).toBe(15); // (10 + 20) / 2
  });

  it("should calculate pixels per second", () => {
    const sessionId = monitor.startRender(10, 10000);

    const metrics = monitor.endRender(sessionId);
    // Duration might be very small, so pixelsPerSecond might be 0 or very large
    expect(metrics.pixelsPerSecond).toBeGreaterThanOrEqual(0);
    expect(metrics.duration).toBeGreaterThanOrEqual(0);
  });

  it("should track progress during render", () => {
    const sessionId = monitor.startRender(10, 10000);

    expect(monitor.getProgress(sessionId)).toBe(0);

    monitor.recordChunk(sessionId, 0, 10, false);
    monitor.recordChunk(sessionId, 1, 10, false);
    monitor.recordChunk(sessionId, 2, 10, false);

    expect(monitor.getProgress(sessionId)).toBe(30);

    monitor.endRender(sessionId);
    expect(monitor.getProgress(sessionId)).toBeNull();
  });

  it("should maintain session history", () => {
    const session1 = monitor.startRender(10, 10000);
    monitor.endRender(session1);

    const session2 = monitor.startRender(20, 20000);
    monitor.endRender(session2);

    const history = monitor.getHistory();
    expect(history).toHaveLength(2);
    expect(history[0].sessionId).toBe(session1);
    expect(history[1].sessionId).toBe(session2);
  });

  it("should limit history size", () => {
    monitor.setMaxHistorySize(3);

    for (let i = 0; i < 5; i++) {
      const sessionId = monitor.startRender(10, 10000);
      monitor.endRender(sessionId);
    }

    const history = monitor.getHistory();
    expect(history).toHaveLength(3);
  });

  it("should get last render metrics", () => {
    expect(monitor.getLastRenderMetrics()).toBeNull();

    const sessionId = monitor.startRender(10, 10000);
    monitor.endRender(sessionId);

    const lastMetrics = monitor.getLastRenderMetrics();
    expect(lastMetrics).not.toBeNull();
    expect(lastMetrics!.sessionId).toBe(sessionId);
  });

  it("should calculate aggregate statistics", () => {
    const session1 = monitor.startRender(10, 10000);
    monitor.recordChunk(session1, 0, 10, false);
    monitor.recordChunk(session1, 1, 0, true);
    monitor.endRender(session1);

    const session2 = monitor.startRender(10, 10000);
    monitor.recordChunk(session2, 0, 10, false);
    monitor.recordChunk(session2, 1, 10, false);
    monitor.endRender(session2);

    const stats = monitor.getStats();
    expect(stats.totalRenders).toBe(2);
    expect(stats.averageDuration).toBeGreaterThanOrEqual(0);
    expect(stats.averagePixelsPerSecond).toBeGreaterThanOrEqual(0);
    expect(stats.averageCacheHitRate).toBeGreaterThan(0);
  });

  it("should clear history", () => {
    const sessionId = monitor.startRender(10, 10000);
    monitor.endRender(sessionId);

    expect(monitor.getHistory()).toHaveLength(1);

    monitor.clearHistory();
    expect(monitor.getHistory()).toHaveLength(0);
    expect(monitor.getLastRenderMetrics()).toBeNull();
  });

  it("should handle empty history in stats", () => {
    const stats = monitor.getStats();
    expect(stats.totalRenders).toBe(0);
    expect(stats.averageDuration).toBe(0);
    expect(stats.averagePixelsPerSecond).toBe(0);
    expect(stats.averageCacheHitRate).toBe(0);
  });
});

