import { useState, useEffect, useCallback } from "react";

interface InteractionState {
  isDragging: boolean;
  dragOffset: { x: number; y: number };
  wheelDelta: { x: number; y: number };
}

export function usePanZoom(elementRef: React.RefObject<HTMLElement | null>, onActivity: () => void) {
  const [interactionState, setInteractionState] = useState<InteractionState>({
    isDragging: false,
    dragOffset: { x: 0, y: 0 },
    wheelDelta: { x: 0, y: 0 },
  });
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);

  const getDevicePixelRatio = useCallback(() => {
    return window.devicePixelRatio || 1;
  }, []);

  // Pointer down means the user starts dragging
  const handlePointerDown = useCallback(
    (event: PointerEvent) => {
      const dpr = getDevicePixelRatio();
      // Start from previous drag offset to accumulate movements
      setDragStart({
        x: event.clientX - interactionState.dragOffset.x / dpr,
        y: event.clientY - interactionState.dragOffset.y / dpr,
      });
      setInteractionState((prev) => ({
        ...prev,
        isDragging: true,
      }));
      // let the hook consumer do their thing
      onActivity();
    },
    [onActivity, interactionState.dragOffset, getDevicePixelRatio]
  );

  // Pointer move while holding the mouse down means the user is dragging
  const handlePointerMove = useCallback(
    (event: PointerEvent) => {
      if (dragStart && interactionState.isDragging) {
        const dpr = getDevicePixelRatio();
        // Calculate new offset based on how much the user has moved since they
        // started dragging
        setInteractionState((prev) => ({
          ...prev,
          dragOffset: {
            x: (event.clientX - dragStart.x) * dpr,
            y: (event.clientY - dragStart.y) * dpr,
          },
        }));
        // let the hook consumer do their thing
        onActivity();
      }
    },
    [dragStart, interactionState.isDragging, onActivity, getDevicePixelRatio]
  );

  // Pointer up means the user has finished dragging
  const handlePointerUp = useCallback(() => {
    setDragStart(null);
    setInteractionState((prev) => ({
      ...prev,
      isDragging: false,
    }));
    onActivity();
  }, [onActivity]);

  // Mouse wheel means the user is zooming in or out
  const handleWheel = useCallback(
    (e: WheelEvent) => {
      onActivity();
      const dpr = getDevicePixelRatio();
      setInteractionState((prev) => ({
        ...prev,
        wheelDelta: {
          x: prev.wheelDelta.x + e.deltaX * dpr,
          y: prev.wheelDelta.y + e.deltaY * dpr,
        },
        pointerPosition: { x: e.clientX, y: e.clientY },
      }));
    },
    [onActivity, getDevicePixelRatio]
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
    setInteractionState({
      isDragging: false,
      dragOffset: { x: 0, y: 0 },
      wheelDelta: { x: 0, y: 0 },
    });
    setDragStart(null);
  }, []);

  return {
    ...interactionState,
    setInteractionState,
    resetInteractionState,
  };
}
