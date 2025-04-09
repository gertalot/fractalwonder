import { useState, useEffect, useRef } from "react";
import { SiGithub } from "@icons-pack/react-simple-icons";
import { useFractalStore } from "./hooks/use-store";
import { Popover, PopoverTrigger, PopoverContent } from "@radix-ui/react-popover";
import { Info, Maximize, Minimize } from "lucide-react";
import { Button } from "./ui/button";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { useFullscreen } from "./hooks/use-full-screen";

export const UI = () => {
  const [isVisible, setIsVisible] = useState(true);
  const [isPopoverOpen, setIsPopoverOpen] = useState(false);
  const { params, colorScheme } = useFractalStore();

  const { isFullscreen, toggle: toggleFullscreen } = useFullscreen();

  const [isHovering, setIsHovering] = useState(false);
  const [isPointerActive, setIsPointerActive] = useState(false);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  const resetTimeout = () => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    if (!isHovering && !isPointerActive && !isPopoverOpen) {
      timeoutRef.current = setTimeout(() => setIsVisible(false), 2000);
    }
  };

  if (isVisible) {
    resetTimeout();
  }

  // trigger UI visibility when the user does... stuff.
  useEffect(() => {
    const handleVisibilityTrigger = () => {
      setIsVisible(true);
    };

    const handlePointerStart = () => {
      setIsVisible(true);
      setIsPointerActive(true);
    };
    const handlePointerEnd = () => {
      setIsPointerActive(false);
    };

    // Add wheel listener
    window.addEventListener("wheel", handleVisibilityTrigger);
    window.addEventListener("mousemove", handleVisibilityTrigger);
    window.addEventListener("mousedown", handlePointerStart);
    window.addEventListener("mouseup", handlePointerEnd);
    window.addEventListener("touchstart", handlePointerStart);
    window.addEventListener("touchend", handlePointerEnd);
    window.addEventListener("keydown", handleVisibilityTrigger);

    return () => {
      window.removeEventListener("wheel", handleVisibilityTrigger);
      window.removeEventListener("mousemove", handleVisibilityTrigger);
      window.removeEventListener("mousedown", handlePointerStart);
      window.removeEventListener("mouseup", handlePointerEnd);
      window.removeEventListener("touchstart", handlePointerStart);
      window.removeEventListener("touchend", handlePointerEnd);
      window.removeEventListener("keydown", handleVisibilityTrigger);
    };
  }, []);

  return (
    <div
      className="fixed inset-x-0 bottom-0"
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
    >
      <div
        className={`
        flex items-center justify-between px-4 py-3
        bg-black/50 backdrop-blur-sm transition-opacity
        duration-300 ${isVisible ? "opacity-100" : "opacity-0"}
      `}
      >
        <Popover
          open={isPopoverOpen}
          onOpenChange={(open) => {
            setIsPopoverOpen(open);
            setIsVisible(true);
          }}
        >
          <PopoverTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="text-white hover:text-gray-200 hover:bg-white/10 mr-4 rounded-full cursor-pointer"
            >
              <Info size={24} />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-80 bg-black/70 backdrop-blur-sm border-gray-800 mb-3 p-4">
            <div className="space-y-2 text-white">
              <h3 className="font-medium">Fractal Wonder</h3>
              <p className="text-sm text-muted-foreground">
                Use mouse/touch to pan and zoom. Keyboard shortcuts: [ and ] to cycle color schemes.
              </p>
              <div className="mt-8 flex items-center gap-2 text-sm text-muted-foreground">
                <a
                  href="https://github.com/gertalot/fractalwonder"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-white hover:text-gray-200 transition-colors"
                >
                  <SiGithub size={24} />
                </a>
                <span>Made by Gert</span>
              </div>
            </div>
          </PopoverContent>
        </Popover>
        <div className="flex space-x-4">
          <p className="text-sm text-muted-foreground pr-12">
            x: {params.center.x}, y: {params.center.y}
          </p>
          <p className="text-sm text-muted-foreground pr-12">
            zoom: {params.zoom.toFixed(2)}, maxIter: {params.maxIterations}
          </p>
          <p className="text-sm text-muted-foreground pr-12">colorScheme: {colorScheme}</p>
        </div>

        {/* Add fullscreen toggle button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="text-white hover:bg-white/10 hover:text-white rounded-full cursor-pointer"
              onClick={toggleFullscreen}
            >
              {isFullscreen ? <Minimize size={24} /> : <Maximize size={24} />}
            </Button>
          </TooltipTrigger>
          <TooltipContent className="bg-black/70 backdrop-blur-sm border-gray-800 text-white">
            {isFullscreen ? "Exit full screen (f)" : "Full screen (f)"}
          </TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
};
