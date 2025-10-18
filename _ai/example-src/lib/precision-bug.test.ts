import { describe, expect, it } from "vitest";
import { fractalToPixelCoordinateUltraHP, pixelToFractalCoordinateUltraHP } from "./coordinates";

describe("Precision Loss Bug at Extreme Zoom", () => {
  it("should preserve 1 pixel movement precision at zoom 6.5×10^15", () => {
    const zoom = 6.5e15;
    const center = { x: -0.5, y: 0 };
    const canvasWidth = 800;
    const canvasHeight = 600;

    // Test 1: Center pixel
    const centerPixel = { x: 400, y: 300 };
    const centerFractal = pixelToFractalCoordinateUltraHP(centerPixel, canvasWidth, canvasHeight, center, zoom);

    // Test 2: 1 pixel right - THIS IS WHERE THE BUG OCCURS
    const rightPixel = { x: 401, y: 300 };
    const rightFractal = pixelToFractalCoordinateUltraHP(rightPixel, canvasWidth, canvasHeight, center, zoom);

    // THE BUG: These should be DIFFERENT but they're the same!
    console.log("Center fractal:", centerFractal);
    console.log("Right fractal:", rightFractal);
    console.log("Are they the same?", centerFractal.x === rightFractal.x && centerFractal.y === rightFractal.y);

    // This test SHOULD FAIL with the current implementation
    expect(centerFractal.x).not.toBe(rightFractal.x);
    expect(centerFractal.y).not.toBe(rightFractal.y);
  });

  it("should preserve round-trip precision for 1 pixel movement at zoom 6.5×10^15", () => {
    const zoom = 6.5e15;
    const center = { x: -0.5, y: 0 };
    const canvasWidth = 800;
    const canvasHeight = 600;

    const rightPixel = { x: 401, y: 300 };
    const fractal = pixelToFractalCoordinateUltraHP(rightPixel, canvasWidth, canvasHeight, center, zoom);

    const backToPixel = fractalToPixelCoordinateUltraHP(fractal, canvasWidth, canvasHeight, center, zoom);

    // Round-trip should be exact
    expect(Math.abs(backToPixel.x - rightPixel.x)).toBeLessThan(0.1);
    expect(Math.abs(backToPixel.y - rightPixel.y)).toBeLessThan(0.1);
  });

  it("should detect precision loss in coordinate conversion chain", () => {
    const zoom = 6.5e15;
    const center = { x: -0.5, y: 0 };
    const canvasWidth = 800;
    const canvasHeight = 600;

    // Test multiple 1-pixel movements
    const testPixels = [
      { x: 400, y: 300 }, // Center
      { x: 401, y: 300 }, // 1 right
      { x: 400, y: 301 }, // 1 down
      { x: 399, y: 300 }, // 1 left
    ];

    const fractals = testPixels.map((pixel) =>
      pixelToFractalCoordinateUltraHP(pixel, canvasWidth, canvasHeight, center, zoom)
    );

    // All fractal coordinates should be different
    for (let i = 0; i < fractals.length; i++) {
      for (let j = i + 1; j < fractals.length; j++) {
        const same = fractals[i].x === fractals[j].x && fractals[i].y === fractals[j].y;
        if (same) {
          console.log(`BUG: Pixels ${i} and ${j} have same fractal coordinates!`);
          console.log(`Pixel ${i}:`, testPixels[i]);
          console.log(`Pixel ${j}:`, testPixels[j]);
          console.log(`Fractal ${i}:`, fractals[i]);
          console.log(`Fractal ${j}:`, fractals[j]);
        }
        expect(same).toBe(false);
      }
    }
  });
});
