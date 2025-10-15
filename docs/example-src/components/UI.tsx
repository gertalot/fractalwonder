"use client";

import { Progress } from "@/components/ui/progress";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useFullscreen } from "@/hooks/use-full-screen";
import { derivedRealIterations, useFractalStore } from "@/hooks/use-store";
import { useUIVisibilityTrigger } from "@/hooks/use-ui-visibility-trigger";
import { SiGithub } from "@icons-pack/react-simple-icons";
import { Popover, PopoverContent, PopoverTrigger } from "@radix-ui/react-popover";
import { Home, Info, Maximize, Minimize } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "./ui/button";

export const UI = () => {
  const center = useFractalStore((state) => state.params.center);
  const zoom = useFractalStore((state) => state.params.zoom);
  const maxIterations = useFractalStore((state) => state.params.maxIterations);
  const iterationScalingFactor = useFractalStore((state) => state.params.iterationScalingFactor);
  const realMaxIterations = derivedRealIterations({ center, zoom, maxIterations, iterationScalingFactor });
  const colorScheme = useFractalStore((state) => state.colorScheme);
  // Get exact renderProgress for the progress bar (no rounding)
  const renderProgress = useFractalStore((state) => state.renderProgress);
  const algorithmMode = useFractalStore((state) => state.algorithmMode);
  const lastRenderTime = useFractalStore((state) => state.lastRenderTime);
  // const setFractalParams = useFractalStore((state) => state.setFractalParams);
  const resetFractalState = useFractalStore((state) => state.resetFractalState);
  const setUIVisible = useFractalStore((state) => state.setUIVisible);
  const setAlgorithmMode = useFractalStore((state) => state.setAlgorithmMode);
  // const setColorScheme = useFractalStore((state) => state.setColorScheme);

  const [isPopoverOpen, setIsPopoverOpen] = useState(false);
  const { isFullscreen, toggleFullscreen } = useFullscreen();
  const { isVisible, setIsVisible, setIsHovering} = useUIVisibilityTrigger({ isAlwaysVisible: isPopoverOpen });

  // Sync UI visibility to the store
  useEffect(() => {
    setUIVisible(isVisible);
  }, [isVisible, setUIVisible]);

  // // Keyboard shortcut: T to toggle algorithm
  // useEffect(() => {
  //   const handleKeyPress = (e: KeyboardEvent) => {
  //     if (e.key === 't' || e.key === 'T') {
  //       setAlgorithmMode(algorithmMode === "perturbation" ? "standard" : "perturbation");
  //     }
  //   };
  //   window.addEventListener('keydown', handleKeyPress);
  //   return () => window.removeEventListener('keydown', handleKeyPress);
  // }, [algorithmMode, setAlgorithmMode]);

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
          {isVisible && renderProgress > 0 && renderProgress < 100 && (
            <div className="ml-4 w-40">
              <Progress value={renderProgress} className="h-3" />
            </div>
          )}
        </div>
        <div className="flex-1 text-center">
          <p className="text-sm text-white">
            Center: x: {center.x.toFixed(6)}, y: {center.y.toFixed(6)}, zoom: {zoom.toExponential(2)}, max. iterations: {realMaxIterations}
          </p>
          <p className="text-sm text-white">
            Algorithm: {algorithmMode === "perturbation" ? "Perturbation" : "Standard"} | 
            Render: {lastRenderTime > 0 ? `${(lastRenderTime / 1000).toFixed(2)}s` : "N/A"} | 
            Color: {colorScheme}
            {renderProgress > 0 && renderProgress < 100 && ` | Rendering: ${renderProgress.toFixed(0)}%`}
          </p>
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
