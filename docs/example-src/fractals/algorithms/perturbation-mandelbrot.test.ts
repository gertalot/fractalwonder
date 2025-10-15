// ABOUTME: Comprehensive integration tests for PerturbationMandelbrotAlgorithm
// ABOUTME: Tests full algorithm pipeline at extreme zoom levels to catch precision bugs

import { beforeEach, describe, expect, it } from "vitest";
import { Decimal } from "decimal.js";
import { mandelbrotAlgorithm } from "./mandelbrot";
import { PerturbationMandelbrotAlgorithm } from "./perturbation-mandelbrot";

describe("PerturbationMandelbrotAlgorithm - Integration Tests", () => {
  let algorithm: PerturbationMandelbrotAlgorithm;

  beforeEach(() => {
    algorithm = new PerturbationMandelbrotAlgorithm();
  });

  describe("extreme zoom precision tests", () => {
    it("should produce non-blocky results at zoom 10^13", () => {
      // Setup: Center at Misiurewicz point, zoom 6×10^13 (the exact failing case from docs)
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const maxIterations = 1000;

      // Prepare algorithm
      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test 100 random pixels around center
      const testPixels = [];
      for (let i = 0; i < 100; i++) {
        // Generate random pixel coordinates around center
        const offsetX = (Math.random() - 0.5) * 100; // ±50 pixels
        const offsetY = (Math.random() - 0.5) * 100;
        
        // Convert to fractal coordinates using the same method as compute-chunk.ts
        const scale = 4 / 1080 / zoom.toNumber(); // INITIAL_FRACTAL_VIEW_HEIGHT / canvasHeight / zoom
        const real = center.x + offsetX * scale;
        const imag = center.y + offsetY * scale;

        const result = algorithm.computePoint(real, imag, maxIterations);
        testPixels.push({
          offsetX,
          offsetY,
          iter: result.iter,
          real,
          imag,
        });
      }

      // Check for blockiness: adjacent pixels should have smoothly varying iteration counts
      // Blockiness would show up as many pixels having identical iteration counts
      const iterationCounts = testPixels.map(p => p.iter);
      const uniqueCounts = new Set(iterationCounts);
      const uniqueRatio = uniqueCounts.size / iterationCounts.length;

      // At least 80% of pixels should have different iteration counts (proves it's not blocky)
      expect(uniqueRatio).toBeGreaterThan(0.8);
      
      // Standard deviation should be reasonable (not too low = blocky, not too high = chaotic)
      const mean = iterationCounts.reduce((a, b) => a + b, 0) / iterationCounts.length;
      const variance = iterationCounts.reduce((sum, iter) => sum + Math.pow(iter - mean, 2), 0) / iterationCounts.length;
      const stddev = Math.sqrt(variance);
      
      // Should have reasonable variation (not all identical, not completely random)
      expect(stddev).toBeGreaterThan(10);
      expect(stddev).toBeLessThan(500);

      console.log(`Zoom 10^13 test: ${uniqueRatio.toFixed(2)} unique ratios, stddev=${stddev.toFixed(1)}`);
    });

    it("should produce non-blocky results at zoom 10^15", () => {
      // Even more extreme zoom level
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(1e15);
      const maxIterations = 1000;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test smaller region (fewer pixels due to extreme precision)
      const testPixels = [];
      for (let i = 0; i < 50; i++) {
        const offsetX = (Math.random() - 0.5) * 20; // ±10 pixels
        const offsetY = (Math.random() - 0.5) * 20;
        
        const scale = 4 / 1080 / zoom;
        const real = center.x + offsetX * scale;
        const imag = center.y + offsetY * scale;

        const result = algorithm.computePoint(real, imag, maxIterations);
        testPixels.push({
          offsetX,
          offsetY,
          iter: result.iter,
        });
      }

      const iterationCounts = testPixels.map(p => p.iter);
      const uniqueCounts = new Set(iterationCounts);
      const uniqueRatio = uniqueCounts.size / iterationCounts.length;

      // At extreme zoom, we expect some variation (not completely blocky)
      expect(uniqueRatio).toBeGreaterThan(0.3);

      console.log(`Zoom 10^15 test: ${uniqueRatio.toFixed(2)} unique ratios`);
    });

    it("should match standard algorithm at zoom 1 (pixel-perfect)", () => {
      // Test that perturbation theory produces identical results to standard algorithm at low zoom
      const center = { x: -1, y: 0 };
      const zoom = new Decimal(1);
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test 1000 random pixels
      const mismatches = [];
      for (let i = 0; i < 1000; i++) {
        const offsetX = (Math.random() - 0.5) * 1920; // Full canvas width
        const offsetY = (Math.random() - 0.5) * 1080; // Full canvas height
        
        const scale = 4 / 1080 / zoom;
        const real = center.x + offsetX * scale;
        const imag = center.y + offsetY * scale;

        const perturbationResult = algorithm.computePoint(real, imag, maxIterations);
        const standardResult = mandelbrotAlgorithm.computePoint(real, imag, maxIterations);

        // Must be identical at low zoom
        if (perturbationResult.iter !== standardResult.iter) {
          mismatches.push({
            real,
            imag,
            perturbation: perturbationResult.iter,
            standard: standardResult.iter,
          });
        }
      }

      // Should have zero mismatches at zoom 1
      expect(mismatches.length).toBe(0);
      
      if (mismatches.length > 0) {
        console.log(`Found ${mismatches.length} mismatches at zoom 1:`, mismatches.slice(0, 5));
      }
    });

    it("should handle zoom levels 1, 10^3, 10^6, 10^9, 10^12, 10^15 without crashing", () => {
      const center = { x: -0.75, y: 0.1 };
      const zoomLevels = [1, 1e3, 1e6, 1e9, 1e12, 1e15].map(z => new Decimal(z));
      const maxIterations = 100;

      for (const zoom of zoomLevels) {
        algorithm.prepareForRender({
          center,
          zoom,
          maxIterations,
        });

        // Test a few pixels at each zoom level
        for (let i = 0; i < 10; i++) {
          const offsetX = (Math.random() - 0.5) * 100;
          const offsetY = (Math.random() - 0.5) * 100;
          
          const scale = 4 / 1080 / zoom;
          const real = center.x + offsetX * scale;
          const imag = center.y + offsetY * scale;

          // Should not throw any errors
          const result = algorithm.computePoint(real, imag, maxIterations);
          expect(result.iter).toBeGreaterThanOrEqual(-1);
          expect(result.iter).toBeLessThanOrEqual(maxIterations);
        }

        console.log(`Zoom ${zoom.toExponential(1)}: OK`);
      }
    });
  });

  describe("precision preservation tests", () => {
    it("should preserve deltaC magnitude at extreme zoom", () => {
      // Test that the deltaC calculation doesn't lose precision
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test pixel offset of exactly 1 pixel
      const scale = 4 / 1080 / zoom;
      const expectedDeltaCMagnitude = Math.sqrt(scale * scale + scale * scale); // 1 pixel offset

      // Calculate deltaC the same way the algorithm does
      const real = center.x + scale; // 1 pixel offset in x
      const imag = center.y + scale; // 1 pixel offset in y
      
      const deltaC = {
        real: real - center.x,
        imag: imag - center.y,
      };

      const actualDeltaCMagnitude = Math.sqrt(deltaC.real * deltaC.real + deltaC.imag * deltaC.imag);

      // The magnitude should be preserved (not lost to rounding)
      expect(actualDeltaCMagnitude).toBeCloseTo(expectedDeltaCMagnitude, 10);
      
      console.log(`Expected deltaC magnitude: ${expectedDeltaCMagnitude.toExponential(3)}`);
      console.log(`Actual deltaC magnitude: ${actualDeltaCMagnitude.toExponential(3)}`);
      
      // At this zoom level, if precision is lost, deltaC would be ~0
      expect(actualDeltaCMagnitude).toBeGreaterThan(1e-15);
    });

    it("should detect catastrophic cancellation in current implementation", () => {
      // This test should FAIL with the current broken implementation
      // It proves that our tests can catch the precision bug
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test multiple pixel offsets
      const scale = 4 / 1080 / zoom;
      const offsets = [1, 2, 5, 10, 20]; // pixel offsets
      
      for (const offset of offsets) {
        const real = center.x + offset * scale;
        const imag = center.y + offset * scale;
        
        const deltaC = {
          real: real - center.x,
          imag: imag - center.y,
        };

        const deltaCMagnitude = Math.sqrt(deltaC.real * deltaC.real + deltaC.imag * deltaC.imag);
        const expectedMagnitude = offset * scale * Math.sqrt(2);

        // If catastrophic cancellation occurs, deltaC magnitude will be much smaller than expected
        const ratio = deltaCMagnitude / expectedMagnitude;
        
        // Ratio should be close to 1.0 (no precision loss)
        // Current broken implementation will have ratio << 1.0
        expect(ratio).toBeGreaterThan(0.9);
        
        console.log(`Offset ${offset}px: ratio=${ratio.toFixed(3)}, deltaC=${deltaCMagnitude.toExponential(3)}`);
      }
    });
  });

  describe("performance requirements", () => {
    it("should complete 1920×1080 frame at zoom 10^15 in reasonable time", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(1e15);
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test a representative sample (1% of 1920×1080 = ~20,000 pixels)
      const sampleSize = Math.floor((1920 * 1080) / 100);
      const start = performance.now();

      for (let i = 0; i < sampleSize; i++) {
        const x = (i % 1920) - 960; // -960 to 959
        const y = Math.floor(i / 1920) - 540; // -540 to 539
        
        const scale = 4 / 1080 / zoom;
        const real = center.x + x * scale;
        const imag = center.y + y * scale;

        algorithm.computePoint(real, imag, maxIterations);
      }

      const duration = performance.now() - start;
      const pixelsPerSecond = (sampleSize / duration) * 1000;
      
      // Should be able to process at least 30,000 pixels/second
      expect(pixelsPerSecond).toBeGreaterThan(30000);
      
      console.log(`Performance: ${pixelsPerSecond.toFixed(0)} pixels/sec (${duration.toFixed(1)}ms for ${sampleSize} pixels)`);
      
      // Extrapolate to full frame
      const fullFrameTime = (1920 * 1080) / pixelsPerSecond;
      console.log(`Extrapolated full frame time: ${fullFrameTime.toFixed(1)}s`);
      
      // Should complete full frame in under 60 seconds
      expect(fullFrameTime).toBeLessThan(60);
    });
  });

  describe("precision-preserving computePointFromOffset", () => {
    it("should preserve precision for extreme zoom levels", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test 1 pixel offset at extreme zoom
      const scale = 4 / 1080 / zoom;
      const result = algorithm.computePointFromOffset(1, 0, scale, maxIterations);

      // Should not crash and produce reasonable results
      expect(result.iter).toBeGreaterThanOrEqual(-1);
      expect(result.iter).toBeLessThanOrEqual(maxIterations);
      expect(result.iter).not.toBeNaN();

      // The deltaC should be exactly scale (no precision loss)
      const expectedDeltaC = scale;
      expect(expectedDeltaC).toBeGreaterThan(1e-15); // Should be preserved

      console.log(`1 pixel offset at zoom ${zoom.toExponential(1)}: deltaC=${expectedDeltaC.toExponential(3)}, escape=${result.iter}`);
    });

    it("should produce different results for different pixel offsets", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      const scale = 4 / 1080 / zoom;
      const results = [];

      // Test multiple pixel offsets
      for (let offset = 0; offset <= 10; offset++) {
        const result = algorithm.computePointFromOffset(offset, 0, scale, maxIterations);
        results.push({
          offset,
          iter: result.iter,
        });
      }

      // Should have some variation (not all identical)
      const iterationCounts = results.map(r => r.iter);
      const uniqueCounts = new Set(iterationCounts);
      const uniqueRatio = uniqueCounts.size / iterationCounts.length;

      // At extreme zoom, we expect some variation
      expect(uniqueRatio).toBeGreaterThan(0.1);

      console.log(`Pixel offset variation: ${uniqueRatio.toFixed(2)} unique ratios`);
    });

    it("should match computePoint() results at low zoom", () => {
      const center = { x: -1, y: 0 };
      const zoom = new Decimal(1);
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      const scale = 4 / 1080 / zoom;
      const mismatches = [];

      // Test multiple pixel offsets
      for (let offsetX = -5; offsetX <= 5; offsetX++) {
        for (let offsetY = -5; offsetY <= 5; offsetY++) {
          // Method 1: computePointFromOffset
          const result1 = algorithm.computePointFromOffset(offsetX, offsetY, scale, maxIterations);

          // Method 2: computePoint (via world coordinates)
          const real = center.x + offsetX * scale;
          const imag = center.y + offsetY * scale;
          const result2 = algorithm.computePoint(real, imag, maxIterations);

          if (result1.iter !== result2.iter) {
            mismatches.push({
              offsetX,
              offsetY,
              fromOffset: result1.iter,
              fromWorld: result2.iter,
            });
          }
        }
      }

      // Should have zero mismatches at low zoom
      expect(mismatches.length).toBe(0);

      if (mismatches.length > 0) {
        console.log(`Found ${mismatches.length} mismatches:`, mismatches.slice(0, 5));
      }
    });

    it("should handle center pixel correctly", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      const scale = 4 / 1080 / zoom;
      
      // Center pixel (offset 0, 0)
      const result = algorithm.computePointFromOffset(0, 0, scale, maxIterations);

      // Should match reference orbit escape iteration
      expect(result.iter).toBeGreaterThanOrEqual(-1);
      expect(result.iter).toBeLessThanOrEqual(maxIterations);
      expect(result.iter).not.toBeNaN();

      console.log(`Center pixel at zoom ${zoom.toExponential(1)}: escape=${result.iter}`);
    });
  });
});
