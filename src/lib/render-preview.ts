import { FractalParams } from "@/hooks/use-store"; // Adjust path if needed
import canvasSize from "@/lib/canvas-size"; // Adjust path if needed
import { fractalToPixelCoordinate, pixelToFractalCoordinate } from "@/lib/coordinates"; // Adjust path if needed

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
  const scaleRatio = newParams.zoom / lastParams.zoom;

  // 2. Calculate the destination dimensions on the canvas
  const destWidth = canvasWidth * scaleRatio;
  const destHeight = canvasHeight * scaleRatio;

  // 3. Find the fractal coordinate corresponding to the top-left pixel (0,0)
  //    of the *last* rendered view.
  const lastViewTopLeftFractal = pixelToFractalCoordinate(
    { x: 0, y: 0 },
    canvasWidth, // Use current canvas dimensions
    canvasHeight,
    lastParams.center,
    lastParams.zoom
  );

  // 4. Find where this fractal point should be located in *pixel* coordinates
  //    within the *new* view (defined by newParams). This tells us where
  //    the top-left corner of our scaled source image should be drawn.
  const newViewTopLeftPixel = fractalToPixelCoordinate(
    lastViewTopLeftFractal,
    canvasWidth,
    canvasHeight,
    newParams.center,
    newParams.zoom
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
