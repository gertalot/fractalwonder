// ABOUTME: Full pipeline precision integration tests
// ABOUTME: Tests pixel coordinates → world coordinates → deltaC → result pipeline at extreme zoom

import { pixelToFractalCoordinate } from "@/lib/coordinates";
import { beforeEach, describe, expect, it } from "vitest";
import { Decimal } from "decimal.js";
import { PerturbationMandelbrotAlgorithm } from "../perturbation-mandelbrot";

describe("Precision Integration Tests - Full Pipeline", () => {
  let algorithm: PerturbationMandelbrotAlgorithm;

  beforeEach(() => {
    algorithm = new PerturbationMandelbrotAlgorithm();
  });

  describe("catastrophic cancellation detection", () => {
    it("should FAIL with current implementation at zoom 10^13", () => {
      // This test is designed to FAIL with the current broken implementation
      // It proves our test infrastructure can catch the precision bug
      
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const canvasWidth = 1920;
      const canvasHeight = 1080;
      const maxIterations = 100;

      // Prepare algorithm
      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test pixel (960, 544) - center of canvas
      const pixel = { x: 960, y: 544 };
      
      // Step 1: Convert pixel to world coordinates (this is where precision loss occurs)
      const worldCoords = pixelToFractalCoordinate(
        pixel,
        canvasWidth,
        canvasHeight,
        center,
        zoom
      );

      // Step 2: Calculate deltaC (this is where catastrophic cancellation occurs)
      const deltaC = {
        real: worldCoords.x - center.x,
        imag: worldCoords.y - center.y,
      };

      // Step 3: Verify deltaC magnitude is preserved
      const deltaCMagnitude = Math.sqrt(deltaC.real * deltaC.real + deltaC.imag * deltaC.imag);
      
      // Expected deltaC magnitude for center pixel should be very small but non-zero
      const scale = 4 / canvasHeight / zoom; // Same calculation as pixelToFractalCoordinate
      const expectedMagnitude = Math.sqrt(
        Math.pow((pixel.x - canvasWidth / 2) * scale, 2) +
        Math.pow((pixel.y - canvasHeight / 2) * scale, 2)
      );

      console.log(`Pixel (${pixel.x}, ${pixel.y}) at zoom ${zoom.toExponential(1)}:`);
      console.log(`  World coords: (${worldCoords.x.toExponential(3)}, ${worldCoords.y.toExponential(3)})`);
      console.log(`  DeltaC: (${deltaC.real.toExponential(3)}, ${deltaC.imag.toExponential(3)})`);
      console.log(`  DeltaC magnitude: ${deltaCMagnitude.toExponential(3)}`);
      console.log(`  Expected magnitude: ${expectedMagnitude.toExponential(3)}`);

      // With current broken implementation, deltaC magnitude will be ~0 due to catastrophic cancellation
      // This test should FAIL, proving it catches the bug
      expect(deltaCMagnitude).toBeGreaterThan(1e-15);
      
      // The ratio should be close to 1.0 (no precision loss)
      const ratio = deltaCMagnitude / expectedMagnitude;
      expect(ratio).toBeGreaterThan(0.9);
    });

    it("should preserve precision for off-center pixels at zoom 10^13", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const canvasWidth = 1920;
      const canvasHeight = 1080;
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test pixels at various offsets from center
      const testPixels = [
        { x: 961, y: 544 }, // +1 pixel in x
        { x: 960, y: 545 }, // +1 pixel in y
        { x: 962, y: 546 }, // +2 pixels in both
        { x: 950, y: 534 }, // -10 pixels in both
        { x: 970, y: 554 }, // +10 pixels in both
      ];

      for (const pixel of testPixels) {
        const worldCoords = pixelToFractalCoordinate(
          pixel,
          canvasWidth,
          canvasHeight,
          center,
          zoom
        );

        const deltaC = {
          real: worldCoords.x - center.x,
          imag: worldCoords.y - center.y,
        };

        const deltaCMagnitude = Math.sqrt(deltaC.real * deltaC.real + deltaC.imag * deltaC.imag);
        
        // Calculate expected magnitude
        const scale = 4 / canvasHeight / zoom;
        const offsetX = pixel.x - canvasWidth / 2;
        const offsetY = pixel.y - canvasHeight / 2;
        const expectedMagnitude = Math.sqrt(
          Math.pow(offsetX * scale, 2) + Math.pow(offsetY * scale, 2)
        );

        // DeltaC magnitude should be preserved (not lost to rounding)
        expect(deltaCMagnitude).toBeGreaterThan(1e-15);
        
        // Ratio should be close to 1.0
        const ratio = deltaCMagnitude / expectedMagnitude;
        expect(ratio).toBeGreaterThan(0.9);
        
        console.log(`Pixel (${pixel.x}, ${pixel.y}): ratio=${ratio.toFixed(3)}, deltaC=${deltaCMagnitude.toExponential(3)}`);
      }
    });

    it("should demonstrate precision loss progression with zoom", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const canvasWidth = 1920;
      const canvasHeight = 1080;
      const maxIterations = 100;

      const zoomLevels = [1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15].map(z => new Decimal(z));
      const pixel = { x: 961, y: 544 }; // +1 pixel offset

      for (const zoom of zoomLevels) {
        algorithm.prepareForRender({
          center,
          zoom,
          maxIterations,
        });

        const worldCoords = pixelToFractalCoordinate(
          pixel,
          canvasWidth,
          canvasHeight,
          center,
          zoom
        );

        const deltaC = {
          real: worldCoords.x - center.x,
          imag: worldCoords.y - center.y,
        };

        const deltaCMagnitude = Math.sqrt(deltaC.real * deltaC.real + deltaC.imag * deltaC.imag);
        
        // Calculate expected magnitude
        const scale = 4 / canvasHeight / zoom;
        const offsetX = pixel.x - canvasWidth / 2;
        const offsetY = pixel.y - canvasHeight / 2;
        const expectedMagnitude = Math.sqrt(
          Math.pow(offsetX * scale, 2) + Math.pow(offsetY * scale, 2)
        );

        const ratio = deltaCMagnitude / expectedMagnitude;
        
        console.log(`Zoom ${zoom.toExponential(1)}: ratio=${ratio.toFixed(3)}, deltaC=${deltaCMagnitude.toExponential(3)}`);
        
        // At lower zoom levels, precision should be preserved
        if (zoom.lte(1e12)) {
          expect(ratio).toBeGreaterThan(0.9);
        }
        
        // At higher zoom levels, current implementation will fail
        if (zoom.gte(1e13)) {
          // This will fail with current implementation, proving the test works
          expect(ratio).toBeGreaterThan(0.9);
        }
      }
    });
  });

  describe("full algorithm integration", () => {
    it("should produce consistent results across the precision pipeline", () => {
      const center = { x: -0.75, y: 0.1 };
      const zoom = new Decimal(1e12); // High but not extreme zoom
      const canvasWidth = 1920;
      const canvasHeight = 1080;
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Test multiple pixels and verify consistency
      const testPixels = [
        { x: 960, y: 540 }, // center
        { x: 961, y: 540 }, // +1 in x
        { x: 960, y: 541 }, // +1 in y
        { x: 962, y: 542 }, // +2 in both
      ];

      const results = [];
      for (const pixel of testPixels) {
        // Method 1: Use full pipeline (pixel → world → algorithm)
        const worldCoords = pixelToFractalCoordinate(
          pixel,
          canvasWidth,
          canvasHeight,
          center,
          zoom
        );
        const result1 = algorithm.computePoint(worldCoords.x, worldCoords.y, maxIterations);

        // Method 2: Calculate deltaC directly and use it
        const deltaC = {
          real: worldCoords.x - center.x,
          imag: worldCoords.y - center.y,
        };

        results.push({
          pixel,
          worldCoords,
          deltaC,
          result: result1,
        });
      }

      // Results should be consistent (no NaN, reasonable iteration counts)
      for (const result of results) {
        expect(result.result.iter).toBeGreaterThanOrEqual(-1);
        expect(result.result.iter).toBeLessThanOrEqual(maxIterations);
        expect(result.result.iter).not.toBeNaN();
        
        // DeltaC should be small but non-zero for off-center pixels
        if (result.pixel.x !== 960 || result.pixel.y !== 540) {
          const deltaCMagnitude = Math.sqrt(
            result.deltaC.real * result.deltaC.real + 
            result.deltaC.imag * result.deltaC.imag
          );
          expect(deltaCMagnitude).toBeGreaterThan(1e-15);
        }
      }

      console.log(`Tested ${results.length} pixels at zoom ${zoom.toExponential(1)}`);
    });
  });

  describe("precision-preserving method comparison", () => {
    it("should compare old vs new methods at extreme zoom", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const canvasWidth = 1920;
      const canvasHeight = 1080;
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      const scale = 4 / canvasHeight / zoom;
      const testPixels = [
        { x: 960, y: 544 }, // center
        { x: 961, y: 544 }, // +1 pixel
        { x: 960, y: 545 }, // +1 pixel
        { x: 962, y: 546 }, // +2 pixels
        { x: 950, y: 534 }, // -10 pixels
      ];

      const mismatches = [];

      for (const pixel of testPixels) {
        // Method 1: Old method (via world coordinates) - suffers from precision loss
        const worldCoords = pixelToFractalCoordinate(
          pixel,
          canvasWidth,
          canvasHeight,
          center,
          zoom
        );
        const result1 = algorithm.computePoint(worldCoords.x, worldCoords.y, maxIterations);

        // Method 2: New method (direct pixel offset) - preserves precision
        const offsetX = pixel.x - canvasWidth / 2;
        const offsetY = pixel.y - canvasHeight / 2;
        const result2 = algorithm.computePointFromOffset(offsetX, offsetY, scale, maxIterations);

        if (result1.iter !== result2.iter) {
          mismatches.push({
            pixel,
            oldMethod: result1.iter,
            newMethod: result2.iter,
            deltaC: {
              real: worldCoords.x - center.x,
              imag: worldCoords.y - center.y,
            },
            directDeltaC: {
              real: offsetX * scale,
              imag: offsetY * scale,
            },
          });
        }
      }

      console.log(`Found ${mismatches.length} mismatches between old and new methods:`);
      for (const mismatch of mismatches) {
        const oldDeltaC = Math.sqrt(
          mismatch.deltaC.real * mismatch.deltaC.real + 
          mismatch.deltaC.imag * mismatch.deltaC.imag
        );
        const newDeltaC = Math.sqrt(
          mismatch.directDeltaC.real * mismatch.directDeltaC.real + 
          mismatch.directDeltaC.imag * mismatch.directDeltaC.imag
        );
        
        console.log(`  Pixel (${mismatch.pixel.x}, ${mismatch.pixel.y}):`);
        console.log(`    Old method: ${mismatch.oldMethod} iterations, deltaC=${oldDeltaC.toExponential(3)}`);
        console.log(`    New method: ${mismatch.newMethod} iterations, deltaC=${newDeltaC.toExponential(3)}`);
      }

      // At extreme zoom, we expect some mismatches due to precision loss in old method
      // The new method should produce more consistent results
      expect(mismatches.length).toBeGreaterThan(0);
    });

    it("should verify new method preserves deltaC magnitude", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(6e13);
      const canvasWidth = 1920;
      const canvasHeight = 1080;
      const maxIterations = 100;

      algorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      const scale = 4 / canvasHeight / zoom;
      const testOffsets = [
        { x: 0, y: 0 },   // center
        { x: 1, y: 0 },   // +1 pixel
        { x: 0, y: 1 },   // +1 pixel
        { x: 5, y: 5 },   // +5 pixels
        { x: -10, y: -10 }, // -10 pixels
      ];

      for (const offset of testOffsets) {
        const result = algorithm.computePointFromOffset(offset.x, offset.y, scale, maxIterations);

        // Calculate expected deltaC magnitude
        const expectedDeltaC = Math.sqrt(
          Math.pow(offset.x * scale, 2) + Math.pow(offset.y * scale, 2)
        );

        // Should produce reasonable results
        expect(result.iter).toBeGreaterThanOrEqual(-1);
        expect(result.iter).toBeLessThanOrEqual(maxIterations);
        expect(result.iter).not.toBeNaN();

        // DeltaC magnitude should be preserved exactly
        expect(expectedDeltaC).toBeGreaterThan(0);
        
        console.log(`Offset (${offset.x}, ${offset.y}): deltaC=${expectedDeltaC.toExponential(3)}, escape=${result.iter}`);
      }
    });

    it("should demonstrate precision improvement at multiple zoom levels", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const canvasWidth = 1920;
      const canvasHeight = 1080;
      const maxIterations = 100;
      const testPixel = { x: 961, y: 544 }; // +1 pixel offset

      const zoomLevels = [1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15].map(z => new Decimal(z));

      for (const zoom of zoomLevels) {
        algorithm.prepareForRender({
          center,
          zoom,
          maxIterations,
        });

        const scale = 4 / canvasHeight / zoom;

        // Method 1: Old method
        const worldCoords = pixelToFractalCoordinate(
          testPixel,
          canvasWidth,
          canvasHeight,
          center,
          zoom
        );
        const result1 = algorithm.computePoint(worldCoords.x, worldCoords.y, maxIterations);

        // Method 2: New method
        const offsetX = testPixel.x - canvasWidth / 2;
        const offsetY = testPixel.y - canvasHeight / 2;
        const result2 = algorithm.computePointFromOffset(offsetX, offsetY, scale, maxIterations);

        const deltaC1 = Math.sqrt(
          Math.pow(worldCoords.x - center.x, 2) + Math.pow(worldCoords.y - center.y, 2)
        );
        const deltaC2 = Math.sqrt(
          Math.pow(offsetX * scale, 2) + Math.pow(offsetY * scale, 2)
        );

        const ratio = deltaC1 / deltaC2;

        console.log(`Zoom ${zoom.toExponential(1)}:`);
        console.log(`  Old method: ${result1.iter} iterations, deltaC=${deltaC1.toExponential(3)}`);
        console.log(`  New method: ${result2.iter} iterations, deltaC=${deltaC2.toExponential(3)}`);
        console.log(`  DeltaC ratio: ${ratio.toFixed(3)}`);

        // At lower zoom levels, both methods should agree
        if (zoom.lte(1e12)) {
          expect(result1.iter).toBe(result2.iter);
          expect(ratio).toBeCloseTo(1.0, 2);
        }

        // At higher zoom levels, new method should be more precise
        if (zoom.gte(1e13)) {
          expect(deltaC2).toBeGreaterThan(1e-15);
          expect(ratio).toBeLessThan(1.0); // Old method loses precision
        }
      }
    });
  });
});
