import { describe, it, expect } from "vitest";
import { pixelToFractalCoordinate, fractalToPixelCoordinate } from "./coordinates";

describe("pixelToFractalCoordinate", () => {
  it("should find the right corner points and center for a square pixel grid", () => {
    const width = 100;
    const height = 100;

    const tl = pixelToFractalCoordinate({ x: 0, y: 0 }, width, height, { x: 0, y: 0 }, 1);
    const tr = pixelToFractalCoordinate({ x: 100, y: 0 }, width, height, { x: 0, y: 0 }, 1);
    const bl = pixelToFractalCoordinate({ x: 0, y: 100 }, width, height, { x: 0, y: 0 }, 1);
    const br = pixelToFractalCoordinate({ x: 100, y: 100 }, width, height, { x: 0, y: 0 }, 1);
    const cr = pixelToFractalCoordinate({ x: 50, y: 50 }, width, height, { x: 0, y: 0 }, 1);

    expect(tl.x).toBeCloseTo(-2, 5);
    expect(tl.y).toBeCloseTo(-2, 5);

    expect(tr.x).toBeCloseTo(2, 5);
    expect(tr.y).toBeCloseTo(-2, 5);

    expect(bl.x).toBeCloseTo(-2, 5);
    expect(bl.y).toBeCloseTo(2, 5);

    expect(br.x).toBeCloseTo(2, 5);
    expect(br.y).toBeCloseTo(2, 5);

    expect(cr.x).toBeCloseTo(0, 5);
    expect(cr.y).toBeCloseTo(0, 5);
  });

  it("should find the right corner points and center for a rectangular pixel grid", () => {
    const width = 800;
    const height = 400;

    const tl = pixelToFractalCoordinate({ x: 200, y: 0 }, width, height, { x: 0, y: 0 }, 1);
    const tr = pixelToFractalCoordinate({ x: 600, y: 0 }, width, height, { x: 0, y: 0 }, 1);
    const bl = pixelToFractalCoordinate({ x: 200, y: 400 }, width, height, { x: 0, y: 0 }, 1);
    const br = pixelToFractalCoordinate({ x: 600, y: 400 }, width, height, { x: 0, y: 0 }, 1);
    const cr = pixelToFractalCoordinate({ x: 400, y: 200 }, width, height, { x: 0, y: 0 }, 1);

    expect(tl.x).toBeCloseTo(-2, 5);
    expect(tl.y).toBeCloseTo(-2, 5);

    expect(tr.x).toBeCloseTo(2, 5);
    expect(tr.y).toBeCloseTo(-2, 5);

    expect(bl.x).toBeCloseTo(-2, 5);
    expect(bl.y).toBeCloseTo(2, 5);

    expect(br.x).toBeCloseTo(2, 5);
    expect(br.y).toBeCloseTo(2, 5);

    expect(cr.x).toBeCloseTo(0, 5);
    expect(cr.y).toBeCloseTo(0, 5);
  });

  it("should find the right corner points and center for a rectangular pixel grid with zoom=2", () => {
    const width = 800;
    const height = 400;

    const tl = pixelToFractalCoordinate({ x: 200, y: 0 }, width, height, { x: 0, y: 0 }, 2);
    const tr = pixelToFractalCoordinate({ x: 600, y: 0 }, width, height, { x: 0, y: 0 }, 2);
    const bl = pixelToFractalCoordinate({ x: 200, y: 400 }, width, height, { x: 0, y: 0 }, 2);
    const br = pixelToFractalCoordinate({ x: 600, y: 400 }, width, height, { x: 0, y: 0 }, 2);
    const cr = pixelToFractalCoordinate({ x: 400, y: 200 }, width, height, { x: 0, y: 0 }, 2);

    expect(tl.x).toBeCloseTo(-1, 5);
    expect(tl.y).toBeCloseTo(-1, 5);

    expect(tr.x).toBeCloseTo(1, 5);
    expect(tr.y).toBeCloseTo(-1, 5);

    expect(bl.x).toBeCloseTo(-1, 5);
    expect(bl.y).toBeCloseTo(1, 5);

    expect(br.x).toBeCloseTo(1, 5);
    expect(br.y).toBeCloseTo(1, 5);

    expect(cr.x).toBeCloseTo(0, 5);
    expect(cr.y).toBeCloseTo(0, 5);
  });
});

