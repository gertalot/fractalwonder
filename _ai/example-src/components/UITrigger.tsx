"use client";

import { ChevronDownIcon } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

export const UITrigger = ({ isOpen: isOpen }: { isOpen?: boolean }) => {
  const [isHovering, setIsHovering] = useState(false);
  const [isVisible, setIsVisible] = useState(true);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Helper function to set the hide timeout
  const setHideTimeout = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    if (!isHovering && !isOpen) {
      timeoutRef.current = setTimeout(() => {
        setIsVisible(false);
      }, 1000);
    }
  }, [isHovering, isOpen]);

  // Handle mouse movement
  useEffect(() => {
    const handleMouseMove = () => {
      setIsVisible(true);
      setHideTimeout();
    };

    window.addEventListener("mousemove", handleMouseMove);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [isHovering, isOpen, setHideTimeout]);

  // Handle popover state changes
  useEffect(() => {
    if (isOpen) {
      setIsVisible(true);
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    } else {
      // When popover closes, start the hide timeout
      setHideTimeout();
    }
  }, [isOpen, isHovering, setHideTimeout]);

  return (
    <ChevronDownIcon
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
      className={`
        fixed bottom-4 left-4 
        rounded-full
        ${isOpen ? "bg-neutral-500" : "bg-neutral-600"}
        hover:bg-neutral-500
        cursor-pointer
        transition-all duration-300 ease-in-out
        flex items-center justify-center
        ${isVisible || isHovering || isOpen ? "opacity-100" : "opacity-0"}
        ${isHovering || isOpen ? "scale-120" : "scale-100"}
        ${isOpen ? "rotate-180" : "rotate-0"}
      `}
    />
  );
};
