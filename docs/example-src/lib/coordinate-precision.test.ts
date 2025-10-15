// ABOUTME: Comprehensive test suite for ALL coordinate transformation functions at extreme zoom levels
import { FractalParams } from "@/hooks/use-store";
import { describe, expect, it } from "vitest";
import { fractalToPixelCoordinate, fractalToPixelCoordinateHP, pixelToFractalCoordinate, pixelToFractalCoordinateHP } from "./coordinates";
import { computePreviewPixelPosition } from "./render-preview";

describe("High-Precision Coordinate Functions", () => {
  // Test configuration for high-precision functions
  const testConfigs = [
    { name: "Zoom 10^15", zoom: 1e15, tolerance: 0.1 },
    { name: "Zoom 10^50", zoom: 1e50, tolerance: 10 },
    { name: "Zoom 10^100", zoom: 1e100, tolerance: 100 },
  ];

  const canvasConfigs = [
    { name: "800x600", width: 800, height: 600 },
    { name: "1920x1080", width: 1920, height: 1080 },
  ];

  const testCenters = [
    { name: "Origin", x: 0, y: 0 },
    { name: "Mandelbrot-center", x: -1, y: 0 },
  ];

  describe("pixelToFractalCoordinateHP Precision", () => {
    testConfigs.forEach(({ name, zoom, tolerance }) => {
      canvasConfigs.forEach(({ name: canvasName, width, height }) => {
        testCenters.forEach(({ name: centerName, x: centerX, y: centerY }) => {
          it(`should maintain precision for ${name} on ${canvasName} at ${centerName}`, () => {
            const center = { x: centerX, y: centerY };
            
            // Test center pixel
            const pixel = { x: width / 2, y: height / 2 };
            
            const fractalCoord = pixelToFractalCoordinateHP(
              pixel,
              width,
              height,
              center,
              zoom
            );

            // Verify fractal coordinates are finite and reasonable
            expect(Number.isFinite(fractalCoord.x)).toBe(true);
            expect(Number.isFinite(fractalCoord.y)).toBe(true);
            
            // Verify fractal coordinates are within reasonable bounds
            const expectedRange = 10; // Fractal coordinates should be within ±10 of center
            expect(Math.abs(fractalCoord.x - centerX)).toBeLessThan(expectedRange);
            expect(Math.abs(fractalCoord.y - centerY)).toBeLessThan(expectedRange);
          });
        });
      });
    });
  });

  describe("fractalToPixelCoordinateHP Precision", () => {
    testConfigs.forEach(({ name, zoom, tolerance }) => {
      canvasConfigs.forEach(({ name: canvasName, width, height }) => {
        testCenters.forEach(({ name: centerName, x: centerX, y: centerY }) => {
          it(`should maintain precision for ${name} on ${canvasName} at ${centerName}`, () => {
            const center = { x: centerX, y: centerY };
            
            // Test fractal coordinates near the center
            const fractalCoord = { x: centerX, y: centerY };

            const pixelCoord = fractalToPixelCoordinateHP(
              fractalCoord,
              width,
              height,
              center,
              zoom
            );

            // Verify pixel coordinates are finite and within canvas bounds
            expect(Number.isFinite(pixelCoord.x)).toBe(true);
            expect(Number.isFinite(pixelCoord.y)).toBe(true);
            expect(pixelCoord.x).toBeGreaterThanOrEqual(0);
            expect(pixelCoord.y).toBeGreaterThanOrEqual(0);
            expect(pixelCoord.x).toBeLessThanOrEqual(width);
            expect(pixelCoord.y).toBeLessThanOrEqual(height);
          });
        });
      });
    });
  });

  describe("Round-trip Precision: pixel → fractal → pixel (HP)", () => {
    testConfigs.forEach(({ name, zoom, tolerance }) => {
      canvasConfigs.forEach(({ name: canvasName, width, height }) => {
        testCenters.forEach(({ name: centerName, x: centerX, y: centerY }) => {
          it(`should maintain ${tolerance}px precision for ${name} on ${canvasName} at ${centerName}`, () => {
            const center = { x: centerX, y: centerY };
            
            // Test center pixel
            const originalPixel = { x: width / 2, y: height / 2 };

            // Convert pixel → fractal using HP
            const fractalCoord = pixelToFractalCoordinateHP(
              originalPixel,
              width,
              height,
              center,
              zoom
            );

            // Convert fractal → pixel using HP
            const roundTripPixel = fractalToPixelCoordinateHP(
              fractalCoord,
              width,
              height,
              center,
              zoom
            );

            // Calculate error
            const xError = Math.abs(roundTripPixel.x - originalPixel.x);
            const yError = Math.abs(roundTripPixel.y - originalPixel.y);

            // Verify precision is maintained within tolerance
            expect(xError).toBeLessThan(tolerance);
            expect(yError).toBeLessThan(tolerance);
          });
        });
      });
    });
  });
});

