import { RefObject } from "react";
import canvasSize from "@/lib/canvas-size";
import { FractalParams } from "@/hooks/use-store";

export type RenderPreviewProps = {
  canvasRef: RefObject<HTMLCanvasElement | null>;
  lastImageDataRef: RefObject<ImageData | null>;
  lastParamsRef: RefObject<FractalParams | null>;
  params: FractalParams;
};

export function renderPreview({ canvasRef, lastImageDataRef, lastParamsRef, params }: RenderPreviewProps): void {
  if (!lastImageDataRef.current || !canvasRef.current || !lastParamsRef.current) {
    console.log("no image data or canvas");
    return;
  }
  const ctx = canvasRef.current.getContext("2d");
  if (!ctx) return;

  const { width: canvasWidth, height: canvasHeight } = canvasSize(canvasRef.current);

  ctx.clearRect(0, 0, canvasWidth, canvasHeight);

  const tempCanvas = document.createElement("canvas");
  const imageData = lastImageDataRef.current;
  tempCanvas.width = imageData.width;
  tempCanvas.height = imageData.height;
  const tempCtx = tempCanvas.getContext("2d");
  if (!tempCtx) return;

  tempCtx.putImageData(imageData, 0, 0);

  const lastCenter = lastParamsRef.current.center;
  const lastZoom = lastParamsRef.current.zoom;
  const zoomRatio = params.zoom / lastZoom;
  const centerDiffX = params.center.x - lastCenter.x;
  const centerDiffY = params.center.y - lastCenter.y;
  const scale = 4 / canvasHeight / params.zoom;
  const pixelDiffX = centerDiffX / scale;
  const pixelDiffY = centerDiffY / scale;

  ctx.save();
  ctx.translate(canvasWidth / 2, canvasHeight / 2);
  ctx.scale(zoomRatio, zoomRatio);
  ctx.translate(-pixelDiffX, -pixelDiffY);
  ctx.drawImage(tempCanvas, -tempCanvas.width / 2, -tempCanvas.height / 2);
  ctx.restore();
}