describe("fractalToPixelCoordinate", () => {
  it("should find the right corner points and center for a square pixel grid", () => {
    const width = 100;
    const height = 100;

    const tl = fractalToPixelCoordinate({ x: -2, y: -2 }, width, height, { x: 0, y: 0 }, 1);
    const tr = fractalToPixelCoordinate({ x: 2, y: -2 }, width, height, { x: 0, y: 0 }, 1);
    const bl = fractalToPixelCoordinate({ x: -2, y: 2 }, width, height, { x: 0, y: 0 }, 1);
    const br = fractalToPixelCoordinate({ x: 2, y: 2 }, width, height, { x: 0, y: 0 }, 1);
    const cr = fractalToPixelCoordinate({ x: 0, y: 0 }, width, height, { x: 0, y: 0 }, 1);

    expect(tl.x).toBeCloseTo(0, 5);
    expect(tl.y).toBeCloseTo(0, 5);

    expect(tr.x).toBeCloseTo(100, 5);
    expect(tr.y).toBeCloseTo(0, 5);

    expect(bl.x).toBeCloseTo(0, 5);
    expect(bl.y).toBeCloseTo(100, 5);

    expect(br.x).toBeCloseTo(100, 5);
    expect(br.y).toBeCloseTo(100, 5);

    expect(cr.x).toBeCloseTo(50, 5);
    expect(cr.y).toBeCloseTo(50, 5);
  });

  it("should find the right corner points and center for a rectangular pixel grid", () => {
    const width = 800;
    const height = 400;

    const tl = fractalToPixelCoordinate({ x: -2, y: -2 }, width, height, { x: 0, y: 0 }, 1);
    const tr = fractalToPixelCoordinate({ x: 2, y: -2 }, width, height, { x: 0, y: 0 }, 1);
    const bl = fractalToPixelCoordinate({ x: -2, y: 2 }, width, height, { x: 0, y: 0 }, 1);
    const br = fractalToPixelCoordinate({ x: 2, y: 2 }, width, height, { x: 0, y: 0 }, 1);
    const cr = fractalToPixelCoordinate({ x: 0, y: 0 }, width, height, { x: 0, y: 0 }, 1);

    expect(tl.x).toBeCloseTo(200, 5);
    expect(tl.y).toBeCloseTo(0, 5);

    expect(tr.x).toBeCloseTo(600, 5);
    expect(tr.y).toBeCloseTo(0, 5);

    expect(bl.x).toBeCloseTo(200, 5);
    expect(bl.y).toBeCloseTo(400, 5);

    expect(br.x).toBeCloseTo(600, 5);
    expect(br.y).toBeCloseTo(400, 5);

    expect(cr.x).toBeCloseTo(400, 5);
    expect(cr.y).toBeCloseTo(200, 5);
  });

  it("should find the right corner points and center for a rectangular pixel grid with zoom=2", () => {
    const width = 800;
    const height = 400;

    const tl = fractalToPixelCoordinate({ x: -1, y: -1 }, width, height, { x: 0, y: 0 }, 2);
    const tr = fractalToPixelCoordinate({ x: 1, y: -1 }, width, height, { x: 0, y: 0 }, 2);
    const bl = fractalToPixelCoordinate({ x: -1, y: 1 }, width, height, { x: 0, y: 0 }, 2);
    const br = fractalToPixelCoordinate({ x: 1, y: 1 }, width, height, { x: 0, y: 0 }, 2);
    const cr = fractalToPixelCoordinate({ x: 0, y: 0 }, width, height, { x: 0, y: 0 }, 2);

    expect(tl.x).toBeCloseTo(200, 5);
    expect(tl.y).toBeCloseTo(0, 5);

    expect(tr.x).toBeCloseTo(600, 5);
    expect(tr.y).toBeCloseTo(0, 5);

    expect(bl.x).toBeCloseTo(200, 5);
    expect(bl.y).toBeCloseTo(400, 5);

    expect(br.x).toBeCloseTo(600, 5);
    expect(br.y).toBeCloseTo(400, 5);

    expect(cr.x).toBeCloseTo(400, 5);
    expect(cr.y).toBeCloseTo(200, 5);
  });
});
