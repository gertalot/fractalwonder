// ABOUTME: Test cases for precision in preview pixel computation at extreme zoom levels
import { FractalParams } from "@/hooks/use-store";
import { describe, expect, it } from "vitest";
import { fractalToPixelCoordinateHP, pixelToFractalCoordinateHP } from "./coordinates";
import { computePreviewPixelPosition } from "./render-preview";

describe("computePreviewPixelPosition", () => {
  it("should maintain precision at extreme zoom levels", () => {
    // Test case: Tiny fractal center change at zoom level 10^15
    const lastParams: FractalParams = {
      center: { x: -1, y: 0 },
      zoom: 1e15,
      maxIterations: 1000,
      iterationScalingFactor: 1000,
    };

    const newParams: FractalParams = {
      center: { x: -1.000000000000001, y: 0 }, // Tiny fractal center change
      zoom: 1e15,
      maxIterations: 1000,
      iterationScalingFactor: 1000,
    };

    const canvasWidth = 800;
    const canvasHeight = 600;

    const result = computePreviewPixelPosition(
      lastParams,
      newParams,
      canvasWidth,
      canvasHeight
    );

    // The result should be finite and within reasonable bounds
    // At zoom 10^15, a tiny fractal center change should result in some pixel movement
    expect(Number.isFinite(result.x)).toBe(true);
    expect(Number.isFinite(result.y)).toBe(true);
    expect(result.x).toBeGreaterThan(-canvasWidth); // Allow for negative values (preview can be off-screen)
    expect(result.y).toBeGreaterThan(-canvasHeight);
    expect(result.x).toBeLessThan(canvasWidth * 2); // Allow for scaling
    expect(result.y).toBeLessThan(canvasHeight * 2);
  });

  it("should handle smooth pixel transitions at extreme zoom", () => {
    // Test case: Multiple small fractal center changes should result in smooth pixel changes
    const baseParams: FractalParams = {
      center: { x: -1, y: 0 },
      zoom: 1e20,
      maxIterations: 1000,
      iterationScalingFactor: 1000,
    };

    const canvasWidth = 800;
    const canvasHeight = 600;

    const results = [];
    for (let i = 0; i < 10; i++) {
      const newParams: FractalParams = {
        ...baseParams,
        center: { 
          x: -1 + i * 1e-20, // Very small incremental changes
          y: 0 
        },
      };

      const result = computePreviewPixelPosition(
        baseParams,
        newParams,
        canvasWidth,
        canvasHeight
      );
      results.push(result.x);
    }

    // Check that pixel values change smoothly (no jumps)
    for (let i = 1; i < results.length; i++) {
      const diff = Math.abs(results[i] - results[i - 1]);
      expect(diff).toBeLessThan(100); // Should not jump by more than 100 pixels
    }
  });

  it("should handle zoom level 10^100", () => {
    // Test case: Extreme zoom level that requires maximum precision
    const lastParams: FractalParams = {
      center: { x: -1, y: 0 },
      zoom: 1e100,
      maxIterations: 1000,
      iterationScalingFactor: 1000,
    };

    const newParams: FractalParams = {
      center: { x: -1.0000000000000000000000000000001, y: 0 }, // Extremely tiny change
      zoom: 1e100,
      maxIterations: 1000,
      iterationScalingFactor: 1000,
    };

    const canvasWidth = 800;
    const canvasHeight = 600;

    const result = computePreviewPixelPosition(
      lastParams,
      newParams,
      canvasWidth,
      canvasHeight
    );

    // Should not throw errors and should return reasonable values
    expect(Number.isFinite(result.x)).toBe(true);
    expect(Number.isFinite(result.y)).toBe(true);
    // Preview position can be negative (off-screen) or positive, both are valid
    expect(result.x).toBeGreaterThan(-canvasWidth);
    expect(result.y).toBeGreaterThan(-canvasHeight);
  });
});

