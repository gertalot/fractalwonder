import { pixelToFractalCoordinate } from "@/lib/coordinates";
import debounce from "@/lib/debounce";
import { renderPreview } from "@/lib/render-preview";
import { RefObject, useCallback, useEffect, useRef } from "react";
import { FractalParams, initialFractalParamState, Point, useFractalStore } from "./use-store";

const ZOOM_SENSITIVITY = 0.0005;
const ZOOM_SENSITIVITY_WITH_CTRL = 0.005;
const MIN_ZOOM = 1.0;
const MAX_ZOOM = Number.MAX_SAFE_INTEGER;
const COMMIT_DELAY = 1500;

interface UseFractalInteractionProps {
  canvasRef: RefObject<HTMLCanvasElement | null>;
  lastImageDataRef: RefObject<ImageData | null>;
  lastParamsRef: RefObject<FractalParams | null>;
  onInteractionStart?: () => void;
  onInteractionEnd?: () => void;
  onParamsChange?: (params: FractalParams) => void;
}

const noop = () => {};
const noopParams = (_params: FractalParams) => {};

export function useFractalInteraction({
  canvasRef, // the canvas we're drawing on
  lastImageDataRef, // image data from the last full render
  lastParamsRef, // corresponding fractal parameters from the last full render
  onInteractionStart = noop, // callback when the user starts dragging/zooming
  onInteractionEnd = noop, // callback when the user is done dragging/zooming
  onParamsChange = noopParams, // called when parameters change during interaction
}: UseFractalInteractionProps) {
  const { params, setFractalParams } = useFractalStore();

  // ------------------------------------------------------------------------
  // interaction state
  // ------------------------------------------------------------------------

  // track if we're currently in a drag or zoom operation
  const isDraggingRef = useRef(false);
  const isZoomingRef = useRef(false);
  const isOneShotPreviewLoop = useRef(false);

  // track the starting point for drag and scroll wheel interaction so we can
  // compute an offset and an accumulated wheel delta
  const dragStartRef = useRef<Point | null>(null);
  const wheelStartRef = useRef<number | null>(null);

  // keep track of how far the user has dragged or zoomed
  // these are updated during the interaction.
  // NOTE: these values use *device* pixels, taking the device pixel ration
  // into account. The offset is used to calculate the fractal center in params
  const interactionDragOffsetRef = useRef<Point | null>(null);
  const interactionParamsRef = useRef<FractalParams | null>(null);

  // track the fractal parameters at the start of an interaction
  const interactionStartParamsRef = useRef<FractalParams | null>(null);

  // ------------------------------------------------------------------------
  // Preview render loop
  //
  // This loop uses the saved ImageData from the canvas to show a preview
  // of the user's dragging and zooming in real-time, by translating and
  // scaling the image.
  // ------------------------------------------------------------------------

  // track animation frame requests for the preview render loop
  const animationFrameIdRef = useRef<number | null>(null);

  const previewRenderLoop = useCallback(() => {
    if (!isOneShotPreviewLoop.current && !isDraggingRef.current && !isZoomingRef.current) {
      animationFrameIdRef.current = null;
      console.log("previewRenderLoop: not interacting; stopping loop");
      return;
    }

    if (isOneShotPreviewLoop.current) {
      console.log("previewRenderLoop: params changed externally. Running preview loop once");
      interactionParamsRef.current = useFractalStore.getState().params;
    }

    const canvas = canvasRef.current;
    const lastParams = lastParamsRef.current;
    const lastImageData = lastImageDataRef.current;

    if (!canvas || !lastParams || !lastImageData || !interactionParamsRef.current) {
      console.log("renderPreviewLoop: Missing refs for rendering.");

      // only request next frame if we're still interacting, even if this frame
      // failed.
      if (isOneShotPreviewLoop.current || isDraggingRef.current || isZoomingRef.current) {
        animationFrameIdRef.current = requestAnimationFrame(previewRenderLoop);
      } else {
        // stop the loop
        animationFrameIdRef.current = null;
      }
      return;
    }
    // always set this to false, to make it actually "one-shot"
    isOneShotPreviewLoop.current = false;

    renderPreview({
      canvas,
      lastImageData,
      lastParams,
      newParams: interactionParamsRef.current,
    });

    // loop by requesting the next frame
    animationFrameIdRef.current = requestAnimationFrame(previewRenderLoop);
  }, [canvasRef, lastImageDataRef, lastParamsRef]);

  const startPreviewLoopIfNeeded = useCallback(
    (oneShot: boolean = false) => {
      if (animationFrameIdRef.current === null || oneShot) {
        console.log("Starting preview render loop. oneShot = ", oneShot);
        isOneShotPreviewLoop.current = oneShot;
        animationFrameIdRef.current = requestAnimationFrame(previewRenderLoop);
      } else {
        console.log("Preview loop already running");
      }
    },
    [previewRenderLoop]
  );

  // ------------------------------------------------------------------------
  // Interaction Lifecycle Handlers
  // ------------------------------------------------------------------------

  const captureInteractionStartState = useCallback(() => {
    // Determine the most up-to-date parameters to start from:
    let startingCenter: Point;
    let startingZoom: number;

    // If refs hold valid data from a previous interaction segment use that,
    // otherwise use last fully rendered params, fallback to store.
    if (interactionParamsRef.current) {
      console.log("captureInteractionStartState: Resuming from previous interaction refs");
      startingCenter = { ...interactionParamsRef.current.center };
      startingZoom = interactionParamsRef.current.zoom;
    } else if (lastParamsRef.current) {
      console.log("captureInteractionStartState: Starting from lastParamsRef");
      startingCenter = { ...lastParamsRef.current.center };
      startingZoom = lastParamsRef.current.zoom;
    } else {
      console.warn("captureInteractionStartState: Falling back to zustand params");
      const currentParams = useFractalStore.getState().params;
      startingCenter = { ...currentParams.center };
      startingZoom = currentParams.zoom;
    }

    // Capture these as the starting point for calculations in updateInteracting
    // interactionStartParamsRef.current = {
    //   center: startingCenter,
    //   zoom: startingZoom,
    //   maxIterations: params.maxIterations,
    //   iterationScalingFactor: params.iterationScalingFactor,
    // };
    interactionStartParamsRef.current = { ...params };
    console.log(
      "Interaction start params captured:",
      interactionStartParamsRef.current.center,
      "zoom = ",
      interactionStartParamsRef.current.zoom
    );

    startPreviewLoopIfNeeded();

    // Reset relative offsets for the new sequence
    interactionDragOffsetRef.current = { x: 0, y: 0 };
    // dragStartRef is set in handlePointerDown
    wheelStartRef.current = null; // Reset wheel start if used

    // Ensure interaction state refs reflect the captured start state *initially*
    // updateInteracting will modify these based on offsets.
    interactionParamsRef.current = {
      ...initialFractalParamState.params,
      ...interactionParamsRef.current,
      zoom: startingZoom,
      center: { ...startingCenter },
    };

    onInteractionStart(); // Notify consumer
  }, [lastParamsRef, onInteractionStart, params, startPreviewLoopIfNeeded]);

  // Calculates new center/zoom based on offsets relative to interactionStartParamsRef
  const updateInteractingState = useCallback(() => {
    const canvas = canvasRef.current;
    const startParams = interactionStartParamsRef.current;

    // Guards
    if (!interactionParamsRef.current) return;
    if (!canvas || !startParams || canvas.width === 0 || canvas.height === 0) {
      console.warn("updateInteracting skipped: Missing refs or zero dimensions.");
      return;
    }

    // Get current interaction values
    const currentZoom = interactionParamsRef.current.zoom;
    const currentDragOffset = interactionDragOffsetRef.current;
    if (!currentDragOffset) return;

    // Calculate the target canvas center in DEVICE pixels based on drag
    const targetCanvasCenter = {
      x: canvas.width / 2 - currentDragOffset.x,
      y: canvas.height / 2 - currentDragOffset.y,
    };

    // Convert the target canvas center coordinate to a fractal coordinate using
    // the STARTING parameters of this interaction sequence as the frame of reference.
    const newFractalCenter = pixelToFractalCoordinate(
      targetCanvasCenter,
      canvas.width,
      canvas.height,
      startParams.center,
      startParams.zoom
    );

    const newParams: FractalParams = {
      ...startParams,
      center: newFractalCenter,
      zoom: currentZoom,
    };
    interactionParamsRef.current = newParams;
    setFractalParams(newParams);
    console.log("Interacting: zoom = ", newParams.zoom, "center = ", newParams.center);

    onParamsChange(newParams);
  }, [canvasRef, onParamsChange, setFractalParams]);

  // The debounced version of this function is called when the user stops
  // interacting, after a COMMIT_DELAY time of inactivity.
  const stopInteracting = useCallback(() => {
    // don't stop when we're still dragging (i.e. the pointer is down)
    if (isDraggingRef.current) {
      return;
    }

    // we set zooming to false automatically 1 second after any scroll wheel
    // (or other) action; isDraggingRef is set to false in the pointerup handler
    isZoomingRef.current = false;

    // reset all interaction state
    dragStartRef.current = null;
    wheelStartRef.current = null;
    interactionDragOffsetRef.current = null;
    interactionStartParamsRef.current = null;
    interactionParamsRef.current = null;

    const canvas = canvasRef.current;
    if (canvas) {
      canvas.style.cursor = "grab";
    }

    onInteractionEnd();
    console.log("Interactions ended");
  }, [canvasRef, onInteractionEnd]);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const debouncedStopInteracting = useCallback(debounce(stopInteracting, COMMIT_DELAY), [stopInteracting]);

  // ------------------------------------------------------------------------
  // Event and Interaction Handling
  //
  // We set up event handlers for pointer and wheel events, and we listen
  // to changes in device pixel ratio, canvas size, and fractal params being
  // changed outside of this interaction code
  // ------------------------------------------------------------------------

  // Respond to external changes in fractal params
  useEffect(() => {
    if (!interactionParamsRef.current) {
      // something changed and we're not currently interacting.
      // Run the preview loop once.
      interactionParamsRef.current = { ...params };
      isOneShotPreviewLoop.current = true;
      startPreviewLoopIfNeeded(true); // doesn't work, but previewRenderLoop does!
      stopInteracting();
    } else {
      // something changed and we're currently interacting.
      // Run the preview loop once if the params change was external
      // (i.e. not set from the interaction code)
      const paramsMatch =
        interactionParamsRef.current.center.x === params.center.x &&
        interactionParamsRef.current.center.y === params.center.y &&
        interactionParamsRef.current.zoom === params.zoom;

      if (!paramsMatch) {
        interactionParamsRef.current = { ...params };
        isOneShotPreviewLoop.current = true;
        startPreviewLoopIfNeeded(true); // doesn't work, but previewRenderLoop does!
        stopInteracting();
      }
    }
  }, [params, startPreviewLoopIfNeeded, stopInteracting]);

  // ------------------------------------------------------------------------
  // handle device pixel ratio changes
  // (e.g. when the window is moved to another display, or the screen
  // resolution changes)
  // ------------------------------------------------------------------------

  const devicePixelRatioRef = useRef(typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1);

  useEffect(() => {
    const updateDpr = () => {
      const newDpr = window.devicePixelRatio || 1;
      if (newDpr !== devicePixelRatioRef.current) {
        console.log("updateDpr: Device Pixel Ratio changed to:", newDpr);
        devicePixelRatioRef.current = newDpr;
      }
    };
    window.addEventListener("resize", updateDpr);
    updateDpr();
    return () => window.removeEventListener("resize", updateDpr);
  }, []);

  // ------------------------------------------------------------------------
  // Handle pointer and wheel events
  // ------------------------------------------------------------------------

  const handlePointerDown = useCallback(
    (event: PointerEvent) => {
      const canvas = canvasRef.current;

      if (!canvas) {
        return;
      }

      captureInteractionStartState();
      isDraggingRef.current = true;
      dragStartRef.current = { x: event.clientX, y: event.clientY };

      console.log("handlePointerDown: interactionParamsRef.current = ", interactionParamsRef.current);

      canvas.style.cursor = "grabbing";
      canvas.setPointerCapture(event.pointerId);
    },
    [canvasRef, captureInteractionStartState]
  );

  const handlePointerUp = useCallback(
    (event: PointerEvent) => {
      if (isDraggingRef.current) {
        isDraggingRef.current = false;

        const canvas = canvasRef.current;
        if (canvas) {
          canvas.style.cursor = "grab";
          canvas.releasePointerCapture(event.pointerId);
        }
        console.log("handlePointerUp: interactionParamsRef.current = ", interactionParamsRef.current);
        debouncedStopInteracting();
      }
    },
    [canvasRef, debouncedStopInteracting]
  );

  const handlePointerMove = useCallback(
    (event: PointerEvent) => {
      if (!isDraggingRef.current || !dragStartRef.current) {
        return;
      }
      const dpr = devicePixelRatioRef.current;
      const currentPos = { x: event.clientX, y: event.clientY };
      const startPos = dragStartRef.current;
      const cssOffset = {
        x: currentPos.x - startPos.x,
        y: currentPos.y - startPos.y,
      };
      const deviceOffset = {
        x: cssOffset.x * dpr,
        y: cssOffset.y * dpr,
      };
      interactionDragOffsetRef.current = deviceOffset;

      updateInteractingState();
    },
    [updateInteractingState]
  );

  const handleWheel = useCallback(
    (event: WheelEvent) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      event.preventDefault();
      // Capture start state only if not already actively interacting
      if (!isDraggingRef.current && !isZoomingRef.current) {
        captureInteractionStartState();
      }
      isZoomingRef.current = true;

      const isControlPressed = event.getModifierState("Control");
      const zoomSensitivity = isControlPressed ? ZOOM_SENSITIVITY_WITH_CTRL : ZOOM_SENSITIVITY;

      // TODO: Implement zoom centering based on mouse position (event.clientX, event.clientY)
      // This requires calculating the fractal coordinate under the cursor,
      // zooming, and then adjusting the center so that coordinate stays under the cursor.
      // const zoomCenterPx = { x: event.clientX * devicePixelRatioRef.current, y: event.clientY * devicePixelRatioRef.current };

      // const pos = { x: event.clientX, y: event.clientY };
      const delta = event.deltaY * zoomSensitivity;
      const zoomFactor = Math.exp(-delta);
      const newZoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, (interactionParamsRef.current?.zoom || 1) * zoomFactor));
      interactionParamsRef.current = {
        ...initialFractalParamState.params,
        ...interactionParamsRef.current,
        zoom: newZoom,
      };

      updateInteractingState();

      // automatically consider the interaction over when the user
      // stops scrolling the wheel for a while
      debouncedStopInteracting();
    },
    [canvasRef, captureInteractionStartState, debouncedStopInteracting, updateInteractingState]
  );

  // set up event listeners
  useEffect(() => {
    const element = canvasRef.current;
    if (!element) return;

    element.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointermove", handlePointerMove);
    element.addEventListener("wheel", handleWheel, { passive: false });

    return () => {
      element.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointermove", handlePointerMove);
      element.removeEventListener("wheel", handleWheel);
    };
  }, [canvasRef, handlePointerDown, handlePointerMove, handlePointerUp, handleWheel]);
}
