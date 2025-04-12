"use client";

import mandelbrot from "@/fractals/mandelbrot/mandelbrot";
import usePanZoomPreview from "@/hooks/use-pan-zoom-preview";
import { FractalParams, useFractalStore } from "@/hooks/use-store";
import canvasSize from "@/lib/canvas-size";
import { useEffect, useRef, useState } from "react";

export const Canvas = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  /**************************************************************************
   * handle the browser window resizing
   **************************************************************************/

  // keep track of changing dimensions so the canvas can be updated on render
  const [canvasDimensions, setCanvasDimensions] = useState<{
    width: number;
    height: number;
  }>({ width: 0, height: 0 });

  // ensure canvas dimensions are updated to match container size when the window resizes
  // (i.e. every pixel on screen is one pixel in the canvas)
  useEffect(() => {
    const updateCanvasDimensions = () => {
      setCanvasDimensions(canvasSize(canvasRef.current));
    };

    updateCanvasDimensions();
    window.addEventListener("resize", updateCanvasDimensions);
    return () => window.removeEventListener("resize", updateCanvasDimensions);
  }, []);

  // update preview when canvas dimensions change
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // set canvas size to container pixel size
    const { width, height } = canvasDimensions;
    canvas.width = width;
    canvas.height = height;
  }, [canvasDimensions]);

  /**************************************************************************
   * Fast preview of panning, zooming, and canvas resizing by using the
   * existing canvas image data if available.
   **************************************************************************/
  const lastImageDataRef = useRef<ImageData | null>(null);
  const lastParamsRef = useRef<FractalParams | null>(null);

  // handle panning and zooming to update the fractal params
  usePanZoomPreview(canvasRef, lastImageDataRef, lastParamsRef);

  function render() {
    console.log("rendering... center, zoom: ", params.center, params.zoom);
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (canvas.width === 0 || canvas.height === 0) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // renderCheckerboard();

    mandelbrot(canvas, params);

    // Store the image data and params for use in the preview function
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    lastImageDataRef.current = imageData;
    lastParamsRef.current = params;

    console.log("rendering done.");
  }

  /**************************************************************************
   * Track updates to canvas dimensions and parameters, and after an idle
   * period, update the fractal parameters and re-render.
   **************************************************************************/

  const userActivityTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const { params } = useFractalStore();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || canvas.width === 0 || canvas.height === 0) return;

    if (userActivityTimerRef.current) {
      clearTimeout(userActivityTimerRef.current);
    }
    userActivityTimerRef.current = setTimeout(() => {
      userActivityTimerRef.current = null;
      render();
    }, 1000);
  }, [canvasDimensions, params]);

  return (
    <canvas
      ref={canvasRef}
      width="100%"
      height="100%"
      className="block h-full w-full"
      style={{ touchAction: "none" }}
    />
  );
};