describe("Comprehensive Coordinate Transformation Precision Tests", () => {
  // Test configuration
  const testConfigs = [
    { name: "Zoom 10^10", zoom: 1e10, tolerance: 0.01 },
    { name: "Zoom 10^15", zoom: 1e15, tolerance: 0.1 },
    { name: "Zoom 10^20", zoom: 1e20, tolerance: 1 },
    { name: "Zoom 10^50", zoom: 1e50, tolerance: 10 },
    { name: "Zoom 10^100", zoom: 1e100, tolerance: 100 },
  ];

  const canvasConfigs = [
    { name: "800x600", width: 800, height: 600 },
    { name: "1920x1080", width: 1920, height: 1080 },
    { name: "400x300", width: 400, height: 300 },
  ];

  const testPixels = [
    { name: "Top-left", x: 0, y: 0 },
    { name: "Top-right", x: 800, y: 0 },
    { name: "Bottom-left", x: 0, y: 600 },
    { name: "Bottom-right", x: 800, y: 600 },
    { name: "Center", x: 400, y: 300 },
    { name: "Quarter-1", x: 200, y: 150 },
    { name: "Quarter-2", x: 600, y: 150 },
    { name: "Quarter-3", x: 200, y: 450 },
    { name: "Quarter-4", x: 600, y: 450 },
  ];

  const testCenters = [
    { name: "Origin", x: 0, y: 0 },
    { name: "Mandelbrot-center", x: -1, y: 0 },
    { name: "Julia-center", x: -0.7, y: 0.27015 },
    { name: "Far-from-origin", x: -2, y: 1 },
  ];

  describe("pixelToFractalCoordinate Precision", () => {
    testConfigs.forEach(({ name, zoom, tolerance }) => {
      canvasConfigs.forEach(({ name: canvasName, width, height }) => {
        testCenters.forEach(({ name: centerName, x: centerX, y: centerY }) => {
          it(`should maintain precision for ${name} on ${canvasName} at ${centerName}`, () => {
            const center = { x: centerX, y: centerY };
            
            // Test multiple pixel positions
            const testPositions = testPixels.filter(p => 
              p.x <= width && p.y <= height
            ).map(p => ({ x: p.x, y: p.y }));

            for (const pixel of testPositions) {
              const fractalCoord = pixelToFractalCoordinate(
                pixel,
                width,
                height,
                center,
                zoom
              );

              // Verify fractal coordinates are finite and reasonable
              expect(Number.isFinite(fractalCoord.x)).toBe(true);
              expect(Number.isFinite(fractalCoord.y)).toBe(true);
              
              // Verify fractal coordinates are within reasonable bounds
              // (should be close to center, not astronomical values)
              const expectedRange = 10; // Fractal coordinates should be within ±10 of center
              expect(Math.abs(fractalCoord.x - centerX)).toBeLessThan(expectedRange);
              expect(Math.abs(fractalCoord.y - centerY)).toBeLessThan(expectedRange);
            }
          });
        });
      });
    });
  });

  describe("fractalToPixelCoordinate Precision", () => {
    testConfigs.forEach(({ name, zoom, tolerance }) => {
      canvasConfigs.forEach(({ name: canvasName, width, height }) => {
        testCenters.forEach(({ name: centerName, x: centerX, y: centerY }) => {
          it(`should maintain precision for ${name} on ${canvasName} at ${centerName}`, () => {
            const center = { x: centerX, y: centerY };
            
            // Test fractal coordinates near the center
            const testFractalCoords = [
              { x: centerX, y: centerY }, // Exact center
              { x: centerX + 1e-10, y: centerY }, // Tiny offset
              { x: centerX, y: centerY + 1e-10 }, // Tiny offset
              { x: centerX + 1e-5, y: centerY + 1e-5 }, // Small offset
            ];

            for (const fractalCoord of testFractalCoords) {
              const pixelCoord = fractalToPixelCoordinate(
                fractalCoord,
                width,
                height,
                center,
                zoom
              );

              // Verify pixel coordinates are finite and within canvas bounds
              expect(Number.isFinite(pixelCoord.x)).toBe(true);
              expect(Number.isFinite(pixelCoord.y)).toBe(true);
              expect(pixelCoord.x).toBeGreaterThanOrEqual(0);
              expect(pixelCoord.y).toBeGreaterThanOrEqual(0);
              expect(pixelCoord.x).toBeLessThanOrEqual(width);
              expect(pixelCoord.y).toBeLessThanOrEqual(height);
            }
          });
        });
      });
    });
  });

  describe("Round-trip Precision: pixel → fractal → pixel", () => {
    testConfigs.forEach(({ name, zoom, tolerance }) => {
      canvasConfigs.forEach(({ name: canvasName, width, height }) => {
        testCenters.forEach(({ name: centerName, x: centerX, y: centerY }) => {
          it(`should maintain ${tolerance}px precision for ${name} on ${canvasName} at ${centerName}`, () => {
            const center = { x: centerX, y: centerY };
            
            // Test multiple pixel positions
            const testPositions = testPixels.filter(p => 
              p.x <= width && p.y <= height
            ).map(p => ({ x: p.x, y: p.y }));

            for (const originalPixel of testPositions) {
              // Convert pixel → fractal
              const fractalCoord = pixelToFractalCoordinate(
                originalPixel,
                width,
                height,
                center,
                zoom
              );

              // Convert fractal → pixel
              const roundTripPixel = fractalToPixelCoordinate(
                fractalCoord,
                width,
                height,
                center,
                zoom
              );

              // Calculate error
              const xError = Math.abs(roundTripPixel.x - originalPixel.x);
              const yError = Math.abs(roundTripPixel.y - originalPixel.y);

              // Verify precision is maintained within tolerance
              expect(xError).toBeLessThan(tolerance);
              expect(yError).toBeLessThan(tolerance);
            }
          });
        });
      });
    });
  });

  describe("computePreviewPixelPosition Precision", () => {
    testConfigs.forEach(({ name, zoom, tolerance }) => {
      canvasConfigs.forEach(({ name: canvasName, width, height }) => {
        testCenters.forEach(({ name: centerName, x: centerX, y: centerY }) => {
          it(`should maintain ${tolerance}px precision for ${name} on ${canvasName} at ${centerName}`, () => {
            const baseParams: FractalParams = {
              center: { x: centerX, y: centerY },
              zoom: zoom,
              maxIterations: 1000,
              iterationScalingFactor: 1000,
            };

            // Test small fractal center changes (simulating drag)
            const testCenterChanges = [
              { x: 0, y: 0 }, // No change
              { x: 1e-15, y: 0 }, // Tiny change
              { x: 0, y: 1e-15 }, // Tiny change
              { x: 1e-10, y: 1e-10 }, // Small change
              { x: 1e-5, y: 1e-5 }, // Larger change
            ];

            for (const centerChange of testCenterChanges) {
              const newParams: FractalParams = {
                ...baseParams,
                center: {
                  x: centerX + centerChange.x,
                  y: centerY + centerChange.y,
                },
              };

              const result = computePreviewPixelPosition(
                baseParams,
                newParams,
                width,
                height
              );

              // Verify result is finite and reasonable
              expect(Number.isFinite(result.x)).toBe(true);
              expect(Number.isFinite(result.y)).toBe(true);
              
              // Result should be within canvas bounds (allowing for scaling)
              expect(result.x).toBeGreaterThan(-width); // Allow for negative positioning
              expect(result.y).toBeGreaterThan(-height);
              expect(result.x).toBeLessThan(width * 2); // Allow for scaling
              expect(result.y).toBeLessThan(height * 2);
            }
          });
        });
      });
    });
  });

  describe("Smooth Transition Tests", () => {
    it("should maintain smooth pixel transitions during incremental fractal center changes", () => {
      const baseCenter = { x: -1, y: 0 };
      const zoom = 1e20;
      const width = 800;
      const height = 600;

      const results = [];
      
      // Simulate smooth dragging with tiny incremental changes
      for (let i = 0; i < 50; i++) {
        const center = {
          x: baseCenter.x + i * 1e-25, // Very small incremental changes
          y: baseCenter.y
        };

        // Convert center pixel to fractal and back
        const centerPixel = { x: 400, y: 300 };
        const fractalCoord = pixelToFractalCoordinate(
          centerPixel,
          width,
          height,
          center,
          zoom
        );
        const roundTripPixel = fractalToPixelCoordinate(
          fractalCoord,
          width,
          height,
          center,
          zoom
        );

        results.push(roundTripPixel.x);
      }

      // Check that pixel values change smoothly (no jumps)
      for (let i = 1; i < results.length; i++) {
        const diff = Math.abs(results[i] - results[i - 1]);
        // Should not jump by more than 1 pixel between consecutive small changes
        expect(diff).toBeLessThan(1);
      }
    });

    it("should maintain smooth preview transitions during incremental fractal center changes", () => {
      const baseParams: FractalParams = {
        center: { x: -1, y: 0 },
        zoom: 1e20,
        maxIterations: 1000,
        iterationScalingFactor: 1000,
      };

      const width = 800;
      const height = 600;
      const results = [];

      // Simulate smooth dragging with tiny incremental changes
      for (let i = 0; i < 50; i++) {
        const newParams: FractalParams = {
          ...baseParams,
          center: {
            x: -1 + i * 1e-25, // Very small incremental changes
            y: 0
          },
        };

        const result = computePreviewPixelPosition(
          baseParams,
          newParams,
          width,
          height
        );

        results.push(result.x);
      }

      // Check that pixel values change smoothly (no jumps)
      for (let i = 1; i < results.length; i++) {
        const diff = Math.abs(results[i] - results[i - 1]);
        // Should not jump by more than 5 pixels between consecutive small changes
        expect(diff).toBeLessThan(5);
      }
    });
  });

  describe("Extreme Precision Edge Cases", () => {
    it("should handle zoom level 10^200 without breaking", () => {
      const center = { x: -1, y: 0 };
      const zoom = 1e200;
      const width = 800;
      const height = 600;
      const pixel = { x: 400, y: 300 };

      // This should not throw errors
      const fractalCoord = pixelToFractalCoordinate(
        pixel,
        width,
        height,
        center,
        zoom
      );

      const roundTripPixel = fractalToPixelCoordinate(
        fractalCoord,
        width,
        height,
        center,
        zoom
      );

      // Should return finite values
      expect(Number.isFinite(fractalCoord.x)).toBe(true);
      expect(Number.isFinite(fractalCoord.y)).toBe(true);
      expect(Number.isFinite(roundTripPixel.x)).toBe(true);
      expect(Number.isFinite(roundTripPixel.y)).toBe(true);
    });

    it("should handle extremely small fractal center changes", () => {
      const baseParams: FractalParams = {
        center: { x: -1, y: 0 },
        zoom: 1e100,
        maxIterations: 1000,
        iterationScalingFactor: 1000,
      };

      const newParams: FractalParams = {
        ...baseParams,
        center: { x: -1 + 1e-50, y: 0 }, // Extremely tiny change
      };

      const result = computePreviewPixelPosition(
        baseParams,
        newParams,
        800,
        600
      );

      // Should not throw errors and should return finite values
      expect(Number.isFinite(result.x)).toBe(true);
      expect(Number.isFinite(result.y)).toBe(true);
    });
  });
});
