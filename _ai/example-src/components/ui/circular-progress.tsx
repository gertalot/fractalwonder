// ABOUTME: Circular progress indicator component that displays progress as a pie chart filling from 0% to 100%
// ABOUTME: Used for showing render progress with transparent background, matching icon sizes

import { cn } from "@/lib/utils";

interface CircularProgressProps {
  value: number;
  className?: string;
}

function CircularProgress({ value, className }: CircularProgressProps) {
  // Clamp value between 0 and 100
  const clampedValue = Math.max(0, Math.min(100, value));
  
  // Calculate the angle for the pie chart (0 = top, clockwise)
  const angle = (clampedValue / 100) * 360;
  
  // Convert polar to cartesian coordinates for the arc
  // Center is at (12, 12), radius is 10
  const radius = 10;
  const centerX = 12;
  const centerY = 12;
  
  // Calculate the end point of the arc
  // Subtract 90 degrees to start from top (12 o'clock)
  const radians = ((angle - 90) * Math.PI) / 180;
  const endX = centerX + radius * Math.cos(radians);
  const endY = centerY + radius * Math.sin(radians);
  
  // For angles > 180, we need to use the large arc flag
  const largeArcFlag = angle > 180 ? 1 : 0;
  
  // Build the SVG path for the pie slice
  let piePath = "";
  if (clampedValue > 0) {
    if (clampedValue === 100) {
      // Full circle - use a circle element instead of path
      piePath = "";
    } else {
      // Pie slice: Move to center, line to top, arc to end point, line back to center
      piePath = `M ${centerX} ${centerY} L ${centerX} ${centerY - radius} A ${radius} ${radius} 0 ${largeArcFlag} 1 ${endX} ${endY} Z`;
    }
  }
  
  return (
    <svg
      viewBox="0 0 24 24"
      className={cn("size-6", className)}
      role="progressbar"
      aria-valuenow={clampedValue}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={`Progress: ${clampedValue}%`}
    >
      {/* Background circle */}
      <circle
        cx={centerX}
        cy={centerY}
        r={radius}
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        className="text-white/30"
      />
      
      {/* Progress fill */}
      {clampedValue === 100 ? (
        // Full circle
        <circle
          cx={centerX}
          cy={centerY}
          r={radius}
          fill="currentColor"
          className="text-white/70 transition-opacity duration-300"
        />
      ) : clampedValue > 0 ? (
        // Pie slice
        <path
          d={piePath}
          fill="currentColor"
          className="text-white/70 transition-all duration-300"
        />
      ) : null}
    </svg>
  );
}

export { CircularProgress };

