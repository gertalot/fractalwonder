"use client";

import computeMandelbrot from "@/fractals/mandelbrot/basicmandelbrot";
import { detectWorkerSupport, renderFractal, renderFractalParallel } from "@/fractals/render";
import { ParallelRenderer } from "@/fractals/render/parallel-renderer";
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
  
  // Parallel rendering state
  const setRenderProgress = useFractalStore((state) => state.setRenderProgress);
  const rendererRef = useRef<ParallelRenderer | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const workerSupportRef = useRef(detectWorkerSupport());
  const [renderersReady, setRenderersReady] = useState(false);

  const render = useCallback(async () => {
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
    const algorithmMode = useFractalStore.getState().algorithmMode;
    const setLastRenderTime = useFractalStore.getState().setLastRenderTime;

    // Convert algorithmMode to algorithm name
    const algorithmName = algorithmMode === "perturbation" ? "Perturbation Mandelbrot" : "Mandelbrot Set";

    // Cancel any previous render and terminate workers to stop ongoing computations
    abortControllerRef.current?.abort();
    abortControllerRef.current = new AbortController();
    
    // Always terminate existing workers before starting a new render
    // This ensures we don't have old chunks computing in the background
    if (rendererRef.current) {
      console.log("Terminating existing workers before new render...");
      rendererRef.current.terminate();
      rendererRef.current = null;
    }

    const startTime = performance.now();
    
    try {
      if (workerSupportRef.current) {
        // Initialize fresh workers for this render
        console.log("Initializing workers for render...");
        const renderer = new ParallelRenderer();
        await renderer.init();
        rendererRef.current = renderer;
        
        // Parallel rendering (always)
        await renderFractalParallel(
          rendererRef.current,
          canvas,
          currentParams,
          algorithmName,
          {
            onProgress: setRenderProgress,
            signal: abortControllerRef.current.signal,
          }
        );
      } else {
        // Single-threaded fallback (only if workers not supported)
        console.log("Using single-threaded rendering (workers not supported)");
        renderFractal(canvas, currentParams, computeMandelbrot);
        setRenderProgress(100);
      }

      const duration = performance.now() - startTime;
      console.log(`Render completed in ${duration.toFixed(1)}ms`);
      setLastRenderTime(duration);

      // Store the result for preview rendering
      const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
      if (imageData) {
        lastImageDataRef.current = imageData;
        try {
          lastParamsRef.current = structuredClone(currentParams);
        } catch (_e) {
          console.warn("structuredClone not available, using JSON fallback for params copy.");
          lastParamsRef.current = JSON.parse(JSON.stringify(currentParams));
        }
      }
    } catch (error) {
      if (error instanceof Error && error.message.includes("cancelled")) {
        console.log("Render was cancelled");
      } else {
        console.error("Render failed:", error);
        throw error;
      }
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
  // setRenderProgress is stable from zustand, but included in deps causes infinite loop

  // ------------------------------------------------------------------------
  // Initialize ParallelRenderer
  // ------------------------------------------------------------------------
  useEffect(() => {
    if (!workerSupportRef.current) {
      console.log("Web Workers not supported, using single-threaded rendering");
      setRenderersReady(true);
      return;
    }

    console.log("Initializing ParallelRenderer...");
    const renderer = new ParallelRenderer();
    
    renderer.init()
      .then(() => {
        rendererRef.current = renderer;
        setRenderersReady(true);
      })
      .catch((error) => {
        console.error("Failed to initialize ParallelRenderer:", error);
        console.log("Will use single-threaded rendering");
        workerSupportRef.current = false;
        setRenderersReady(true);
      });

    // Cleanup on unmount
    return () => {
      console.log("Cleaning up ParallelRenderer...");
      renderer.terminate();
      rendererRef.current = null;
      setRenderersReady(false);
      initialRenderDoneRef.current = false; // Reset for remount (React Strict Mode)
      abortControllerRef.current = null; // Reset abort controller
      lastImageDataRef.current = null; // Clear cached image data
      lastParamsRef.current = null; // Clear cached params
    };
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

  // Perform the initial fractal render *once* after dimensions are set AND renderers are ready
  useEffect(() => {
    // Ensure dimensions are valid, renderers are ready, and initial render hasn't happened
    if (
      canvasDimensions.width > 0 &&
      canvasDimensions.height > 0 &&
      renderersReady &&
      !initialRenderDoneRef.current // Check the flag
    ) {
      console.log("Performing initial fractal render...");
      render();
      initialRenderDoneRef.current = true; // Set the flag
    }
  }, [canvasDimensions, renderersReady, render]);

  // Re-render when canvas dimensions change (after initial render)
  useEffect(() => {
    if (
      canvasDimensions.width > 0 &&
      canvasDimensions.height > 0 &&
      renderersReady &&
      initialRenderDoneRef.current // Only after initial render
    ) {
      console.log("Canvas dimensions changed, re-rendering fractal...");
      render();
    }
  }, [canvasDimensions, renderersReady, render]);

  // Track if we're currently interacting (for params change detection)
  const isInteractingRef = useRef(false);

  // Store render function in a ref so it doesn't break the debounce in useFractalInteraction
  const renderRef = useRef(render);
  useEffect(() => {
    renderRef.current = render;
  }, [render]);

  // Re-render when params change externally (e.g., home button, not during user interaction)
  const params = useFractalStore((state) => state.params);
  useEffect(() => {
    // Only trigger render if:
    // 1. Initial render is done (this prevents firing on mount)
    // 2. We're NOT currently interacting (to avoid interfering with drag/zoom)
    if (initialRenderDoneRef.current && !isInteractingRef.current) {
      console.log("Params changed externally, triggering full render...");
      renderRef.current();
    }
  }, [params]);

  // Fast preview handling - use stable callbacks that don't break debounce
  const onInteractionStart = useCallback(() => {
    // Cancel any in-progress render when user starts interacting
    console.log("Interaction started - capturing current canvas state and terminating workers");
    isInteractingRef.current = true;
    
    // Capture current canvas state (including any partially rendered chunks)
    // BEFORE terminating workers
    const canvas = canvasRef.current;
    if (canvas) {
      const ctx = canvas.getContext("2d");
      if (ctx) {
        const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        if (imageData) {
          lastImageDataRef.current = imageData;
          try {
            lastParamsRef.current = structuredClone(useFractalStore.getState().params);
          } catch (_e) {
            console.warn("structuredClone not available, using JSON fallback for params copy.");
            lastParamsRef.current = JSON.parse(JSON.stringify(useFractalStore.getState().params));
          }
          console.log("Captured partial render state for preview");
        }
      }
    }
    
    abortControllerRef.current?.abort();
    
    // Terminate all workers immediately to stop ongoing computations
    // Don't touch renderersReady state - preview loop doesn't need it
    if (rendererRef.current) {
      rendererRef.current.terminate();
      rendererRef.current = null;
    }
  }, []);

  const onInteractionEnd = useCallback(() => {
    isInteractingRef.current = false;
    // Workers will be reinitialized inside render() if needed
    renderRef.current();
  }, []);

  useFractalInteraction({
    canvasRef,
    lastImageDataRef,
    lastParamsRef,
    onInteractionStart,
    onInteractionEnd,
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
