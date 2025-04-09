"use client";

import { usePanZoom } from "@/hooks/use-pan-zoom";
import { useRef, useEffect, useState, useCallback } from "react";

// This ensures the canvas width/height is the same as the container size
function canvasSize(canvas: HTMLCanvasElement | null) {
  if (canvas) {
    const dpr = window.devicePixelRatio;
    const { width, height } = canvas.getBoundingClientRect();
    return { width: width * dpr, height: height * dpr };
  }
  return { width: 0, height: 0 };
}

export const Canvas = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  /**************************************************************************
   * Respond to user interaction by showing a quick preview, and only
   * doing a full render after an idle period.
   **************************************************************************/

  const idleTimerRef = useRef<NodeJS.Timeout | null>(null);

  // Handle user activity and trigger preview/render
  const handleUserActivity = useCallback(() => {
    // Clear any existing timer
    if (idleTimerRef.current) {
      clearTimeout(idleTimerRef.current);
    }

    // Preview immediately for quick user feedback
    preview();

    // Set a new timer for the full render
    idleTimerRef.current = setTimeout(() => {
      render();
      // reset the pan/zoom interaction state
      resetInteractionState();
    }, 1000);
  }, []);

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

    // this counts as user interaction, so ensure it's handled. This
    // will cause preview to be called, and render after idle time.
    handleUserActivity();
  }, [canvasDimensions]);

  /**************************************************************************
   * Track panning and zooming actions from the user; preview immediately,
   * but defer full render until user is idle for one second.
   **************************************************************************/

  const { dragOffset, wheelDelta, setInteractionState, resetInteractionState } = usePanZoom(
    canvasRef,
    handleUserActivity
  );

  const dragOffsetRef = useRef({ x: 0, y: 0 });
  const wheelDeltaRef = useRef({ x: 0, y: 0 });

  // Update the refs whenever values change so they are immediately available
  // in the preview function.
  useEffect(() => {
    dragOffsetRef.current = dragOffset;
  }, [dragOffset]);

  useEffect(() => {
    wheelDeltaRef.current = wheelDelta;
  }, [wheelDelta]);

  /**************************************************************************
   * Fast preview of panning, zooming, and canvas resizing by using the
   * existing canvas image data if available.
   **************************************************************************/

  const imageDataRef = useRef<ImageData | null>(null);

  function preview() {
    console.log("previewing...");
    const currentImageData = imageDataRef.current;
    if (currentImageData) {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const { width: currentWidth, height: currentHeight } = canvasSize(canvas);
      const previousWidth = currentImageData.width;
      const previousHeight = currentImageData.height;

      // Calculate size delta between current and previous canvas
      const widthDelta = currentWidth - previousWidth;
      const heightDelta = currentHeight - previousHeight;

      const centerX = currentWidth / 2;
      const centerY = currentHeight / 2;

      // Clear the canvas first
      ctx.clearRect(0, 0, currentWidth, currentHeight);

      // Create a temporary canvas for manipulation
      const tempCanvas = document.createElement("canvas");
      tempCanvas.width = previousWidth;
      tempCanvas.height = previousHeight;
      tempCanvas.getContext("2d")?.putImageData(currentImageData, 0, 0);

      const resizeOffset = {
        x: widthDelta / 2, // Center offset X
        y: heightDelta / 2, // Center offset Y
      };

      // Calculate scale factor based on wheelDelta
      // A smaller divisor makes zooming more sensitive
      let scaleFactor = 1 + wheelDeltaRef.current.y / 1000;

      if (scaleFactor < 1) {
        // Limit minimum scale factor, also reset wheelDelta in that case
        scaleFactor = 1;
        setInteractionState((prev) => ({
          ...prev,
          wheelDelta: { x: prev.wheelDelta.x, y: 0 },
        }));
      }

      // Save the current transformation state
      ctx.save();

      ctx.translate(resizeOffset.x + dragOffsetRef.current.x, resizeOffset.y + dragOffsetRef.current.y);

      // Then scale from the tracked center
      ctx.translate(centerX, centerY);
      ctx.scale(scaleFactor, scaleFactor);
      ctx.translate(-centerX, -centerY);

      // Draw the temp canvas onto the main canvas with transformations applied
      ctx.drawImage(tempCanvas, 0, 0);

      // Restore the original transformation state
      ctx.restore();
    } else {
      console.log("no preview image data available.");
    }
  }

  function render() {
    console.log("rendering...");
    renderCheckerboard();
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (canvas.width === 0 || canvas.height === 0) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Store in both ref (for immediate access) and state (for reactivity)
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    imageDataRef.current = imageData;
    console.log("rendering done.");
  }

  // Add this new function
  function renderCheckerboard() {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Checkerboard pattern parameters
    const tileSize = 64; // pixels per tile
    const colors = ["#0b1220", "#1b2523"]; // darker gray colors

    for (let i = 0; i < canvas.width; i += tileSize) {
      for (let j = 0; j < canvas.height; j += tileSize) {
        ctx.fillStyle = colors[((i + j) / tileSize) % 2 ? 0 : 1];
        ctx.fillRect(i, j, tileSize, tileSize);
      }
    }

    // Draw center circle and crosshairs
    const centerX = canvas.width / 2;
    const centerY = canvas.height / 2;
    const radius = canvas.height * 0.3; // 30% of canvas height

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
  }

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
