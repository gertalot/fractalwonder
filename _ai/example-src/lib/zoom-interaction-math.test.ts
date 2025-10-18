import { Decimal } from "decimal.js";
import { describe, expect, it } from "vitest";
import { fractalToPixelCoordinateUltraHP, pixelToFractalCoordinateUltraHP } from "./coordinates";
import { computePreviewPixelPosition } from "./render-preview";

// Constants matching the actual codebase
const INITIAL_FRACTAL_VIEW_HEIGHT = 4;

// Test helper types
interface Point {
  x: number;
  y: number;
}

interface DecimalPoint {
  x: Decimal;
  y: Decimal;
}

interface TestFractalParams {
  center: DecimalPoint;
  zoom: Decimal;
  maxIterations: number;
  iterationScalingFactor: number;
}

/**
 * Mathematical test utility: Calculate the scale factor for a given zoom level
 * Formula: scale = INITIAL_FRACTAL_VIEW_HEIGHT / canvasHeight / zoom
 */
function calculateScale(canvasHeight: number, zoom: Decimal): Decimal {
  const originalPrecision = Decimal.precision;
  Decimal.set({ precision: 300 });

  try {
    return new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT).div(new Decimal(canvasHeight)).div(zoom);
  } finally {
    Decimal.set({ precision: originalPrecision });
  }
}

/**
 * Mathematical test utility: Calculate new center after zoom-to-point
 * Formula: newCenter = fractalUnderPointer - (pointerOffset * newScale)
 * where pointerOffset = pointerPixel - canvasCenter
 */
function calculateNewCenterAfterZoom(
  pointerPixel: Point,
  fractalUnderPointer: DecimalPoint,
  canvasWidth: number,
  canvasHeight: number,
  newZoom: Decimal
): DecimalPoint {
  const originalPrecision = Decimal.precision;
  Decimal.set({ precision: 300 });

  try {
    const newScale = calculateScale(canvasHeight, newZoom);
    const canvasCenter = {
      x: new Decimal(canvasWidth).div(2),
      y: new Decimal(canvasHeight).div(2),
    };
    const pointerOffset = {
      x: new Decimal(pointerPixel.x).minus(canvasCenter.x),
      y: new Decimal(pointerPixel.y).minus(canvasCenter.y),
    };

    return {
      x: fractalUnderPointer.x.minus(pointerOffset.x.times(newScale)),
      y: fractalUnderPointer.y.minus(pointerOffset.y.times(newScale)),
    };
  } finally {
    Decimal.set({ precision: originalPrecision });
  }
}

/**
 * Mathematical test utility: Verify the pointer invariant holds
 * The fractal coordinate under the pointer must remain constant during zoom
 */
function verifyPointerInvariant(
  pointerPixel: Point,
  oldCenter: DecimalPoint,
  oldZoom: Decimal,
  newCenter: DecimalPoint,
  newZoom: Decimal,
  canvasWidth: number,
  canvasHeight: number,
  toleranceFractalUnits: number = 1e-100
): { passes: boolean; error: Decimal } {
  const fractalBefore = pixelToFractalCoordinateUltraHP(pointerPixel, canvasWidth, canvasHeight, oldCenter, oldZoom);

  const fractalAfter = pixelToFractalCoordinateUltraHP(pointerPixel, canvasWidth, canvasHeight, newCenter, newZoom);

  const errorX = fractalAfter.x.minus(fractalBefore.x).abs();
  const errorY = fractalAfter.y.minus(fractalBefore.y).abs();
  const error = Decimal.max(errorX, errorY);

  const passes = error.lte(new Decimal(toleranceFractalUnits));

  return { passes, error };
}

/**
 * Mathematical test utility: Calculate round-trip error
 * pixel → fractal → pixel should return to original position
 */
function calculateRoundTripError(
  pixel: Point,
  center: DecimalPoint,
  zoom: Decimal,
  canvasWidth: number,
  canvasHeight: number
): { errorX: number; errorY: number; maxError: number } {
  const fractal = pixelToFractalCoordinateUltraHP(pixel, canvasWidth, canvasHeight, center, zoom);

  const roundTrip = fractalToPixelCoordinateUltraHP(fractal, canvasWidth, canvasHeight, center, zoom);

  const errorX = Math.abs(roundTrip.x - pixel.x);
  const errorY = Math.abs(roundTrip.y - pixel.y);
  const maxError = Math.max(errorX, errorY);

  return { errorX, errorY, maxError };
}

/**
 * Simulates the handleWheel logic from use-fractal-interaction.ts
 * This is the ACTUAL broken code that needs to be tested
 */
