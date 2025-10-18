import { FractalParams } from "@/hooks/use-store";
import { describe, expect, it } from "vitest";
import { computePreviewPixelPosition } from "./render-preview";

describe("Complete Drag Behavior at Extreme Zoom", () => {
  it("should move preview smoothly when dragging 1 pixel at zoom 6.5×10^15", () => {
    const zoom = 6.5e15;
    const canvasWidth = 800;
    const canvasHeight = 600;

    // Simulate the COMPLETE drag interaction:
    // 1. User starts at center, drags 1 pixel right
    const lastParams: FractalParams = {
      center: { x: -0.5, y: 0 },
      zoom: zoom,
      maxIterations: 1000,
      iterationScalingFactor: 1000,
    };

    // 2. User drags 1 pixel right - this should cause a tiny fractal center change
    // The new center should be calculated as: newCenter = oldCenter + (pixelOffset * scale)
    const pixelOffset = 1; // 1 pixel right
    const scale = 4 / canvasHeight / zoom; // Same scale used in coordinate conversion
    const fractalOffset = pixelOffset * scale;

    const newParams: FractalParams = {
      center: {
        x: -0.5 + fractalOffset, // This is what the interaction logic should calculate
        y: 0,
      },
      zoom: zoom,
      maxIterations: 1000,
      iterationScalingFactor: 1000,
    };

    console.log("Last params center:", lastParams.center);
    console.log("New params center:", newParams.center);
    console.log("Fractal offset:", fractalOffset);
    console.log("Scale factor:", scale);

    // 3. Calculate where the preview should be positioned
    const previewPosition = computePreviewPixelPosition(lastParams, newParams, canvasWidth, canvasHeight);

    console.log("Preview position:", previewPosition);

    // 4. The preview should move smoothly (small pixel movement)
    // If it jumps, the preview position will be far from the expected position
    const expectedPreviewX = 0; // Top-left of last view should map to top-left of new view
    const expectedPreviewY = 0;

    const previewError = {
      x: Math.abs(previewPosition.x - expectedPreviewX),
      y: Math.abs(previewPosition.y - expectedPreviewY),
    };

    console.log("Preview error:", previewError);
    console.log(
      "Expected smooth movement, got jump of:",
      Math.sqrt(previewError.x ** 2 + previewError.y ** 2),
      "pixels"
    );

    // The preview should move smoothly (error < 10 pixels for 1 pixel drag)
    expect(previewError.x).toBeLessThan(10);
    expect(previewError.y).toBeLessThan(10);
  });

  it("should detect jumping behavior in preview positioning", () => {
    const zoom = 6.5e15;
    const canvasWidth = 800;
    const canvasHeight = 600;

    // Test multiple small drag movements
    const baseParams: FractalParams = {
      center: { x: -0.5, y: 0 },
      zoom: zoom,
      maxIterations: 1000,
      iterationScalingFactor: 1000,
    };

    const dragMovements = [1, 2, 3, 4, 5]; // 1-5 pixel drags
    const scale = 4 / canvasHeight / zoom;

    const previewPositions = dragMovements.map((pixelDrag) => {
      const fractalOffset = pixelDrag * scale;
      const newParams: FractalParams = {
        ...baseParams,
        center: {
          x: baseParams.center.x + fractalOffset,
          y: baseParams.center.y,
        },
      };

      return computePreviewPixelPosition(baseParams, newParams, canvasWidth, canvasHeight);
    });

    console.log("Drag movements:", dragMovements);
    console.log("Preview positions:", previewPositions);

    // Check for jumping: consecutive movements should be smooth
    for (let i = 1; i < previewPositions.length; i++) {
      const prevPos = previewPositions[i - 1];
      const currPos = previewPositions[i];
      const movement = Math.sqrt((currPos.x - prevPos.x) ** 2 + (currPos.y - prevPos.y) ** 2);

      console.log(`Movement from ${i - 1} to ${i}:`, movement, "pixels");

      // Each 1-pixel drag should result in small preview movement
      // If it jumps, movement will be large
      expect(movement).toBeLessThan(50); // Allow some tolerance but not huge jumps
    }
  });
});
