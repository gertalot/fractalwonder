"use client";

import { useFractalInteraction } from "@/hooks/use-fractal-interaction";
import { FractalParams, useFractalStore } from "@/hooks/use-store";
import canvasSize from "@/lib/canvas-size";
import { useCallback, useEffect, useRef, useState } from "react";

export const Canvas = () => {
  // const { params } = useFractalStore();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const lastImageDataRef = useRef<ImageData | null>(null);
  const lastParamsRef = useRef<FractalParams | null>(null);
  const [canvasDimensions, setCanvasDimensions] = useState<{
    width: number;
    height: number;
  }>({ width: 0, height: 0 });

  // 5. Wrap renderCheckerboard in useCallback (optional but good practice)
  const renderCheckerboard = useCallback(() => {
    const canvas = canvasRef.current;
    // Added checks for canvas and context availability early
    if (!canvas) {
      console.warn("renderCheckerboard called before canvas ref was ready.");
      return;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      console.warn("renderCheckerboard called before canvas context was ready.");
      return;
    }
    if (canvas.width === 0 || canvas.height === 0) {
      console.warn("renderCheckerboard called with zero dimensions.");
      return;
    }

    // console.log("Rendering checkerboard pattern... ", canvas.width, canvas.height);

    // Checkerboard pattern parameters
    const tileSize = 64; // pixels per tile
    const colors = ["#0b1220", "#1b2523"]; // darker gray colors

    // draw the tiles
    for (let i = 0; i < canvas.width; i += tileSize) {
      for (let j = 0; j < canvas.height; j += tileSize) {
        ctx.fillStyle = colors[((i + j) / tileSize) % 2 ? 0 : 1];
        ctx.fillRect(i, j, tileSize, tileSize);
      }
    }

    const centerX = canvas.width / 2;
    const centerY = canvas.height / 2;
    const radius = canvas.height * 0.25;

    // Draw circle with red outline
    ctx.beginPath();
    ctx.arc(centerX, centerY, radius, 0, Math.PI * 2);
    ctx.strokeStyle = "red";
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw horizontal line
    ctx.beginPath();
    ctx.moveTo(0, centerY);
    ctx.lineTo(canvas.width, centerY);
    ctx.strokeStyle = "red";
    ctx.lineWidth = 1;
    ctx.stroke();

    // Draw vertical line
    ctx.beginPath();
    ctx.moveTo(centerX, 0);
    ctx.lineTo(centerX, canvas.height);
    ctx.strokeStyle = "red";
    ctx.lineWidth = 1;
    ctx.stroke();
  }, []); // No external dependencies needed here, relies on canvasRef.current

  // Main render function (for fractal, etc.)
  // const render = useCallback(() => {
  //   console.log("rendering fractal... center, zoom: ", params.center, params.zoom);
  //   const canvas = canvasRef.current;
  //   if (!canvas) return;
  //   if (canvas.width === 0 || canvas.height === 0) return;
  //   const ctx = canvas.getContext("2d");
  //   if (!ctx) return;

  //   // 4. REMOVE renderCheckerboard() from here if it was present

  //   // Store the image data and params for use in the preview function
  //   const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
  //   lastImageDataRef.current = imageData;
  //   lastParamsRef.current = params;

  //   console.log("rendering fractal done.");
  // }, [params]);

  /**************************************************************************
   * handle the browser window resizing
   **************************************************************************/
  useEffect(() => {
    const updateCanvasDimensions = () => {
      setCanvasDimensions(canvasSize(canvasRef.current));
    };
    updateCanvasDimensions(); // Initial size calculation
    window.addEventListener("resize", updateCanvasDimensions);
    return () => window.removeEventListener("resize", updateCanvasDimensions);
  }, []); // Runs once on mount

  // Effect to update canvas element dimensions AND render checkerboard
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const { width, height } = canvasDimensions;

    // Only proceed if dimensions are valid (non-zero)
    if (width > 0 && height > 0) {
      // Update the actual canvas element's dimensions if they differ.
      // IMPORTANT: Setting canvas.width or canvas.height clears the canvas.
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }

      // 2. Call renderCheckerboard AFTER dimensions are set/canvas is cleared
      renderCheckerboard();

      lastImageDataRef.current = ctx.getImageData(0, 0, canvas.width, canvas.height);
      lastParamsRef.current = useFractalStore.getState().params;
    }
    // Add renderCheckerboard to dependencies (it's stable due to useCallback)
  }, [canvasDimensions, renderCheckerboard]);

  /**************************************************************************
   * Fast preview handling
   **************************************************************************/
  useFractalInteraction({
    canvasRef,
    lastImageDataRef,
    lastParamsRef,
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
