"use client";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useFullscreen } from "@/hooks/use-full-screen";
import { derivedRealIterations, useFractalStore } from "@/hooks/use-store";
import { useUIVisibilityTrigger } from "@/hooks/use-ui-visibility-trigger";
import { SiGithub } from "@icons-pack/react-simple-icons";
import { Popover, PopoverContent, PopoverTrigger } from "@radix-ui/react-popover";
import { Home, Info, Maximize, Minimize } from "lucide-react";
import { useState } from "react";
import { Button } from "./ui/button";

export const UI = () => {
  const center = useFractalStore((state) => state.params.center);
  const zoom = useFractalStore((state) => state.params.zoom);
  const maxIterations = useFractalStore((state) => state.params.maxIterations);
  const iterationScalingFactor = useFractalStore((state) => state.params.iterationScalingFactor);
  const realMaxIterations = derivedRealIterations({ center, zoom, maxIterations, iterationScalingFactor });
  const colorScheme = useFractalStore((state) => state.colorScheme);
  // const setFractalParams = useFractalStore((state) => state.setFractalParams);
  const resetFractalState = useFractalStore((state) => state.resetFractalState);
  // const setColorScheme = useFractalStore((state) => state.setColorScheme);

  const [isPopoverOpen, setIsPopoverOpen] = useState(false);
  const { isFullscreen, toggleFullscreen } = useFullscreen();
  const { isVisible, setIsVisible, setIsHovering } = useUIVisibilityTrigger({ isAlwaysVisible: isPopoverOpen });

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
        <div className="flex items-center space-x-4">
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
                className="text-white hover:text-gray-200 hover:bg-white/10 rounded-full cursor-pointer"
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
          <Button
            variant="ghost"
            size="icon"
            className="text-white hover:text-gray-200 hover:bg-white/10 rounded-full cursor-pointer"
            onClick={resetFractalState}
          >
            <Home size={24} />
          </Button>
        </div>
        <div className="flex-1 text-center">
          <p className="text-sm text-white">
            Center: x: {center.x}, y: {center.y}, zoom: {zoom.toFixed(2)}, max. iterations: {realMaxIterations}
          </p>
          <p className="text-sm text-white">Color scheme: {colorScheme}</p>
        </div>
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
