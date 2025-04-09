import { useState, useRef, useEffect, useCallback } from "react";

export const useUIVisibilityTrigger = (props?: { isAlwaysVisible: boolean }) => {
  const [isVisible, setIsVisible] = useState(true);
  const [isHovering, setIsHovering] = useState(false);
  const [isPointerActive, setIsPointerActive] = useState(false);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  const resetTimeout = useCallback(() => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    if (!isHovering && !isPointerActive && !props?.isAlwaysVisible) {
      timeoutRef.current = setTimeout(() => setIsVisible(false), 2000);
    }
  }, [isHovering, isPointerActive, props?.isAlwaysVisible]);

  useEffect(() => {
    const handleVisibility = () => {
      setIsVisible(true);
      resetTimeout();
    };

    const handlePointerStart = () => {
      setIsVisible(true);
      setIsPointerActive(true);
    };

    const handlePointerEnd = () => {
      setIsPointerActive(false);
    };

    window.addEventListener("keydown", handleVisibility);
    window.addEventListener("wheel", handleVisibility);
    window.addEventListener("mousemove", handleVisibility);
    window.addEventListener("mousedown", handlePointerStart);
    window.addEventListener("mouseup", handlePointerEnd);
    window.addEventListener("touchstart", handlePointerStart);
    window.addEventListener("touchend", handlePointerEnd);

    return () => {
      window.removeEventListener("keydown", handleVisibility);
      window.removeEventListener("wheel", handleVisibility);
      window.removeEventListener("mousemove", handleVisibility);
      window.removeEventListener("mousedown", handlePointerStart);
      window.removeEventListener("mouseup", handlePointerEnd);
      window.removeEventListener("touchstart", handlePointerStart);
      window.removeEventListener("touchend", handlePointerEnd);
    };
  }, [resetTimeout]);

  useEffect(() => {
    resetTimeout();
  }, [isVisible, resetTimeout]);

  return {
    isVisible,
    setIsVisible,
    isHovering,
    setIsHovering,
  };
};
