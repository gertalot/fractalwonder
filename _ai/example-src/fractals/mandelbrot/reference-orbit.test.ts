// ABOUTME: Unit tests for high-precision reference orbit calculation
// ABOUTME: Verifies correctness, caching, and performance of orbit computation

import { Decimal } from "decimal.js";
import { beforeEach, describe, expect, it } from "vitest";
import { createComplexHP } from "./hp-math";
import {
    calculateReferenceOrbit,
    clearOrbitCache,
    getOrbitCacheStats,
} from "./reference-orbit";

describe("reference-orbit: High-Precision Reference Orbit Calculation", () => {
  beforeEach(() => {
    // Clear cache before each test for isolation
    clearOrbitCache();
    // Set consistent precision for tests
    Decimal.set({ precision: 50 });
  });

  describe("calculateReferenceOrbit", () => {
    it("should calculate orbit for point near boundary: center (-0.75, 0.1)", () => {
      // Point (-0.75, 0.1) is near the boundary and escapes
      const center = createComplexHP(-0.75, 0.1);
      const maxIterations = 1000;

      const orbit = calculateReferenceOrbit(center, maxIterations);

      // This point should escape eventually
      expect(orbit.escapeIteration).toBeGreaterThan(-1);
      expect(orbit.escapeIteration).toBeLessThan(maxIterations);
      expect(orbit.inSet).toBe(false);
      expect(orbit.points.length).toBe(orbit.escapeIteration + 1); // Z_0 through Z_escape
    });

    it("should calculate orbit for point inside set: center (0, 0)", () => {
      const center = createComplexHP(0, 0);
      const maxIterations = 100;

      const orbit = calculateReferenceOrbit(center, maxIterations);

      // Point (0, 0) is in the set (Z_n = 0 + 0 = 0 forever)
      expect(orbit.escapeIteration).toBe(-1);
      expect(orbit.inSet).toBe(true);
      expect(orbit.points.length).toBe(maxIterations + 1); // Z_0 through Z_maxIterations
    });

    it("should calculate orbit for point outside set: center (2, 0)", () => {
      const center = createComplexHP(2, 0);
      const maxIterations = 1000;

      const orbit = calculateReferenceOrbit(center, maxIterations);

      // Point (2, 0): Z_0=0, Z_1=2, |Z_1|²=4 (NOT > 4), Z_2=6, |Z_2|²=36 > 4, escape at n=2
      expect(orbit.escapeIteration).toBe(2);
      expect(orbit.inSet).toBe(false);
      expect(orbit.points.length).toBe(3); // Z_0, Z_1, Z_2
    });

    it("should start orbit at Z_0 = 0", () => {
      const center = createComplexHP(-0.5, 0);
      const orbit = calculateReferenceOrbit(center, 10);

      const z0 = orbit.points[0];
      expect(z0.real.toNumber()).toBe(0);
      expect(z0.imag.toNumber()).toBe(0);
    });

    it("should follow formula: Z_{n+1} = Z_n² + center", () => {
      const center = createComplexHP(-0.5, 0);
      const orbit = calculateReferenceOrbit(center, 10);

      // Verify Z_1 = Z_0² + center = 0² + (-0.5, 0) = (-0.5, 0)
      expect(orbit.points[1].real.toNumber()).toBeCloseTo(-0.5, 10);
      expect(orbit.points[1].imag.toNumber()).toBeCloseTo(0, 10);

      // Verify Z_2 = Z_1² + center = (-0.5)² + (-0.5) = 0.25 - 0.5 = -0.25
      expect(orbit.points[2].real.toNumber()).toBeCloseTo(-0.25, 10);
      expect(orbit.points[2].imag.toNumber()).toBeCloseTo(0, 10);
    });

    it("should maintain high precision at extreme coordinates", () => {
      // Use a coordinate that requires high precision
      const center = createComplexHP("0.123456789012345678901234567890", "0.987654321098765432109876543210");
      const orbit = calculateReferenceOrbit(center, 10);

      // Verify precision is maintained (should have many digits)
      const z1Real = orbit.points[1].real.toString();
      const z1Imag = orbit.points[1].imag.toString();

      // Z_1 should equal center for this iteration (Z_0 = 0, Z_1 = 0² + center = center)
      expect(z1Real).toMatch(/^0\.123456789012345678/);
      expect(z1Imag).toMatch(/^0\.987654321098765432/);
    });

    it("should handle complex numbers with non-zero imaginary parts", () => {
      const center = createComplexHP(0, 1); // Pure imaginary
      const orbit = calculateReferenceOrbit(center, 10);

      // Z_1 = 0² + (0 + i) = i
      expect(orbit.points[1].real.toNumber()).toBeCloseTo(0, 10);
      expect(orbit.points[1].imag.toNumber()).toBeCloseTo(1, 10);

      // Z_2 = i² + i = -1 + i
      expect(orbit.points[2].real.toNumber()).toBeCloseTo(-1, 10);
      expect(orbit.points[2].imag.toNumber()).toBeCloseTo(1, 10);
    });
  });

  describe("caching behavior", () => {
    it("should cache calculated orbits", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 100;

      const statsBefore = getOrbitCacheStats();
      expect(statsBefore.size).toBe(0);

      calculateReferenceOrbit(center, maxIterations);

      const statsAfter = getOrbitCacheStats();
      expect(statsAfter.size).toBe(1);
    });

    it("should return cached orbit on second call (same params)", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 100;

      const orbit1 = calculateReferenceOrbit(center, maxIterations);
      const orbit2 = calculateReferenceOrbit(center, maxIterations);

      // Should be exact same object (cache hit)
      expect(orbit2).toBe(orbit1);
      expect(getOrbitCacheStats().size).toBe(1);
    });

    it("should create different cache entries for different centers", () => {
      const center1 = createComplexHP(-0.5, 0);
      const center2 = createComplexHP(-0.4, 0);
      const maxIterations = 100;

      calculateReferenceOrbit(center1, maxIterations);
      calculateReferenceOrbit(center2, maxIterations);

      expect(getOrbitCacheStats().size).toBe(2);
    });

    it("should create different cache entries for different maxIterations", () => {
      const center = createComplexHP(-0.5, 0);

      calculateReferenceOrbit(center, 100);
      calculateReferenceOrbit(center, 200);

      expect(getOrbitCacheStats().size).toBe(2);
    });

    it("should clear cache when requested", () => {
      const center = createComplexHP(-0.5, 0);
      calculateReferenceOrbit(center, 100);

      expect(getOrbitCacheStats().size).toBe(1);

      clearOrbitCache();

      expect(getOrbitCacheStats().size).toBe(0);
    });
  });

  describe("performance", () => {
    it("should complete 1,000 iterations in reasonable time", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 1000;

      const start = performance.now();
      calculateReferenceOrbit(center, maxIterations);
      const duration = performance.now() - start;

      // Should complete well under 1 second (typically 10-50ms)
      expect(duration).toBeLessThan(1000);
    });

    it("should complete 10,000 iterations in < 1 second", () => {
      const center = createComplexHP("-0.75", "0.1"); // Interesting point
      const maxIterations = 10000;

      const start = performance.now();
      calculateReferenceOrbit(center, maxIterations);
      const duration = performance.now() - start;

      // Performance requirement from Story 2
      expect(duration).toBeLessThan(1000);
      console.log(`10,000 iterations completed in ${duration.toFixed(1)}ms`);
    });

    it("should use cache for performance on repeated calls", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 10000;

      // First call (not cached)
      const start1 = performance.now();
      calculateReferenceOrbit(center, maxIterations);
      const duration1 = performance.now() - start1;

      // Second call (cached)
      const start2 = performance.now();
      calculateReferenceOrbit(center, maxIterations);
      const duration2 = performance.now() - start2;

      // Cached call should be much faster (< 1ms vs potentially 100ms+)
      expect(duration2).toBeLessThan(duration1);
      expect(duration2).toBeLessThan(10); // Cache lookup should be near-instant
      console.log(`Cache speedup: ${duration1.toFixed(1)}ms → ${duration2.toFixed(3)}ms`);
    });
  });

  describe("escape detection", () => {
    it("should detect escape when |Z|² > 4", () => {
      // Use a point that escapes quickly
      const center = createComplexHP(1, 1);
      const orbit = calculateReferenceOrbit(center, 100);

      expect(orbit.escapeIteration).toBeGreaterThan(-1);
      expect(orbit.inSet).toBe(false);

      // Verify the orbit actually escaped (last point has |Z|² > 4)
      const lastPoint = orbit.points[orbit.points.length - 1];
      const lastMagSq = lastPoint.real.times(lastPoint.real).plus(lastPoint.imag.times(lastPoint.imag));
      expect(lastMagSq.greaterThan(new Decimal(4))).toBe(true);
    });

    it("should not escape for point in the set", () => {
      const center = createComplexHP(-0.1, 0.1); // Inside the set
      const maxIterations = 1000;
      const orbit = calculateReferenceOrbit(center, maxIterations);

      expect(orbit.escapeIteration).toBe(-1);
      expect(orbit.inSet).toBe(true);
      expect(orbit.points.length).toBe(maxIterations + 1);
    });

    it("should handle boundary cases near |Z|² = 4", () => {
      // Point that gets very close to escape threshold
      const center = createComplexHP(0.25, 0); // c = 1/4 is on the boundary
      const orbit = calculateReferenceOrbit(center, 1000);

      // This should either escape late or reach maxIterations
      const escaped = orbit.escapeIteration !== -1;
      if (escaped) {
        expect(orbit.escapeIteration).toBeGreaterThan(10); // Takes many iterations
      }
    });
  });
});

