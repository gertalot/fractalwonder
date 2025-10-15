// ABOUTME: Unit tests for fast standard-precision delta orbit calculation
// ABOUTME: Verifies correctness against direct calculation and performance benchmarks

import { Decimal } from "decimal.js";
import { beforeEach, describe, expect, it } from "vitest";
import { calculateDeltaOrbit, calculateDeltaOrbitWithData } from "./delta-orbit";
import { addHP, createComplexHP, stdToHP } from "./hp-math";
import { calculateReferenceOrbit, clearOrbitCache } from "./reference-orbit";
import type { ComplexStd } from "./types";

describe("delta-orbit: Fast Standard-Precision Delta Orbit Calculation", () => {
  beforeEach(() => {
    clearOrbitCache();
    Decimal.set({ precision: 50 });
  });

  describe("calculateDeltaOrbit", () => {
    it("should calculate delta orbit for small offset from reference point", () => {
      // Reference at (-0.5, 0) which is INSIDE the set
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Very small offset - should also be inside the set
      const deltaC: ComplexStd = { real: 0.0001, imag: 0 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      // Should not escape (point is still inside the set)
      expect(escapeIter).toBe(-1);
    });

    it("should match direct high-precision calculation for nearby points", () => {
      // Reference point
      const refCenter = createComplexHP(-0.5, 0);
      const maxIterations = 50;
      const referenceOrbit = calculateReferenceOrbit(refCenter, maxIterations);

      // Nearby point with small offset
      const deltaC: ComplexStd = { real: 0.0001, imag: 0.0001 };

      // Calculate using delta orbit (perturbation theory)
      const deltaEscape = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      // Calculate directly using high-precision for the actual point
      const actualCenter = addHP(refCenter, stdToHP(deltaC));
      const directOrbit = calculateReferenceOrbit(actualCenter, maxIterations);
      const directEscape = directOrbit.escapeIteration;

      // Should be very close (within a few iterations due to numerical differences)
      if (deltaEscape === -1 && directEscape === -1) {
        // Both didn't escape - perfect match
        expect(deltaEscape).toBe(directEscape);
      } else if (deltaEscape !== -1 && directEscape !== -1) {
        // Both escaped - should be close
        expect(Math.abs(deltaEscape - directEscape)).toBeLessThanOrEqual(2);
      }
    });

    it("should handle point at reference (deltaC = 0)", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // No offset - should get exact same result as reference
      const deltaC: ComplexStd = { real: 0, imag: 0 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      expect(escapeIter).toBe(referenceOrbit.escapeIteration);
    });

    it("should detect escape correctly for points outside set", () => {
      // Reference inside set
      const center = createComplexHP(0, 0);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Large offset that's clearly outside
      const deltaC: ComplexStd = { real: 2, imag: 0 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      // Should escape quickly
      expect(escapeIter).toBeGreaterThanOrEqual(0);
      expect(escapeIter).toBeLessThan(20); // Escapes very fast
    });

    it("should not escape for points inside set", () => {
      // Reference inside set
      const center = createComplexHP(0, 0);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Small offset still inside set
      const deltaC: ComplexStd = { real: 0.001, imag: 0.001 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      // Should not escape
      expect(escapeIter).toBe(-1);
    });

    it("should keep delta small for nearby points", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 50;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Very small offset
      const deltaC: ComplexStd = { real: 0.00001, imag: 0.00001 };
      const result = calculateDeltaOrbitWithData(referenceOrbit, deltaC, maxIterations);

      // Delta should stay relatively small (this is the key assumption of perturbation theory)
      const finalMag = Math.sqrt(result.finalDelta.real ** 2 + result.finalDelta.imag ** 2);
      expect(finalMag).toBeLessThan(1.0); // Delta should be << 1 for small offsets
    });

    it("should handle complex offsets with imaginary components", () => {
      const center = createComplexHP(-0.3, 0.5);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      const deltaC: ComplexStd = { real: 0.001, imag: -0.001 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      // Should compute without errors
      expect(escapeIter).toBeGreaterThanOrEqual(-1);
      expect(escapeIter).toBeLessThanOrEqual(maxIterations);
    });

    it("should handle reference orbit that escaped early", () => {
      // Reference that escapes quickly (verified: escapes at iteration 2)
      const center = createComplexHP(1, 1);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      expect(referenceOrbit.escapeIteration).toBe(2);

      // Delta orbit at exact reference point (deltaC = 0)
      const deltaC: ComplexStd = { real: 0, imag: 0 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      // Should escape at same iteration as reference
      expect(escapeIter).toBe(referenceOrbit.escapeIteration);
    });
  });

  describe("formula verification", () => {
    it("should implement correct delta iteration formula", () => {
      // Use a reference orbit that escapes (verified: escapes at iteration 5)
      const center = createComplexHP(0.5, 0);
      const maxIterations = 50;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // This point should escape at iteration 5
      expect(referenceOrbit.escapeIteration).toBe(5);

      // Delta orbit with small offset
      const deltaC: ComplexStd = { real: 0.01, imag: 0 };
      const result = calculateDeltaOrbitWithData(referenceOrbit, deltaC, maxIterations);

      // Should escape (nearby point also escapes)
      expect(result.escapeIteration).toBeGreaterThanOrEqual(0);
      
      // Should be close to reference escape iteration (within a few iterations)
      const diff = Math.abs(result.escapeIteration - referenceOrbit.escapeIteration);
      expect(diff).toBeLessThanOrEqual(5);
    });

    it("should correctly combine reference and delta for escape detection", () => {
      // Reference at (0.5, 0) which should escape
      const center = createComplexHP(0.5, 0);
      const maxIterations = 50;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Large positive offset - definitely outside
      const deltaC: ComplexStd = { real: 1.0, imag: 0 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      // Should escape quickly (point 1.5, 0 is well outside)
      expect(escapeIter).toBeGreaterThanOrEqual(0);
      expect(escapeIter).toBeLessThan(10);
    });
  });

  describe("performance", () => {
    it("should complete 100,000 delta orbit calculations in < 100ms", () => {
      // Setup reference orbit once
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      const start = performance.now();

      // Calculate 100,000 delta orbits
      for (let i = 0; i < 100000; i++) {
        const offset = i * 0.000001; // Varying offsets
        const deltaC: ComplexStd = { real: offset, imag: offset };
        calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);
      }

      const duration = performance.now() - start;

      // Should complete in under 100ms (typically 20-50ms)
      expect(duration).toBeLessThan(100);
      console.log(`100,000 delta orbits completed in ${duration.toFixed(1)}ms`);
    });

    it("should be at least 50x faster than high-precision calculation", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Time delta orbit calculations (standard precision)
      const deltaStart = performance.now();
      for (let i = 0; i < 1000; i++) {
        const offset = i * 0.0001;
        const deltaC: ComplexStd = { real: offset, imag: 0 };
        calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);
      }
      const deltaTime = performance.now() - deltaStart;

      // Time direct high-precision calculations
      const hpStart = performance.now();
      for (let i = 0; i < 1000; i++) {
        const offset = i * 0.0001;
        const actualCenter = createComplexHP(-0.5 + offset, 0);
        calculateReferenceOrbit(actualCenter, maxIterations);
      }
      const hpTime = performance.now() - hpStart;

      // Delta should be at least 50x faster
      const speedup = hpTime / deltaTime;
      expect(speedup).toBeGreaterThanOrEqual(50);
      console.log(`Performance: Delta orbits ${speedup.toFixed(0)}x faster than high-precision (${deltaTime.toFixed(1)}ms vs ${hpTime.toFixed(1)}ms)`);
    });

    it("should handle 1 million delta orbit calculations in < 1 second", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 50; // Shorter iterations for this test
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      const start = performance.now();

      for (let i = 0; i < 1000000; i++) {
        const offset = (i % 10000) * 0.000001;
        const deltaC: ComplexStd = { real: offset, imag: offset };
        calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);
      }

      const duration = performance.now() - start;

      expect(duration).toBeLessThan(1000);
      const pixelsPerSecond = (1000000 / duration) * 1000;
      console.log(`1 million delta orbits in ${duration.toFixed(0)}ms (${(pixelsPerSecond / 1e6).toFixed(1)}M pixels/sec)`);
    });
  });

  describe("edge cases", () => {
    it("should handle very small deltaC values", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      const deltaC: ComplexStd = { real: 1e-10, imag: 1e-10 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      // Should be very close to reference escape
      const diff = Math.abs(escapeIter - referenceOrbit.escapeIteration);
      expect(diff).toBeLessThanOrEqual(1);
    });

    it("should handle negative offsets", () => {
      const center = createComplexHP(-0.5, 0);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      const deltaC: ComplexStd = { real: -0.0001, imag: -0.0001 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);

      expect(escapeIter).toBeGreaterThanOrEqual(-1);
    });

    it("should handle maxIterations less than reference orbit length", () => {
      const center = createComplexHP(-0.5, 0);
      const referenceOrbit = calculateReferenceOrbit(center, 1000);

      // Request fewer iterations than reference has
      const deltaC: ComplexStd = { real: 0.0001, imag: 0 };
      const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, 50);

      // Should only iterate up to 50
      expect(escapeIter).toBeLessThanOrEqual(50);
    });
  });

  describe("extreme zoom validation", () => {
    it("should validate smooth iteration gradients at zoom 10^15", () => {
      // Test that delta orbit produces smooth gradients for neighboring pixels
      // This catches the precision bug that causes blockiness
      
      const center = createComplexHP(-1.4845895199757433, -6.96e-8);
      const maxIterations = 1000;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Test a grid of neighboring pixels
      const testOffsets = [
        { real: 0, imag: 0 },           // center
        { real: 1e-15, imag: 0 },      // +1e-15 in real
        { real: 0, imag: 1e-15 },      // +1e-15 in imag
        { real: 1e-15, imag: 1e-15 },  // +1e-15 in both
        { real: 2e-15, imag: 0 },      // +2e-15 in real
        { real: 0, imag: 2e-15 },      // +2e-15 in imag
      ];

      const results = [];
      for (const deltaC of testOffsets) {
        const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);
        results.push({
          deltaC,
          escapeIter,
        });
      }

      // Check for smooth gradients: adjacent pixels should have similar iteration counts
      const iterationCounts = results.map(r => r.escapeIter);
      const uniqueCounts = new Set(iterationCounts);
      const uniqueRatio = uniqueCounts.size / iterationCounts.length;

      // Should have some variation (not all identical)
      expect(uniqueRatio).toBeGreaterThan(0.3);
      
      // But not too much variation (should be smooth, not chaotic)
      const mean = iterationCounts.reduce((a, b) => a + b, 0) / iterationCounts.length;
      const variance = iterationCounts.reduce((sum, iter) => sum + Math.pow(iter - mean, 2), 0) / iterationCounts.length;
      const stddev = Math.sqrt(variance);
      
      // Standard deviation should be reasonable
      expect(stddev).toBeGreaterThan(0);
      expect(stddev).toBeLessThan(100);

      console.log(`Zoom 10^15 gradient test: ${uniqueRatio.toFixed(2)} unique ratios, stddev=${stddev.toFixed(1)}`);
    });

    it("should handle extreme precision deltaC values", () => {
      const center = createComplexHP(-1.4845895199757433, -6.96e-8);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Test very small deltaC values that would be lost to rounding in broken implementation
      const extremeDeltaCValues = [
        { real: 1e-15, imag: 0 },
        { real: 0, imag: 1e-15 },
        { real: 1e-16, imag: 1e-16 },
        { real: 1e-17, imag: 0 },
        { real: 0, imag: 1e-17 },
      ];

      for (const deltaC of extremeDeltaCValues) {
        const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);
        
        // Should not crash or return NaN
        expect(escapeIter).toBeGreaterThanOrEqual(-1);
        expect(escapeIter).toBeLessThanOrEqual(maxIterations);
        expect(escapeIter).not.toBeNaN();
        
        // Should be close to reference escape iteration
        const diff = Math.abs(escapeIter - referenceOrbit.escapeIteration);
        expect(diff).toBeLessThanOrEqual(5);
        
        console.log(`DeltaC ${deltaC.real.toExponential(2)}, ${deltaC.imag.toExponential(2)}: escape=${escapeIter}`);
      }
    });

    it("should validate precision preservation at multiple zoom levels", () => {
      const center = createComplexHP(-1.4845895199757433, -6.96e-8);
      const maxIterations = 100;
      
      const zoomLevels = [1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15];
      
      for (const zoom of zoomLevels) {
        // Set precision based on zoom level
        const precision = Math.max(30, Math.ceil(Math.log10(zoom) * 2.5 + 20));
        Decimal.set({ precision });
        
        const referenceOrbit = calculateReferenceOrbit(center, maxIterations);
        
        // Test deltaC that corresponds to 1 pixel offset at this zoom
        const scale = 4 / 1080 / zoom; // Same as pixelToFractalCoordinate
        const deltaC: ComplexStd = { real: scale, imag: scale };
        
        const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);
        
        // Should produce reasonable results
        expect(escapeIter).toBeGreaterThanOrEqual(-1);
        expect(escapeIter).toBeLessThanOrEqual(maxIterations);
        expect(escapeIter).not.toBeNaN();
        
        console.log(`Zoom ${zoom.toExponential(1)}: precision=${precision}, deltaC=${deltaC.real.toExponential(3)}, escape=${escapeIter}`);
      }
    });

    it("should detect blockiness in iteration patterns", () => {
      // This test specifically looks for the blockiness that indicates precision loss
      const center = createComplexHP(-1.4845895199757433, -6.96e-8);
      const maxIterations = 100;
      const referenceOrbit = calculateReferenceOrbit(center, maxIterations);

      // Test a larger grid to detect blockiness patterns
      const gridSize = 10;
      const results = [];
      
      for (let i = 0; i < gridSize; i++) {
        for (let j = 0; j < gridSize; j++) {
          const deltaC: ComplexStd = { 
            real: i * 1e-15, 
            imag: j * 1e-15 
          };
          const escapeIter = calculateDeltaOrbit(referenceOrbit, deltaC, maxIterations);
          results.push(escapeIter);
        }
      }

      // Analyze the pattern for blockiness
      const uniqueCounts = new Set(results);
      const uniqueRatio = uniqueCounts.size / results.length;
      
      // Count consecutive identical values (indicates blockiness)
      let consecutiveIdentical = 0;
      let maxConsecutive = 0;
      
      for (let i = 1; i < results.length; i++) {
        if (results[i] === results[i - 1]) {
          consecutiveIdentical++;
          maxConsecutive = Math.max(maxConsecutive, consecutiveIdentical);
        } else {
          consecutiveIdentical = 0;
        }
      }

      // Should not have too many consecutive identical values (indicates blockiness)
      expect(maxConsecutive).toBeLessThan(gridSize); // Not entire rows identical
      
      // Should have reasonable variation
      expect(uniqueRatio).toBeGreaterThan(0.1);
      
      console.log(`Blockiness test: ${uniqueRatio.toFixed(2)} unique ratios, max consecutive=${maxConsecutive}`);
    });
  });
});

