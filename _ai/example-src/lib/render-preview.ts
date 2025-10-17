import { FractalParams } from "@/hooks/use-store"; // Adjust path if needed
import canvasSize from "@/lib/canvas-size"; // Adjust path if needed
import { fractalToPixelCoordinateUltraHP, pixelToFractalCoordinateUltraHP } from "./coordinates";

// Extracted function for testing pixel computation at extreme zoom levels
export function computePreviewPixelPosition(
  lastParams: FractalParams,
  newParams: FractalParams,
  canvasWidth: number,
  canvasHeight: number
): { x: number; y: number } {
  // Use ultra-high-precision coordinate functions for extreme zoom levels
  // Step 1: Convert top-left pixel (0,0) from last view to fractal coordinates
  const topLeftPixel = { x: 0, y: 0 };
  const fractalCoord = pixelToFractalCoordinateUltraHP(
    topLeftPixel,
    canvasWidth,
    canvasHeight,
    lastParams.center,
    lastParams.zoom
  );

  // Step 2: Convert that fractal coordinate to pixel coordinates in the new view
  const newPixelCoord = fractalToPixelCoordinateUltraHP(
    fractalCoord,
    canvasWidth,
    canvasHeight,
    newParams.center,
    newParams.zoom
  );

  return newPixelCoord;
}

export type RenderPreviewProps = {
  canvas: HTMLCanvasElement;
  lastImageData: ImageData | null; // Allow null if no image rendered yet
  lastParams: FractalParams; // Params corresponding to lastImageData
  newParams: FractalParams; // Updated params for preview
};

// Helper to create an ImageBitmap from ImageData (can be optimized)
// Note: createImageBitmap is generally preferred but is async.
// For simplicity in a sync loop, we use an offscreen canvas.
function createImageFromData(imageData: ImageData): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = imageData.width;
  canvas.height = imageData.height;
  const ctx = canvas.getContext("2d");
  if (ctx) {
    ctx.putImageData(imageData, 0, 0);
  }
  return canvas;
}

export function renderPreview({ canvas, lastImageData, lastParams, newParams }: RenderPreviewProps): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    console.error("Could not get 2D context from canvas");
    return;
  }

  const { width: canvasWidth, height: canvasHeight } = canvasSize(canvas);

  // Clear the canvas for the new preview frame
  ctx.clearRect(0, 0, canvasWidth, canvasHeight);

  // If there's no previous image data, we can't draw a preview
  if (!lastImageData) {
    console.log("renderPreview: No stored image to preview");
    return;
  }

  // Ensure lastImageData dimensions match current canvas dimensions.
  // If not, the preview might be distorted. This implementation assumes they match.
  // You might need additional logic if the canvas can resize between renders.
  if (lastImageData.width !== canvasWidth || lastImageData.height !== canvasHeight) {
    console.warn("Canvas dimensions do not match last image data dimensions. Preview might be inaccurate.");
    // Decide how to handle this: maybe skip preview, or try to adapt?
    // For now, we'll proceed assuming they should match.
  }

  // --- Calculate Transformation ---

  // 1. Determine the scale factor based on zoom change
  const scaleRatio = newParams.zoom.div(lastParams.zoom).toNumber();

  // 2. Calculate the destination dimensions on the canvas
  const destWidth = canvasWidth * scaleRatio;
  const destHeight = canvasHeight * scaleRatio;

  // 3. Compute preview pixel position using extracted function for testability
  const newViewTopLeftPixel = computePreviewPixelPosition(
    lastParams,
    newParams,
    canvasWidth,
    canvasHeight
  );

  // --- Draw the Transformed Image ---

  // Create a temporary canvas/image source from lastImageData
  // Note: Doing this every frame isn't ideal for performance.
  // Consider caching the result if lastImageData hasn't changed.
  const sourceImage = createImageFromData(lastImageData);

  // Disable image smoothing for a sharper pixelated look during zoom,
  // which is often preferred for fractal previews.
  ctx.imageSmoothingEnabled = false;

  // Draw the last rendered image (from sourceImage) onto the main canvas,
  // applying the calculated scale and translation.
  // drawImage(image, dx, dy, dWidth, dHeight)
  ctx.drawImage(
    sourceImage,
    newViewTopLeftPixel.x, // dx: destination x
    newViewTopLeftPixel.y, // dy: destination y
    destWidth, // dWidth: destination width
    destHeight // dHeight: destination height
  );

  // Re-enable image smoothing if needed elsewhere
  ctx.imageSmoothingEnabled = true;
}
