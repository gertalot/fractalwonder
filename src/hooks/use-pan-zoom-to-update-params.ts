import { RefObject, useEffect, useRef } from "react";
import { usePanZoom } from "./use-pan-zoom";
import { useFractalStore } from "./use-store";
import { pixelToFractalCoordinate } from "@/lib/coordinates";

interface CanvasSize {
  width: number;
  height: number;
}

function canvasSize(canvas: HTMLCanvasElement | null): CanvasSize {
  if (canvas) {
    const dpr = window.devicePixelRatio || 1;
    const { width, height } = canvas.getBoundingClientRect();
    return { width: width * dpr, height: height * dpr };
  }
  return { width: 0, height: 0 };
}

export function usePanZoomToUpdateParams(canvasRef: RefObject<HTMLCanvasElement | null>) {
  const { params, setParams } = useFractalStore();
  const fractalCenterRef = useRef<{ x: number; y: number } | null>(null);
  const fractalZoomRef = useRef<number | null>(null);

  const { isDragging, isZooming, dragOffset, wheelDelta } = usePanZoom(canvasRef, {
    onDragStart: () => {
      fractalCenterRef.current = params.center;
    },
    onWheelStart: () => {
      fractalZoomRef.current = params.zoom;
    },
  });

  useEffect(() => {
    if (!canvasRef.current) return;
    if (isDragging) {
      const { width: canvasWidth, height: canvasHeight } = canvasSize(canvasRef.current);
      const newCanvasCenter = {
        x: canvasWidth / 2 - dragOffset.x,
        y: canvasHeight / 2 - dragOffset.y,
      };

      const newFractalCenter = pixelToFractalCoordinate(
        newCanvasCenter,
        canvasWidth,
        canvasHeight,
        fractalCenterRef.current || params.center,
        params.zoom
      );

      setParams({
        ...params,
        center: {
          x: newFractalCenter.x,
          y: newFractalCenter.y,
        },
      });
    }
  }, [isDragging, dragOffset]);

  useEffect(() => {
    if (!canvasRef.current) return;
    if (isZooming) {
      const zoomFactor = Math.exp(wheelDelta * 0.01);
      const newFractalZoom = (fractalZoomRef.current || params.zoom) * zoomFactor;
      const clampedZoom = Math.max(1, newFractalZoom);

      setParams({
        ...params,
        zoom: clampedZoom,
      });
    }
  }, [isZooming, wheelDelta]);
}