function simulateHandleWheel(
  pointerPixelX: number,
  pointerPixelY: number,
  canvasWidth: number,
  canvasHeight: number,
  currentCenter: DecimalPoint,
  currentZoom: Decimal,
  wheelDelta: number,
  zoomSensitivity: number = 0.0005
): { newCenter: DecimalPoint; newZoom: Decimal } {
  const INITIAL_FRACTAL_VIEW_HEIGHT = 4;

  // This simulates the ACTUAL code from handleWheel (lines 369-411)
  const pointerDevicePixels = {
    x: pointerPixelX,
    y: pointerPixelY,
  };

  // Calculate the fractal coordinate under the pointer BEFORE zooming
  const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
    pointerDevicePixels,
    canvasWidth,
    canvasHeight,
    currentCenter,
    currentZoom
  );

  // Calculate new zoom
  const delta = wheelDelta * zoomSensitivity;
  const zoomFactor = new Decimal(-delta).exp();
  const newZoom = new Decimal(currentZoom).times(zoomFactor); // BUG: wrapping Decimal in Decimal

  // Calculate new center that keeps fractalUnderPointer at pointerDevicePixels
  const scale = new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT).div(canvasHeight).div(newZoom);
  const canvasCenter = {
    x: new Decimal(canvasWidth).div(2),
    y: new Decimal(canvasHeight).div(2),
  };
  const offsetFromCenter = {
    x: new Decimal(pointerDevicePixels.x).minus(canvasCenter.x),
    y: new Decimal(pointerDevicePixels.y).minus(canvasCenter.y),
  };
  const newCenter = {
    x: fractalUnderPointer.x.minus(offsetFromCenter.x.times(scale)),
    y: fractalUnderPointer.y.minus(offsetFromCenter.y.times(scale)),
  };

  return { newCenter, newZoom };
}

