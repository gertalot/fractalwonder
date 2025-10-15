// ABOUTME: Circular progress indicator positioned in lower left corner during render
// ABOUTME: Displays render progress as a pie chart, visible only during active rendering

import { useFractalStore } from "@/hooks/use-store";
import { CircularProgress } from "./ui/circular-progress";

export const CircularProgressIndicator = () => {
  const renderProgress = useFractalStore((state) => state.renderProgress);
  const isUIVisible = useFractalStore((state) => state.isUIVisible);
  
  // Only show during active rendering (progress between 1 and 99) AND when UI is hidden
  if (renderProgress <= 0 || renderProgress >= 100 || isUIVisible) {
    return null;
  }
  
  return (
    <div className="fixed bottom-4 left-4 z-40">
      <CircularProgress value={renderProgress} />
    </div>
  );
};

