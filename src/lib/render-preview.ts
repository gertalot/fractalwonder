import { FractalParams, Point } from "@/hooks/use-store"; // Adjust path
import canvasSize from "@/lib/canvas-size"; // Adjust path
import { RefObject } from "react";

export type RenderPreviewProps = {
  canvasRef: RefObject<HTMLCanvasElement | null>;
  lastImageDataRef: RefObject<ImageData | null>;
  lastParams: FractalParams; // Params corresponding to lastImageDataRef
  interactionOffset: Point; // Relative offset change (CSS pixels)
  interactionScale: number; // Relative scale change (multiplier)
};

export function renderPreview({
  canvasRef,
  lastImageDataRef,
  lastParams, // Renamed from lastParamsRef for clarity
  interactionOffset,
  interactionScale,
}: RenderPreviewProps): void {
  const canvas = canvasRef.current;
  const imageData = lastImageDataRef.current;

  // Add null checks for clarity
  if (!canvas || !imageData || !lastParams) {
    console.warn("renderPreview skipped: Missing canvas, imageData, or lastParams.");
    return;
  }

  console.log("Preview render");

  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const { width: canvasWidth, height: canvasHeight } = canvasSize(canvas);

  ctx.clearRect(0, 0, canvasWidth, canvasHeight);

  // Create a temporary canvas to hold the last committed image data
  // This is efficient as putImageData/drawImage are fast.
  const tempCanvas = document.createElement("canvas");
  tempCanvas.width = imageData.width;
  tempCanvas.height = imageData.height;
  const tempCtx = tempCanvas.getContext("2d");
  if (!tempCtx) return;
  tempCtx.putImageData(imageData, 0, 0);

  // --- Apply Transformation ---
  // The transformation should map the *original* image (tempCanvas)
  // to its new position/scale based on the *interaction* offset/scale.

  ctx.save();

  // 1. Translate to the center of the canvas (optional, but common for scaling)
  ctx.translate(canvasWidth / 2, canvasHeight / 2);

  // 2. Apply the interaction scale centered around the canvas center
  ctx.scale(interactionScale, interactionScale);

  // 3. Apply the interaction offset (relative drag)
  // The offset is in CSS pixels relative to the start position.
  // Since we scaled relative to the center, we need to apply the offset *after* scaling? No.
  // Let's rethink: The offset/scale are applied *relative* to the state when interaction started.
  // We want to draw the *original* image data, transformed.

  // Reset transform for clarity before applying the final transform
  ctx.restore();
  ctx.save();

  // The final transformation:
  // - Translate by the interactionOffset (how much the view was dragged)
  // - Translate to the canvas center (because we want to scale around the center)
  // - Scale by interactionScale
  // - Translate back from the canvas center
  // - Draw the original image centered

  // Order: Translate (pan), Scale (zoom)
  // The interactionOffset is the pan amount in screen pixels.
  // The interactionScale is the zoom multiplier.

  // To scale around the *center* of the view while also panning:
  // Translate to the point that should end up at the center AFTER transform.
  // Then Scale.
  // Then draw the image relative to the new origin.

  // Alternative: Apply transforms relative to the canvas origin (0,0)
  // 1. Translate by interactionOffset: Moves the origin.
  // 2. Scale by interactionScale: Scales around the *new* origin. This isn't right for zooming towards center/cursor.

  // --- Correct Approach using the standard context transforms ---
  // We want to apply the cumulative effect: the image should appear scaled by `interactionScale`
  // and shifted by `interactionOffset`.
  // Think about a point (0,0) on the *original* image (tempCanvas). Where should it end up?
  // It should be scaled by `interactionScale` and then translated by `interactionOffset`.

  // However, renderPreview usually works by taking the *full* image (lastImageData) and drawing it
  // such that it reflects the *difference* between `lastParams` and the `current effective params`.

  // Let's use the `renderPreview` logic based on params difference, but calculate the
  // *effective* current params based on the interaction state.

  // const currentEffectiveZoom = lastParams.zoom * interactionScale;

  // Calculate the effective center based on offset and scale
  // This needs the inverse of the logic in commitChanges: map the screen offset back to fractal offset
  // This requires pixelToFractalCoordinate or similar logic.
  // Simpler: calculate the pixel offset corresponding to the fractal center change.

  // Let's use the direct transform approach based on offset/scale passed in:
  // We draw the `lastImageData` (which corresponds to `lastParams`) and transform it
  // by the *relative* `interactionOffset` and `interactionScale`.

  // Apply the translation first (pan)
  ctx.translate(interactionOffset.x, interactionOffset.y);

  // Now scale. Scaling should happen around a pivot point.
  // If we want to scale around the center of the canvas:
  ctx.translate(canvasWidth / 2, canvasHeight / 2);
  ctx.scale(interactionScale, interactionScale);
  ctx.translate(-canvasWidth / 2, -canvasHeight / 2);

  // Draw the last rendered image data (tempCanvas) centered on the canvas.
  // The transforms applied above will position and scale it correctly.
  // Ensure the original image data was rendered centered. Assume it was.
  const drawX = (canvasWidth - tempCanvas.width) / 2;
  const drawY = (canvasHeight - tempCanvas.height) / 2;

  ctx.drawImage(tempCanvas, drawX, drawY);

  ctx.restore();
}
