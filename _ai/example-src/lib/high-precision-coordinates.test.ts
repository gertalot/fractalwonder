import { Decimal } from "decimal.js";
import { describe, expect, it } from "vitest";
import { fractalToPixelCoordinateHP, pixelToFractalCoordinateHP } from "./coordinates";

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
            const center = { x: new Decimal(centerX), y: new Decimal(centerY) };

            // Test center pixel
            const pixel = { x: width / 2, y: height / 2 };

            const fractalCoord = pixelToFractalCoordinateHP(pixel, width, height, center, zoom);

            // Verify fractal coordinates are finite and reasonable
            expect(Number.isFinite(fractalCoord.x.toNumber())).toBe(true);
            expect(Number.isFinite(fractalCoord.y.toNumber())).toBe(true);

            // Verify fractal coordinates are within reasonable bounds
            const expectedRange = 10; // Fractal coordinates should be within ±10 of center
            expect(Math.abs(fractalCoord.x.toNumber() - centerX)).toBeLessThan(expectedRange);
            expect(Math.abs(fractalCoord.y.toNumber() - centerY)).toBeLessThan(expectedRange);
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
            const center = { x: new Decimal(centerX), y: new Decimal(centerY) };

            // Test fractal coordinates near the center
            const fractalCoord = { x: new Decimal(centerX), y: new Decimal(centerY) };

            const pixelCoord = fractalToPixelCoordinateHP(fractalCoord, width, height, center, zoom);

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
            const center = { x: new Decimal(centerX), y: new Decimal(centerY) };

            // Test center pixel
            const originalPixel = { x: width / 2, y: height / 2 };

            // Convert pixel → fractal using HP
            const fractalCoord = pixelToFractalCoordinateHP(originalPixel, width, height, center, zoom);

            // Convert fractal → pixel using HP
            const roundTripPixel = fractalToPixelCoordinateHP(fractalCoord, width, height, center, zoom);

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
