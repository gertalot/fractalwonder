"use client";

import mandelbrot from "@/fractals/mandelbrot/mandelbrot";
import { renderFractal } from "@/fractals/render";
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

  // New function to orchestrate rendering using the external module
  const render = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      console.log("Skipping render: Canvas ref is not available.");
      return;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      console.log("Skipping render: Canvas context is not available.");
      return;
    }

    const currentParams = useFractalStore.getState().params;

    renderFractal(canvas, currentParams, mandelbrot);

    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

    // Store the results if rendering was successful
    if (imageData) {
      lastImageDataRef.current = imageData;
      try {
        lastParamsRef.current = structuredClone(currentParams);
      } catch (_e) {
        console.warn("structuredClone not available, using JSON fallback for params copy.");
        lastParamsRef.current = JSON.parse(JSON.stringify(currentParams));
      }
    } else {
      console.log("Render function returned null, skipping ref updates.");
    }
  }, []);

  // ------------------------------------------------------------------------
  // handle the browser window resizing
  // NOTE this should probably be in use-fractal-interaction
  // ------------------------------------------------------------------------

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
        console.log(`Resizing canvas element to: ${width}x${height}.`);
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

  // Fast preview handling
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
