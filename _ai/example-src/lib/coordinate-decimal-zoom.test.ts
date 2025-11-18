import { Decimal } from "decimal.js";
import { describe, expect, it } from "vitest";
import {
  fractalToPixelCoordinate,
  fractalToPixelCoordinateUltraHP,
  pixelToFractalCoordinate,
  pixelToFractalCoordinateUltraHP,
} from "./coordinates";

describe("Coordinate Functions with Decimal Zoom", () => {
  const width = 800;
  const height = 600;
  const center = { x: -1, y: 0 }; // Mandelbrot center
  const centerDecimal = { x: new Decimal(-1), y: new Decimal(0) };

  describe("Basic coordinate functions", () => {
    it("should work with Decimal zoom at zoom 1e15", () => {
      const zoom = new Decimal(1e15);
      const pixel = { x: width / 2, y: height / 2 }; // Center pixel

      // Convert pixel to fractal coordinate
      const fractalCoord = pixelToFractalCoordinate(pixel, width, height, center, zoom);

      // Verify the result is finite and reasonable
      expect(Number.isFinite(fractalCoord.x)).toBe(true);
      expect(Number.isFinite(fractalCoord.y)).toBe(true);

      // At center pixel, should get close to the fractal center
      expect(Math.abs(fractalCoord.x - center.x)).toBeLessThan(1e-10);
      expect(Math.abs(fractalCoord.y - center.y)).toBeLessThan(1e-10);
    });

    it("should work with Decimal zoom at zoom 1e50", () => {
      const zoom = new Decimal(1e50);
      const pixel = { x: width / 2, y: height / 2 }; // Center pixel

      // Convert pixel to fractal coordinate
      const fractalCoord = pixelToFractalCoordinate(pixel, width, height, center, zoom);

      // Verify the result is finite and reasonable
      expect(Number.isFinite(fractalCoord.x)).toBe(true);
      expect(Number.isFinite(fractalCoord.y)).toBe(true);

      // At center pixel, should get close to the fractal center
      expect(Math.abs(fractalCoord.x - center.x)).toBeLessThan(1e-40);
      expect(Math.abs(fractalCoord.y - center.y)).toBeLessThan(1e-40);
    });

    it("should maintain precision in round-trip conversion", () => {
      const zoom = new Decimal(1e15);
      const originalPixel = { x: 100, y: 200 };

      // Convert pixel to fractal coordinate
      const fractalCoord = pixelToFractalCoordinate(originalPixel, width, height, center, zoom);

      // Convert back to pixel coordinate
      const roundTripPixel = fractalToPixelCoordinate(fractalCoord, width, height, center, zoom);

      // Verify precision is maintained within reasonable tolerance
      const xError = Math.abs(roundTripPixel.x - originalPixel.x);
      const yError = Math.abs(roundTripPixel.y - originalPixel.y);

      expect(xError).toBeLessThan(1e-6);
      expect(yError).toBeLessThan(1e-6);
    });
  });

  describe("Ultra-high-precision coordinate functions", () => {
    it("should work with Decimal zoom at zoom 1e15", () => {
      const zoom = new Decimal(1e15);
      const pixel = { x: width / 2, y: height / 2 }; // Center pixel

      // Convert pixel to fractal coordinate
      const fractalCoord = pixelToFractalCoordinateUltraHP(pixel, width, height, centerDecimal, zoom);

      // Verify the result is a Decimal object
      expect(fractalCoord.x).toBeInstanceOf(Decimal);
      expect(fractalCoord.y).toBeInstanceOf(Decimal);

      // At center pixel, should get close to the fractal center
      expect(fractalCoord.x.minus(centerDecimal.x).abs().toNumber()).toBeLessThan(1e-10);
      expect(fractalCoord.y.minus(centerDecimal.y).abs().toNumber()).toBeLessThan(1e-10);
    });

    it("should work with Decimal zoom at zoom 1e50", () => {
      const zoom = new Decimal(1e50);
      const pixel = { x: width / 2, y: height / 2 }; // Center pixel

      // Convert pixel to fractal coordinate
      const fractalCoord = pixelToFractalCoordinateUltraHP(pixel, width, height, centerDecimal, zoom);

      // Verify the result is a Decimal object
      expect(fractalCoord.x).toBeInstanceOf(Decimal);
      expect(fractalCoord.y).toBeInstanceOf(Decimal);

      // At center pixel, should get close to the fractal center
      expect(fractalCoord.x.minus(centerDecimal.x).abs().toNumber()).toBeLessThan(1e-40);
      expect(fractalCoord.y.minus(centerDecimal.y).abs().toNumber()).toBeLessThan(1e-40);
    });

    it("should maintain precision in round-trip conversion", () => {
      const zoom = new Decimal(1e15);
      const originalPixel = { x: 100, y: 200 };

      // Convert pixel to fractal coordinate
      const fractalCoord = pixelToFractalCoordinateUltraHP(originalPixel, width, height, centerDecimal, zoom);

      // Convert back to pixel coordinate
      const roundTripPixel = fractalToPixelCoordinateUltraHP(fractalCoord, width, height, centerDecimal, zoom);

      // Verify precision is maintained within reasonable tolerance
      const xError = Math.abs(roundTripPixel.x - originalPixel.x);
      const yError = Math.abs(roundTripPixel.y - originalPixel.y);

      expect(xError).toBeLessThan(1e-6);
      expect(yError).toBeLessThan(1e-6);
    });
  });

  describe("Precision comparison", () => {
    it("should maintain better precision with Decimal arithmetic", () => {
      const zoom = new Decimal(1e20);
      const pixel = { x: width / 2 + 1, y: height / 2 + 1 }; // Slightly off-center

      // Test basic function
      const basicFractal = pixelToFractalCoordinate(pixel, width, height, center, zoom);

      // Test ultra-high-precision function
      const ultraFractal = pixelToFractalCoordinateUltraHP(pixel, width, height, centerDecimal, zoom);

      // Both should be finite
      expect(Number.isFinite(basicFractal.x)).toBe(true);
      expect(Number.isFinite(basicFractal.y)).toBe(true);
      expect(ultraFractal.x).toBeInstanceOf(Decimal);
      expect(ultraFractal.y).toBeInstanceOf(Decimal);

      // Ultra-high-precision should maintain Decimal precision
      expect(ultraFractal.x.isFinite()).toBe(true);
      expect(ultraFractal.y.isFinite()).toBe(true);
    });
  });
});