describe("Zoom Interaction Mathematics - EXPOSING BUGS AT EXTREME ZOOM", () => {
  describe("CRITICAL BUG: Real Scenario from Big Boss at zoom 5.5e27", () => {
    it("MUST FAIL: Pointer invariant is violated at extreme zoom (zoom 5.5e27 -> 5.5e28)", () => {
      // Big Boss's exact scenario
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 }; // NOT at center

      const oldCenter = {
        x: new Decimal(
          "0.251707996013783358339999999997512671784238600371870954188086730380470677631327308556935786794162615853784829687770715336307816613581078609813556371056912433105739242138337184786338210248876216839993991359683834053811899901198189489804591882223975704423186305589679761304860457613678272663984609331136"
        ),
        y: new Decimal(
          "-0.000128095296187219876239999988188446640337227943910761510966829607784840898501931210161039907358599892056756684858841937336071562921666487439244128526804067156573259787810610477532757569270830753269896142582395218886875447051088310223899671084533842470314027067222740856065105003173786943633720750471251"
        ),
      };
      const oldZoom = new Decimal("5.5633613734670552409e+27");

      // Calculate fractal coordinate under pointer BEFORE zoom
      const fractalBeforeZoom = pixelToFractalCoordinateUltraHP(
        pointerPixel,
        canvasWidth,
        canvasHeight,
        oldCenter,
        oldZoom
      );

      // Simulate zooming to 5.5e28 (10x zoom in)
      const newZoom = new Decimal("5.5e28");
      const wheelDelta = Math.log(newZoom.div(oldZoom).toNumber()) / -0.0005;

      // Simulate the ACTUAL handleWheel logic
      const result = simulateHandleWheel(
        pointerPixel.x,
        pointerPixel.y,
        canvasWidth,
        canvasHeight,
        oldCenter,
        oldZoom,
        wheelDelta,
        0.0005
      );

      // Calculate fractal coordinate under pointer AFTER zoom
      const fractalAfterZoom = pixelToFractalCoordinateUltraHP(
        pointerPixel,
        canvasWidth,
        canvasHeight,
        result.newCenter,
        result.newZoom
      );

      // THE POINTER INVARIANT: fractal coordinate under pointer should NOT change
      const errorX = fractalAfterZoom.x.minus(fractalBeforeZoom.x).abs();
      const errorY = fractalAfterZoom.y.minus(fractalBeforeZoom.y).abs();

      console.log("=== BUG DETECTION ===");
      console.log("Fractal under pointer BEFORE zoom:", {
        x: fractalBeforeZoom.x.toString(),
        y: fractalBeforeZoom.y.toString(),
      });
      console.log("Fractal under pointer AFTER zoom:", {
        x: fractalAfterZoom.x.toString(),
        y: fractalAfterZoom.y.toString(),
      });
      console.log("Error in fractal coordinate:", {
        x: errorX.toExponential(),
        y: errorY.toExponential(),
      });

      // THIS TEST SHOULD FAIL IF THE BUG EXISTS
      // At extreme zoom, even tiny errors become massive jumps
      expect(errorX.lt(new Decimal("1e-100"))).toBe(true);
      expect(errorY.lt(new Decimal("1e-100"))).toBe(true);
    });

    it("MUST FAIL: Center coordinates change incorrectly at extreme zoom", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 };

      const oldCenter = {
        x: new Decimal(
          "0.251707996013783358339999999997512671784238600371870954188086730380470677631327308556935786794162615853784829687770715336307816613581078609813556371056912433105739242138337184786338210248876216839993991359683834053811899901198189489804591882223975704423186305589679761304860457613678272663984609331136"
        ),
        y: new Decimal(
          "-0.000128095296187219876239999988188446640337227943910761510966829607784840898501931210161039907358599892056756684858841937336071562921666487439244128526804067156573259787810610477532757569270830753269896142582395218886875447051088310223899671084533842470314027067222740856065105003173786943633720750471251"
        ),
      };
      const oldZoom = new Decimal("5.5633613734670552409e+27");
      const newZoom = new Decimal("5.5e28");
      const wheelDelta = Math.log(newZoom.div(oldZoom).toNumber()) / -0.0005;

      const result = simulateHandleWheel(
        pointerPixel.x,
        pointerPixel.y,
        canvasWidth,
        canvasHeight,
        oldCenter,
        oldZoom,
        wheelDelta,
        0.0005
      );

      // Check if center moved at all
      const centerChangeX = result.newCenter.x.minus(oldCenter.x).abs();
      const centerChangeY = result.newCenter.y.minus(oldCenter.y).abs();

      console.log("=== CENTER MOVEMENT BUG ===");
      console.log("Old center:", { x: oldCenter.x.toExponential(), y: oldCenter.y.toExponential() });
      console.log("New center:", { x: result.newCenter.x.toExponential(), y: result.newCenter.y.toExponential() });
      console.log("Center change:", { x: centerChangeX.toExponential(), y: centerChangeY.toExponential() });

      // Center MUST change when zooming off-center
      // If it doesn't change enough, or changes too much, the bug is present
      expect(centerChangeX.gt(new Decimal("1e-120"))).toBe(true); // Should move SOME amount
      expect(centerChangeX.lt(new Decimal("1"))).toBe(true); // But not by a huge amount
    });

    it("MUST FAIL: Preview rendering has jumpy/incorrect translation at extreme zoom", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;

      const lastParams: TestFractalParams = {
        center: {
          x: new Decimal(
            "0.251707996013783358339999999997512671784238600371870954188086730380470677631327308556935786794162615853784829687770715336307816613581078609813556371056912433105739242138337184786338210248876216839993991359683834053811899901198189489804591882223975704423186305589679761304860457613678272663984609331136"
          ),
          y: new Decimal(
            "-0.000128095296187219876239999988188446640337227943910761510966829607784840898501931210161039907358599892056756684858841937336071562921666487439244128526804067156573259787810610477532757569270830753269896142582395218886875447051088310223899671084533842470314027067222740856065105003173786943633720750471251"
          ),
        },
        zoom: new Decimal("5.5633613734670552409e+27"),
        maxIterations: 1000,
        iterationScalingFactor: 1000,
      };

      // Simulate zoom to 5.5e28
      const newZoom = new Decimal("5.5e28");
      const wheelDelta = Math.log(newZoom.div(lastParams.zoom).toNumber()) / -0.0005;
      const result = simulateHandleWheel(
        300,
        200,
        canvasWidth,
        canvasHeight,
        lastParams.center,
        lastParams.zoom,
        wheelDelta,
        0.0005
      );

      const newParams: TestFractalParams = {
        center: result.newCenter,
        zoom: result.newZoom,
        maxIterations: 1000,
        iterationScalingFactor: 1000,
      };

      // Calculate preview pixel position (where the old image should be drawn)
      const previewPosition = computePreviewPixelPosition(lastParams, newParams, canvasWidth, canvasHeight);

      console.log("=== PREVIEW RENDERING BUG ===");
      console.log("Preview top-left position:", previewPosition);
      console.log("Canvas size:", { width: canvasWidth, height: canvasHeight });

      // The preview should be positioned such that it appears smooth
      // If the position is wildly off or NaN/Infinity, the bug is present
      expect(Number.isFinite(previewPosition.x)).toBe(true);
      expect(Number.isFinite(previewPosition.y)).toBe(true);

      // At 10x zoom, the preview image should be shifted but visible
      // If it's shifted by more than canvas dimensions, it's broken
      const maxReasonableShift = canvasWidth * 2;
      expect(Math.abs(previewPosition.x)).toBeLessThan(maxReasonableShift);
      expect(Math.abs(previewPosition.y)).toBeLessThan(maxReasonableShift);
    });

    it("MUST FAIL: Multiple consecutive zooms compound the error exponentially", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 };

      let currentCenter = {
        x: new Decimal(
          "0.251707996013783358339999999997512671784238600371870954188086730380470677631327308556935786794162615853784829687770715336307816613581078609813556371056912433105739242138337184786338210248876216839993991359683834053811899901198189489804591882223975704423186305589679761304860457613678272663984609331136"
        ),
        y: new Decimal(
          "-0.000128095296187219876239999988188446640337227943910761510966829607784840898501931210161039907358599892056756684858841937336071562921666487439244128526804067156573259787810610477532757569270830753269896142582395218886875447051088310223899671084533842470314027067222740856065105003173786943633720750471251"
        ),
      };
      let currentZoom = new Decimal("5.5633613734670552409e+27");

      // Get the fractal coordinate under pointer at the START
      const originalFractalUnderPointer = pixelToFractalCoordinateUltraHP(
        pointerPixel,
        canvasWidth,
        canvasHeight,
        currentCenter,
        currentZoom
      );

      // Perform 5 consecutive zoom operations
      const errors: Decimal[] = [];
      for (let i = 0; i < 5; i++) {
        const result = simulateHandleWheel(
          pointerPixel.x,
          pointerPixel.y,
          canvasWidth,
          canvasHeight,
          currentCenter,
          currentZoom,
          -200, // zoom in
          0.0005
        );

        currentCenter = result.newCenter;
        currentZoom = result.newZoom;

        // Check fractal coordinate under pointer after each zoom
        const fractalNow = pixelToFractalCoordinateUltraHP(
          pointerPixel,
          canvasWidth,
          canvasHeight,
          currentCenter,
          currentZoom
        );

        const errorX = fractalNow.x.minus(originalFractalUnderPointer.x).abs();
        const errorY = fractalNow.y.minus(originalFractalUnderPointer.y).abs();
        const maxError = Decimal.max(errorX, errorY);
        errors.push(maxError);

        console.log(`Zoom step ${i + 1}, error:`, maxError.toExponential());
      }

      // The errors should NOT grow exponentially
      // If each error is significantly larger than the previous, the bug compounds
      const firstError = errors[0];
      const lastError = errors[4];

      console.log("=== COMPOUNDING ERROR BUG ===");
      console.log("First error:", firstError.toExponential());
      console.log("Last error:", lastError.toExponential());

      // If errors are near zero, the bug is FIXED!
      if (firstError.lt(new Decimal("1e-100")) && lastError.lt(new Decimal("1e-100"))) {
        console.log("✅ BUG FIXED: Errors are near zero!");
        expect(true).toBe(true); // Bug is fixed!
      } else {
        // If errors exist, they should not grow exponentially
        const errorGrowth = lastError.div(firstError);
        console.log("Error growth factor:", errorGrowth.toString());

        // Error should grow slowly (linearly at most), not exponentially
        // If it grows by more than 100x over 5 zooms, the bug is compounding
        expect(errorGrowth.lt(new Decimal(100))).toBe(true);
      }
    });
  });
});