describe("Pixel ↔ Fractal Coordinate Round-trip Precision", () => {
  it("should maintain precision in pixel → fractal → pixel conversion at zoom 10^15", () => {
    const center = { x: -1, y: 0 };
    const zoom = 1e15;
    const width = 800;
    const height = 600;

    // Test multiple pixel positions
    const testPixels = [
      { x: 0, y: 0 },           // Top-left
      { x: 400, y: 300 },       // Center
      { x: 800, y: 600 },       // Bottom-right
      { x: 100, y: 200 },       // Random position
      { x: 700, y: 500 },       // Another random position
    ];

    for (const originalPixel of testPixels) {
      // Convert pixel → fractal using high-precision functions
      const fractalCoord = pixelToFractalCoordinateHP(
        originalPixel,
        width,
        height,
        center,
        zoom
      );

      // Convert fractal → pixel using high-precision functions
      const roundTripPixel = fractalToPixelCoordinateHP(
        fractalCoord,
        width,
        height,
        center,
        zoom
      );

      // The round-trip should be very close to the original pixel
      // At zoom 10^15, we expect some precision loss, but it should be minimal
      const xError = Math.abs(roundTripPixel.x - originalPixel.x);
      const yError = Math.abs(roundTripPixel.y - originalPixel.y);

      // Allow for small precision loss but not major jumps
      // High-precision functions should maintain much better precision than standard functions
      expect(xError).toBeLessThan(10); // Less than 10 pixel error (much better than broken functions)
      expect(yError).toBeLessThan(10); // Less than 10 pixel error (much better than broken functions)
    }
  });

  it("should maintain precision in pixel → fractal → pixel conversion at zoom 10^50", () => {
    const center = { x: -1, y: 0 };
    const zoom = 1e50;
    const width = 800;
    const height = 600;

    // Test center pixel (most critical for dragging)
    const originalPixel = { x: 400, y: 300 };
    
    const fractalCoord = pixelToFractalCoordinateHP(
      originalPixel,
      width,
      height,
      center,
      zoom
    );

    const roundTripPixel = fractalToPixelCoordinateHP(
      fractalCoord,
      width,
      height,
      center,
      zoom
    );

    const xError = Math.abs(roundTripPixel.x - originalPixel.x);
    const yError = Math.abs(roundTripPixel.y - originalPixel.y);

    // At zoom 10^50, we expect more precision loss, but still manageable
    expect(xError).toBeLessThan(1); // Less than 1 pixel error
    expect(yError).toBeLessThan(1); // Less than 1 pixel error
  });

  it("should maintain precision in pixel → fractal → pixel conversion at zoom 10^100", () => {
    const center = { x: -1, y: 0 };
    const zoom = 1e100;
    const width = 800;
    const height = 600;

    // Test center pixel
    const originalPixel = { x: 400, y: 300 };
    
    const fractalCoord = pixelToFractalCoordinateHP(
      originalPixel,
      width,
      height,
      center,
      zoom
    );

    const roundTripPixel = fractalToPixelCoordinateHP(
      fractalCoord,
      width,
      height,
      center,
      zoom
    );

    const xError = Math.abs(roundTripPixel.x - originalPixel.x);
    const yError = Math.abs(roundTripPixel.y - originalPixel.y);

    // At zoom 10^100, we expect significant precision loss, but should still be reasonable
    expect(xError).toBeLessThan(10); // Less than 10 pixel error
    expect(yError).toBeLessThan(10); // Less than 10 pixel error
  });

  it("should maintain smooth transitions during small fractal center changes", () => {
    // This test simulates the exact scenario that causes "jumpy" preview behavior
    const baseCenter = { x: -1, y: 0 };
    const zoom = 1e20;
    const width = 800;
    const height = 600;

    const results = [];
    
    // Simulate small incremental fractal center changes (like during dragging)
    for (let i = 0; i < 20; i++) {
      const center = {
        x: baseCenter.x + i * 1e-25, // Very small incremental changes
        y: baseCenter.y
      };

      // Convert center pixel to fractal and back
      const centerPixel = { x: 400, y: 300 };
      const fractalCoord = pixelToFractalCoordinateHP(
        centerPixel,
        width,
        height,
        center,
        zoom
      );
      const roundTripPixel = fractalToPixelCoordinateHP(
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

  it("should handle extreme precision requirements for dragging at zoom 10^100", () => {
    // This test specifically targets the precision loss that causes "jumpy" dragging
    const center = { x: -1, y: 0 };
    const zoom = 1e100;
    const width = 800;
    const height = 600;

    // Test a 1-pixel drag at extreme zoom
    const startPixel = { x: 400, y: 300 };
    const endPixel = { x: 401, y: 300 }; // 1 pixel to the right

    // Convert both pixels to fractal coordinates
    const startFractal = pixelToFractalCoordinateHP(
      startPixel,
      width,
      height,
      center,
      zoom
    );
    const endFractal = pixelToFractalCoordinateHP(
      endPixel,
      width,
      height,
      center,
      zoom
    );

    // Calculate the fractal center change needed for this 1-pixel drag
    const fractalCenterChange = {
      x: endFractal.x - startFractal.x,
      y: endFractal.y - startFractal.y
    };

    // Apply this center change and verify the pixel position
    const newCenter = {
      x: center.x + fractalCenterChange.x,
      y: center.y + fractalCenterChange.y
    };

    const newPixel = fractalToPixelCoordinateHP(
      startFractal, // Use the original fractal coordinate
      width,
      height,
      newCenter, // With the new center
      zoom
    );

    // The new pixel should be very close to the target end pixel
    const xError = Math.abs(newPixel.x - endPixel.x);
    const yError = Math.abs(newPixel.y - endPixel.y);

    // At zoom 10^100, even small errors can cause noticeable jumps
    // High-precision functions should maintain much better precision than standard functions
    expect(xError).toBeLessThan(10);
    expect(yError).toBeLessThan(10);
  });
});
