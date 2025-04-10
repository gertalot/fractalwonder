import { RefObject, useEffect } from "react";
import { FractalParams, useFractalStore } from "./use-store";

function canvasSize(canvas: HTMLCanvasElement | null) {
  if (canvas) {
    const dpr = window.devicePixelRatio || 1;
    const { width, height } = canvas.getBoundingClientRect();
    return { width: width * dpr, height: height * dpr };
  }
  return { width: 0, height: 0 };
}

export function usePreview(
  canvasRef: RefObject<HTMLCanvasElement | null>,
  lastImageDataRef: RefObject<ImageData | null>,
  lastParamsRef: RefObject<FractalParams | null>
) {
  const { params } = useFractalStore();

  useEffect(() => {
    if (!lastImageDataRef.current || !canvasRef.current || !lastParamsRef.current) {
      console.log("no image data or canvas");
      return;
    }
    const ctx = canvasRef.current.getContext("2d");
    if (!ctx) return;

    console.log("previewing...");

    const { width: canvasWidth, height: canvasHeight } = canvasSize(canvasRef.current);

    // Clear the canvas
    ctx.clearRect(0, 0, canvasWidth, canvasHeight);

    // Create a temporary canvas to hold the image data
    const tempCanvas = document.createElement("canvas");
    const imageData = lastImageDataRef.current;
    tempCanvas.width = imageData.width;
    tempCanvas.height = imageData.height;
    const tempCtx = tempCanvas.getContext("2d");
    if (!tempCtx) return;

    // Put the image data on the temporary canvas
    tempCtx.putImageData(imageData, 0, 0);

    const lastCenter = lastParamsRef.current.center;
    const lastZoom = lastParamsRef.current.zoom;

    // Calculate the transformation between the original view and the current view
    const zoomRatio = params.zoom / lastZoom;

    // Calculate the translation in fractal coordinates
    const centerDiffX = params.center.x - lastCenter.x;
    const centerDiffY = params.center.y - lastCenter.y;

    // Convert this to pixel coordinates
    const scale = 4 / canvasHeight / params.zoom;
    const pixelDiffX = centerDiffX / scale;
    const pixelDiffY = centerDiffY / scale;

    // Draw the image with the appropriate transformation
    ctx.save();

    // Move to the center of the canvas
    ctx.translate(canvasWidth / 2, canvasHeight / 2);

    // Apply zoom
    ctx.scale(zoomRatio, zoomRatio);

    // Apply translation
    ctx.translate(-pixelDiffX, -pixelDiffY);

    // Draw the image centered
    ctx.drawImage(tempCanvas, -tempCanvas.width / 2, -tempCanvas.height / 2);

    ctx.restore();
  }, [params]);
}