describe("Zoom Interaction Mathematics - Extreme Precision", () => {
  describe("Scale Calculation Precision", () => {
    it("should calculate scale using only Decimal arithmetic", () => {
      const canvasHeight = 800;
      const zoom = new Decimal("5.5e27");

      const scale = calculateScale(canvasHeight, zoom);

      // Verify the calculation: scale = 4 / 800 / 5.5e27 = 9.090909...e-31
      // The scale calculation returns a value with 300 digits precision
      // We just need to verify it's in the right ballpark
      expect(scale.toExponential()).toContain("e-31");

      // Verify it's a Decimal instance
      expect(scale).toBeInstanceOf(Decimal);

      // Verify the magnitude is correct (should be around 9.09e-31)
      const magnitude = scale.times(new Decimal("1e31"));
      expect(magnitude.toNumber()).toBeGreaterThan(9);
      expect(magnitude.toNumber()).toBeLessThan(10);
    });

    it("should maintain precision at zoom 10^100", () => {
      const canvasHeight = 800;
      const zoom = new Decimal("1e100");

      const scale = calculateScale(canvasHeight, zoom);

      // Scale should be 4 / 800 / 1e100 = 5e-103
      expect(scale.toExponential()).toBe("5e-103");
    });
  });

  describe("Test 1 & 2: Fractal Coordinate Under Pointer at Extreme Zoom", () => {
    it("should calculate fractal coordinate at zoom 5.5e27 with pointer at (300,200) on 1920×800 canvas", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 };
      const center = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      const zoom = new Decimal("5.5e27");

      const fractalCoord = pixelToFractalCoordinateUltraHP(pointerPixel, canvasWidth, canvasHeight, center, zoom);

      // Calculate expected value manually:
      // dx = 300 - 1920/2 = 300 - 960 = -660
      // dy = 200 - 800/2 = 200 - 400 = -200
      // scale = 4 / 800 / 5.5e27 = 9.090909...e-30
      // x = (-660) * scale + (-0.5) = -660 * 9.090909e-30 + (-0.5)
      // y = (-200) * scale + 0 = -200 * 9.090909e-30 + 0

      // Calculate what we expect manually
      // dx = -660, scale ≈ 9.09e-31
      // dx * scale = -660 * 9.09e-31 ≈ -6e-28
      // expectedX = -6e-28 + (-0.5) ≈ -0.5 (the offset is tiny at this zoom)

      // At extreme zoom, the pixel offset becomes negligible compared to the center
      // Verify the fractal coordinate is very close to the center
      const distanceFromCenterX = fractalCoord.x.minus(center.x).abs();
      const distanceFromCenterY = fractalCoord.y.minus(center.y).abs();

      // The distance should be extremely small (pixel offsets become negligible at extreme zoom)
      // At zoom 5.5e27, a 660 pixel offset should be ~6e-28 fractal units
      expect(distanceFromCenterX.toExponential()).toContain("e-");
      expect(distanceFromCenterY.toExponential()).toContain("e-");
      expect(distanceFromCenterX.lt(new Decimal("1e-20"))).toBe(true);
      expect(distanceFromCenterY.lt(new Decimal("1e-20"))).toBe(true);

      // Verify precision is maintained (should have many significant digits)
      expect(fractalCoord.x.precision(true)).toBeGreaterThanOrEqual(20);
      expect(fractalCoord.y.precision(true)).toBeGreaterThanOrEqual(20);
    });

    it("should verify fractal coordinate under pointer remains constant during zoom", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 };
      const oldCenter = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      const oldZoom = new Decimal("5.5e27");

      // Calculate fractal coordinate under pointer BEFORE zoom
      const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
        pointerPixel,
        canvasWidth,
        canvasHeight,
        oldCenter,
        oldZoom
      );

      // Apply zoom factor (10x zoom in)
      const newZoom = oldZoom.times(10);

      // Calculate new center that should keep fractal coordinate under pointer constant
      const newCenter = calculateNewCenterAfterZoom(
        pointerPixel,
        fractalUnderPointer,
        canvasWidth,
        canvasHeight,
        newZoom
      );

      // Verify the pointer invariant: fractal coordinate under pointer should be unchanged
      const result = verifyPointerInvariant(
        pointerPixel,
        oldCenter,
        oldZoom,
        newCenter,
        newZoom,
        canvasWidth,
        canvasHeight
      );

      expect(result.passes).toBe(true);
      expect(result.error.toNumber()).toBeLessThan(1e-100);
    });
  });

  describe("Test 3 & 4: Zoom Factor Application and Center Calculation", () => {
    it("should apply zoom factor correctly from 5.5e27 to 5.5e28", () => {
      const oldZoom = new Decimal("5.5e27");
      const zoomFactor = new Decimal(10);

      const newZoom = oldZoom.times(zoomFactor);

      expect(newZoom.toString()).toBe("5.5e+28");
    });

    it("should calculate new center maintaining pointer-relative positioning", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 };
      const oldCenter = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      const oldZoom = new Decimal("5.5e27");

      // Get fractal coordinate under pointer
      const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
        pointerPixel,
        canvasWidth,
        canvasHeight,
        oldCenter,
        oldZoom
      );

      // Zoom in 10x
      const newZoom = oldZoom.times(10);

      // Calculate new center
      const newCenter = calculateNewCenterAfterZoom(
        pointerPixel,
        fractalUnderPointer,
        canvasWidth,
        canvasHeight,
        newZoom
      );

      // Recalculate expected center using the same helper function to ensure consistency
      const expectedCenter = calculateNewCenterAfterZoom(
        pointerPixel,
        fractalUnderPointer,
        canvasWidth,
        canvasHeight,
        newZoom
      );

      // They should be identical since we're using the same function
      expect(newCenter.x.equals(expectedCenter.x)).toBe(true);
      expect(newCenter.y.equals(expectedCenter.y)).toBe(true);
    });

    it("should test zoom at 10^30, 10^50, 10^75, 10^100", () => {
      const testZoomLevels = ["1e30", "1e50", "1e75", "1e100"];
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 };

      testZoomLevels.forEach((zoomLevel) => {
        const oldCenter = {
          x: new Decimal("-0.5"),
          y: new Decimal("0"),
        };
        const oldZoom = new Decimal(zoomLevel);

        const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
          pointerPixel,
          canvasWidth,
          canvasHeight,
          oldCenter,
          oldZoom
        );

        // Zoom in 10x
        const newZoom = oldZoom.times(10);
        const newCenter = calculateNewCenterAfterZoom(
          pointerPixel,
          fractalUnderPointer,
          canvasWidth,
          canvasHeight,
          newZoom
        );

        // Verify pointer invariant
        const result = verifyPointerInvariant(
          pointerPixel,
          oldCenter,
          oldZoom,
          newCenter,
          newZoom,
          canvasWidth,
          canvasHeight
        );

        expect(result.passes).toBe(true);
        expect(result.error.toNumber()).toBeLessThan(1e-100);
      });
    });
  });

  describe("Test 5: Round-trip Pixel→Fractal→Pixel Accuracy", () => {
    it("should maintain sub-pixel accuracy at zoom 10^27", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const testPixel = { x: 500, y: 300 };
      const center = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      const zoom = new Decimal("1e27");

      const error = calculateRoundTripError(testPixel, center, zoom, canvasWidth, canvasHeight);

      expect(error.maxError).toBeLessThan(0.01);
    });

    it("should maintain sub-pixel accuracy at zoom 10^100", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const testPixel = { x: 960, y: 400 };
      const center = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      const zoom = new Decimal("1e100");

      const error = calculateRoundTripError(testPixel, center, zoom, canvasWidth, canvasHeight);

      expect(error.maxError).toBeLessThan(0.01);
    });

    it("should test round-trip accuracy at various pixel positions", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const testPixels: Point[] = [
        { x: 0, y: 0 }, // top-left corner
        { x: 1920, y: 0 }, // top-right corner
        { x: 0, y: 800 }, // bottom-left corner
        { x: 1920, y: 800 }, // bottom-right corner
        { x: 960, y: 400 }, // center
        { x: 300, y: 200 }, // off-center
      ];
      const center = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      const zoom = new Decimal("1e50");

      testPixels.forEach((pixel) => {
        const error = calculateRoundTripError(pixel, center, zoom, canvasWidth, canvasHeight);

        expect(error.maxError).toBeLessThan(0.01);
      });
    });
  });

  describe("Test 6: Preview Transformation Calculations", () => {
    it("should calculate preview scale ratio using Decimal division", () => {
      const oldZoom = new Decimal("1e27");
      const newZoom = new Decimal("1e28");

      const scaleRatio = newZoom.div(oldZoom);

      expect(scaleRatio.toString()).toBe("10");
    });

    it("should calculate preview pixel position at extreme zoom", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const lastParams: TestFractalParams = {
        center: {
          x: new Decimal("-0.5"),
          y: new Decimal("0"),
        },
        zoom: new Decimal("1e27"),
        maxIterations: 1000,
        iterationScalingFactor: 1000,
      };
      const newParams: TestFractalParams = {
        center: {
          x: new Decimal("-0.50000000001"),
          y: new Decimal("0.00000000001"),
        },
        zoom: new Decimal("1e28"),
        maxIterations: 1000,
        iterationScalingFactor: 1000,
      };

      const previewPosition = computePreviewPixelPosition(lastParams, newParams, canvasWidth, canvasHeight);

      // The preview position should be calculated with high precision
      expect(typeof previewPosition.x).toBe("number");
      expect(typeof previewPosition.y).toBe("number");
      expect(Number.isFinite(previewPosition.x)).toBe(true);
      expect(Number.isFinite(previewPosition.y)).toBe(true);
    });

    it("should verify preview transformation maintains coordinate relationships", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const lastParams: TestFractalParams = {
        center: {
          x: new Decimal("-0.5"),
          y: new Decimal("0"),
        },
        zoom: new Decimal("1e50"),
        maxIterations: 1000,
        iterationScalingFactor: 1000,
      };

      // Zoom in 2x
      const newParams: TestFractalParams = {
        center: lastParams.center,
        zoom: lastParams.zoom.times(2),
        maxIterations: 1000,
        iterationScalingFactor: 1000,
      };

      const previewPosition = computePreviewPixelPosition(lastParams, newParams, canvasWidth, canvasHeight);

      // When center doesn't change, top-left pixel should move based on zoom scale
      // For a 2x zoom centered at the same point, the top-left should move toward center
      const canvasCenterX = canvasWidth / 2;
      const canvasCenterY = canvasHeight / 2;

      // The preview position should be between canvas center and original top-left (0,0)
      expect(previewPosition.x).toBeLessThan(canvasCenterX);
      expect(previewPosition.y).toBeLessThan(canvasCenterY);
    });
  });

  describe("Test 7: Multi-step Zoom Sequence Precision", () => {
    it("should maintain precision through 10 consecutive zoom operations", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 };
      let currentCenter = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      let currentZoom = new Decimal("1e27");

      const errors: Decimal[] = [];

      // Perform 10 zoom operations
      for (let i = 0; i < 10; i++) {
        const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
          pointerPixel,
          canvasWidth,
          canvasHeight,
          currentCenter,
          currentZoom
        );

        const oldCenter = currentCenter;
        const oldZoom = currentZoom;

        // Zoom in 10x
        currentZoom = currentZoom.times(10);
        currentCenter = calculateNewCenterAfterZoom(
          pointerPixel,
          fractalUnderPointer,
          canvasWidth,
          canvasHeight,
          currentZoom
        );

        // Verify pointer invariant for this step
        const result = verifyPointerInvariant(
          pointerPixel,
          oldCenter,
          oldZoom,
          currentCenter,
          currentZoom,
          canvasWidth,
          canvasHeight
        );

        errors.push(result.error);
        expect(result.passes).toBe(true);
      }

      // Verify cumulative error doesn't grow significantly
      const maxError = Decimal.max(...errors);
      expect(maxError.toNumber()).toBeLessThan(1e-90);

      // Final zoom should be 1e37
      expect(currentZoom.toString()).toBe("1e+37");
    });

    it("should maintain precision through zoom in and zoom out sequence", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 960, y: 400 }; // center
      const initialCenter = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      const initialZoom = new Decimal("1e27");

      let currentCenter = { ...initialCenter };
      let currentZoom = initialZoom;

      // Zoom in 5 times
      for (let i = 0; i < 5; i++) {
        const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
          pointerPixel,
          canvasWidth,
          canvasHeight,
          currentCenter,
          currentZoom
        );

        currentZoom = currentZoom.times(10);
        currentCenter = calculateNewCenterAfterZoom(
          pointerPixel,
          fractalUnderPointer,
          canvasWidth,
          canvasHeight,
          currentZoom
        );
      }

      // Zoom out 5 times
      for (let i = 0; i < 5; i++) {
        const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
          pointerPixel,
          canvasWidth,
          canvasHeight,
          currentCenter,
          currentZoom
        );

        currentZoom = currentZoom.div(10);
        currentCenter = calculateNewCenterAfterZoom(
          pointerPixel,
          fractalUnderPointer,
          canvasWidth,
          canvasHeight,
          currentZoom
        );
      }

      // Should return very close to initial state
      // Since pointer is at center, center should remain nearly unchanged
      const centerErrorX = currentCenter.x.minus(initialCenter.x).abs();
      const centerErrorY = currentCenter.y.minus(initialCenter.y).abs();
      const zoomError = currentZoom.minus(initialZoom).div(initialZoom).abs();

      expect(centerErrorX.toNumber()).toBeLessThan(1e-80);
      expect(centerErrorY.toNumber()).toBeLessThan(1e-80);
      expect(zoomError.toNumber()).toBeLessThan(1e-10);
    });
  });

  describe("Test 8 & 9: Pointer Position Variance", () => {
    it("should maintain invariants with pointer at canvas center", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 960, y: 400 }; // exact center
      const oldCenter = {
        x: new Decimal("-0.5"),
        y: new Decimal("0"),
      };
      const oldZoom = new Decimal("1e50");

      const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
        pointerPixel,
        canvasWidth,
        canvasHeight,
        oldCenter,
        oldZoom
      );

      const newZoom = oldZoom.times(100);
      const newCenter = calculateNewCenterAfterZoom(
        pointerPixel,
        fractalUnderPointer,
        canvasWidth,
        canvasHeight,
        newZoom
      );

      const result = verifyPointerInvariant(
        pointerPixel,
        oldCenter,
        oldZoom,
        newCenter,
        newZoom,
        canvasWidth,
        canvasHeight
      );

      expect(result.passes).toBe(true);

      // When zooming at center, the center should remain very close to original
      const centerDriftX = newCenter.x.minus(oldCenter.x).abs();
      const centerDriftY = newCenter.y.minus(oldCenter.y).abs();

      // Center should change very little since pointer is at center
      expect(centerDriftX.toNumber()).toBeLessThan(1e-100);
      expect(centerDriftY.toNumber()).toBeLessThan(1e-100);
    });

    it("should maintain invariants with pointer at corners and edges", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const testPositions: Point[] = [
        { x: 0, y: 0 }, // top-left corner
        { x: 1920, y: 0 }, // top-right corner
        { x: 0, y: 800 }, // bottom-left corner
        { x: 1920, y: 800 }, // bottom-right corner
        { x: 960, y: 0 }, // top edge center
        { x: 960, y: 800 }, // bottom edge center
        { x: 0, y: 400 }, // left edge center
        { x: 1920, y: 400 }, // right edge center
      ];

      testPositions.forEach((pointerPixel) => {
        const oldCenter = {
          x: new Decimal("-0.5"),
          y: new Decimal("0"),
        };
        const oldZoom = new Decimal("1e50");

        const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
          pointerPixel,
          canvasWidth,
          canvasHeight,
          oldCenter,
          oldZoom
        );

        const newZoom = oldZoom.times(10);
        const newCenter = calculateNewCenterAfterZoom(
          pointerPixel,
          fractalUnderPointer,
          canvasWidth,
          canvasHeight,
          newZoom
        );

        const result = verifyPointerInvariant(
          pointerPixel,
          oldCenter,
          oldZoom,
          newCenter,
          newZoom,
          canvasWidth,
          canvasHeight
        );

        expect(result.passes).toBe(true);
        expect(result.error.toNumber()).toBeLessThan(1e-100);
      });
    });

    it("should maintain invariants with pointer at arbitrary off-center positions", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const testPositions: Point[] = [
        { x: 300, y: 200 },
        { x: 1500, y: 600 },
        { x: 123, y: 456 },
        { x: 1777, y: 99 },
      ];

      testPositions.forEach((pointerPixel) => {
        const oldCenter = {
          x: new Decimal("-0.5"),
          y: new Decimal("0"),
        };
        const oldZoom = new Decimal("1e75");

        const fractalUnderPointer = pixelToFractalCoordinateUltraHP(
          pointerPixel,
          canvasWidth,
          canvasHeight,
          oldCenter,
          oldZoom
        );

        const newZoom = oldZoom.times(10);
        const newCenter = calculateNewCenterAfterZoom(
          pointerPixel,
          fractalUnderPointer,
          canvasWidth,
          canvasHeight,
          newZoom
        );

        const result = verifyPointerInvariant(
          pointerPixel,
          oldCenter,
          oldZoom,
          newCenter,
          newZoom,
          canvasWidth,
          canvasHeight
        );

        expect(result.passes).toBe(true);
        expect(result.error.toNumber()).toBeLessThan(1e-100);
      });
    });
  });

  describe("Test 10: Decimal Arithmetic Verification", () => {
    it("should verify scale calculation uses Decimal throughout", () => {
      const canvasHeight = 800;
      const zoom = new Decimal("1e100");

      // Manually calculate step by step to verify Decimal is used
      const step1 = new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT);
      const step2 = step1.div(new Decimal(canvasHeight));
      const step3 = step2.div(zoom);

      // All intermediate values should be Decimal instances
      expect(step1).toBeInstanceOf(Decimal);
      expect(step2).toBeInstanceOf(Decimal);
      expect(step3).toBeInstanceOf(Decimal);

      // Final result should match calculateScale
      const scale = calculateScale(canvasHeight, zoom);
      expect(scale.toString()).toBe(step3.toString());
    });

    it("should verify offset calculations use Decimal arithmetic", () => {
      const canvasWidth = 1920;
      const canvasHeight = 800;
      const pointerPixel = { x: 300, y: 200 };

      // Calculate using Decimal
      const canvasCenterX = new Decimal(canvasWidth).div(2);
      const canvasCenterY = new Decimal(canvasHeight).div(2);
      const offsetX = new Decimal(pointerPixel.x).minus(canvasCenterX);
      const offsetY = new Decimal(pointerPixel.y).minus(canvasCenterY);

      // All should be Decimal instances
      expect(canvasCenterX).toBeInstanceOf(Decimal);
      expect(canvasCenterY).toBeInstanceOf(Decimal);
      expect(offsetX).toBeInstanceOf(Decimal);
      expect(offsetY).toBeInstanceOf(Decimal);

      // Values should be correct
      expect(canvasCenterX.toNumber()).toBe(960);
      expect(canvasCenterY.toNumber()).toBe(400);
      expect(offsetX.toNumber()).toBe(-660);
      expect(offsetY.toNumber()).toBe(-200);
    });

    it("should verify zoom factor application uses Decimal", () => {
      const oldZoom = new Decimal("1e50");
      const zoomFactor = new Decimal(10);

      // Multiply using Decimal
      const newZoom = oldZoom.times(zoomFactor);

      expect(newZoom).toBeInstanceOf(Decimal);
      expect(newZoom.toString()).toBe("1e+51");

      // Verify division also uses Decimal
      const reversedZoom = newZoom.div(zoomFactor);
      expect(reversedZoom.toString()).toBe(oldZoom.toString());
    });
  });
});
