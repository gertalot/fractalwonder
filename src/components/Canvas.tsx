"use client";

import mandelbrot from "@/fractals/mandelbrot/mandelbrot";
import { useFractalInteraction } from "@/hooks/use-fractal-interaction";
import { FractalParams, useFractalStore } from "@/hooks/use-store";
import canvasSize from "@/lib/canvas-size";
import { useCallback, useEffect, useRef, useState } from "react";

export const Canvas = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const lastImageDataRef = useRef<ImageData | null>(null);
  const lastParamsRef = useRef<FractalParams | null>(null);
  const [canvasDimensions, setCanvasDimensions] = useState<{
    width: number;
    height: number;
  }>({ width: 0, height: 0 });
  const initialRenderDoneRef = useRef(false); // Flag to ensure initial render runs only once

  // Main render function (for fractal, etc.)
  const render = useCallback(() => {
    const currentParams = useFractalStore.getState().params;
    console.log("rendering fractal... center, zoom: ", currentParams.center, currentParams.zoom);

    const canvas = canvasRef.current;
    if (!canvas) return;
    if (canvas.width === 0 || canvas.height === 0) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    mandelbrot(canvas, currentParams);

    // Store the image data and params for use in the preview function
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    lastImageDataRef.current = imageData;
    lastParamsRef.current = JSON.parse(JSON.stringify(currentParams));

    console.log("rendering fractal done.");
  }, []);

  /**************************************************************************
   * handle the browser window resizing
   * NOTE this should be in use-fractal-interaction
   **************************************************************************/

  useEffect(() => {
    const updateCanvasDimensions = () => {
      const size = canvasSize(canvasRef.current);
      // Only update state if dimensions actually changed to avoid loops
      setCanvasDimensions((prevSize) => {
        if (prevSize.width !== size.width || prevSize.height !== size.height) {
          console.log("Updating canvas dimensions state:", size);
          return size;
        }
        return prevSize;
      });
    };

    updateCanvasDimensions(); // Initial size calculation
    window.addEventListener("resize", updateCanvasDimensions);
    return () => {
      console.log("Cleaning up resize listener.");
      window.removeEventListener("resize", updateCanvasDimensions);
    };
  }, []); // Runs once on mount

  // Effect to update canvas element dimensions AND render
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const { width, height } = canvasDimensions;
    if (width > 0 && height > 0) {
      // Update the actual canvas element's dimensions if they differ.
      if (canvas.width !== width || canvas.height !== height) {
        console.log(`Resizing canvas element to: ${width}x${height} and drawing checkerboard.`);
        canvas.width = width;
        canvas.height = height;
      }
    } else {
      console.log("Skipping canvas element resize: dimensions are zero.");
    }
  }, [canvasDimensions]);

  // Perform the initial fractal render *once* after dimensions are set
  useEffect(() => {
    // Ensure dimensions are valid and initial render hasn't happened
    if (
      canvasDimensions.width > 0 &&
      canvasDimensions.height > 0 &&
      !initialRenderDoneRef.current // Check the flag
    ) {
      console.log("Performing initial fractal render...");
      render();
      initialRenderDoneRef.current = true; // Set the flag
    }
  }, [canvasDimensions, render]);

  /**************************************************************************
   * Fast preview handling
   **************************************************************************/
  useFractalInteraction({
    canvasRef,
    lastImageDataRef,
    lastParamsRef,
    onInteractionEnd: render,
  });

  return (
    <canvas
      ref={canvasRef}
      // Remove explicit width/height attributes here; they are set dynamically by the useEffect
      className="block h-full w-full"
      style={{ touchAction: "none", cursor: "grab" }}
    />
  );
};
