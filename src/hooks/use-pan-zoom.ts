import { useState, useEffect, useCallback, useRef, useMemo } from "react";

interface InteractionState {
  isDragging: boolean;
  isZooming: boolean;
  dragOffset: { x: number; y: number };
  wheelDelta: number;
  pointerPosition: { x: number; y: number } | null;
}

const initialInteractionState: InteractionState = {
  isDragging: false,
  isZooming: false,
  dragOffset: { x: 0, y: 0 },
  wheelDelta: 0,
  pointerPosition: { x: 0, y: 0 },
};

interface PanZoomProps {
  wheelSensitivity?: number;
  onDragStart?: () => void;
  onDragEnd?: () => void;
  onWheelStart?: () => void;
  onWheelEnd?: () => void;
}

const panZoomPropsDefaults: PanZoomProps = {
  wheelSensitivity: 0.1,
  onDragStart: () => {},
  onDragEnd: () => {},
  onWheelStart: () => {},
  onWheelEnd: () => {},
};

export function usePanZoom(elementRef: React.RefObject<HTMLElement | null>, props?: PanZoomProps) {
  const [interactionState, setInteractionState] = useState<InteractionState>(initialInteractionState);
  const mergedProps = useMemo(() => ({ ...panZoomPropsDefaults, ...props }) as Required<PanZoomProps>, [props]);
  const dragStartRef = useRef<{ x: number; y: number } | null>(null);
  const wheelStartRef = useRef<number | null>(null);
  const wheelTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const getDevicePixelRatio = useCallback(() => {
    return window.devicePixelRatio || 1;
  }, []);

  // Pointer down means the user starts dragging
  const handlePointerDown = useCallback(
    (event: PointerEvent) => {
      // console.log("handlePointerDown: Starting drag", { x: event.clientX, y: event.clientY });

      // track starting position for drag
      dragStartRef.current = {
        x: event.clientX,
        y: event.clientY,
      };
      setInteractionState((prev) => ({
        ...prev,
        isDragging: true,
        dragOffset: { x: 0, y: 0 },
      }));
      mergedProps.onDragStart();
    },
    [interactionState.dragOffset, getDevicePixelRatio]
  );

  // Pointer move while holding the mouse down means the user is dragging
  const handlePointerMove = useCallback(
    (event: PointerEvent) => {
      if (dragStartRef.current && interactionState.isDragging) {
        const dpr = getDevicePixelRatio();
        // Calculate new offset based on how much the user has moved since they
        // started dragging
        const dragStart = dragStartRef.current;
        const dragOffset = {
          x: (event.clientX - dragStart.x) * dpr,
          y: (event.clientY - dragStart.y) * dpr,
        };
        // console.log("handlePointerMove: Dragging", { x: event.clientX, y: event.clientY }, "dragOffset: ", dragOffset);
        setInteractionState((prev) => {
          return {
            ...prev,
            dragOffset,
          };
        });
      }
    },
    [interactionState.isDragging, getDevicePixelRatio]
  );

  // Pointer up means the user has finished dragging
  const handlePointerUp = useCallback(() => {
    // console.log("handlePointerUp: Finished drag");
    setInteractionState((prev) => ({
      ...prev,
      isDragging: false,
    }));
    dragStartRef.current = null;
    mergedProps.onDragEnd();
  }, []);

  // Mouse wheel means the user is zooming in or out
  const handleWheel = useCallback(
    (event: WheelEvent) => {
      event.preventDefault();
      // Initialize wheelStartRef if null
      if (!wheelStartRef.current) {
        setInteractionState((prev) => ({
          ...prev,
          isZooming: true,
        }));
        wheelStartRef.current = 0;
        mergedProps.onWheelStart();
      }

      // Update wheelStartRef with new deltas
      wheelStartRef.current += event.deltaY / (10 / mergedProps.wheelSensitivity);

      // Clear previous timeout if exists
      if (wheelTimeoutRef.current) {
        clearTimeout(wheelTimeoutRef.current);
      }

      setInteractionState((prev) => ({
        ...prev,
        wheelDelta: wheelStartRef.current || 0,
        pointerPosition: { x: event.clientX, y: event.clientY },
      }));

      // Set new timeout to reset wheelStartRef
      wheelTimeoutRef.current = setTimeout(() => {
        wheelStartRef.current = null;
        setInteractionState((prev) => ({
          ...prev,
          isZooming: false,
        }));
        mergedProps.onWheelEnd();
      }, 1000);
    },
    [mergedProps]
  );

  // Set up event listeners
  useEffect(() => {
    const element = elementRef?.current;
    if (!element) return;

    element.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    element.addEventListener("wheel", handleWheel);

    return () => {
      element.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      element.removeEventListener("wheel", handleWheel);
    };
  }, [elementRef, handlePointerDown, handlePointerMove, handlePointerUp, handleWheel]);

  // Reset function to clear state
  const resetInteractionState = useCallback(() => {
    setInteractionState(initialInteractionState);
    dragStartRef.current = null;
  }, []);

  return {
    ...interactionState,
    setInteractionState,
    resetInteractionState,
  };
}
