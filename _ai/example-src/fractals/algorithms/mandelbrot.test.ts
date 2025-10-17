import { describe, expect, it } from "vitest";
import { MandelbrotAlgorithm, mandelbrotAlgorithm } from "./mandelbrot";

describe("MandelbrotAlgorithm", () => {
  const algorithm = new MandelbrotAlgorithm();
  const maxIterations = 1000;

  describe("metadata", () => {
    it("should have correct name", () => {
      expect(algorithm.name).toBe("Mandelbrot Set");
    });

    it("should have a description", () => {
      expect(algorithm.description).toBeDefined();
      expect(typeof algorithm.description).toBe("string");
    });
  });

  describe("computePoint", () => {
    it("should iterate to maxIterations for point at origin (in the set)", () => {
      const result = algorithm.computePoint(0, 0, maxIterations);
      expect(result.iter).toBe(maxIterations);
    });

    it("should iterate to maxIterations for point at (-1, 0) (in the set)", () => {
      const result = algorithm.computePoint(-1, 0, maxIterations);
      expect(result.iter).toBe(maxIterations);
    });

    it("should escape quickly for point at (2, 2)", () => {
      const result = algorithm.computePoint(2, 2, maxIterations);
      expect(result.iter).toBeLessThan(5);
      expect(result.iter).toBe(1); // Escapes immediately: 0² + (2+2i) has magnitude > 2
    });

    it("should escape for point at (0.4, 0.4)", () => {
      const result = algorithm.computePoint(0.4, 0.4, maxIterations);
      expect(result.iter).toBeGreaterThan(0);
      expect(result.iter).toBeLessThan(maxIterations);
      // This point should escape after several iterations
      expect(result.iter).toBeGreaterThan(5);
    });

    it("should return correct final z values for escaped points", () => {
      const result = algorithm.computePoint(2, 2, maxIterations);
      // After escaping, |z|² should be > 4
      const magnitudeSquared = result.zr * result.zr + result.zi * result.zi;
      expect(magnitudeSquared).toBeGreaterThan(4);
    });

    it("should handle point at (-0.5, 0) (in the set)", () => {
      const result = algorithm.computePoint(-0.5, 0, maxIterations);
      expect(result.iter).toBe(maxIterations);
    });

    it("should escape for point at (0.5, 0)", () => {
      const result = algorithm.computePoint(0.5, 0, maxIterations);
      expect(result.iter).toBeLessThan(maxIterations);
    });

    it("should handle edge case: point at (-0.75, 0.1) (near boundary)", () => {
      const result = algorithm.computePoint(-0.75, 0.1, maxIterations);
      // This point is near the boundary and should escape after many iterations
      expect(result.iter).toBeGreaterThan(0);
      expect(result.iter).toBeLessThan(maxIterations);
    });

    it("should be consistent with repeated calls", () => {
      const result1 = algorithm.computePoint(0.4, 0.4, maxIterations);
      const result2 = algorithm.computePoint(0.4, 0.4, maxIterations);
      expect(result1.iter).toBe(result2.iter);
      expect(result1.zr).toBe(result2.zr);
      expect(result1.zi).toBe(result2.zi);
    });

    it("should respect maxIterations parameter", () => {
      const result1 = algorithm.computePoint(0, 0, 10);
      const result2 = algorithm.computePoint(0, 0, 100);
      const result3 = algorithm.computePoint(0, 0, 1000);
      expect(result1.iter).toBe(10);
      expect(result2.iter).toBe(100);
      expect(result3.iter).toBe(1000);
    });
  });

  describe("default instance", () => {
    it("should export a default instance", () => {
      expect(mandelbrotAlgorithm).toBeInstanceOf(MandelbrotAlgorithm);
    });

    it("should work the same as a new instance", () => {
      const result1 = mandelbrotAlgorithm.computePoint(0.3, 0.3, maxIterations);
      const result2 = algorithm.computePoint(0.3, 0.3, maxIterations);
      expect(result1.iter).toBe(result2.iter);
      expect(result1.zr).toBe(result2.zr);
      expect(result1.zi).toBe(result2.zi);
    });
  });
});

