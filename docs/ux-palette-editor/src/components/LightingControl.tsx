import React, { useRef, useState } from 'react';

interface LightingControlProps {
  azimuth: number; // 0-360
  elevation: number; // 0-90
  onChange: (azimuth: number, elevation: number) => void;
}

export function LightingControl({ azimuth, elevation, onChange }: LightingControlProps) {
  const circleRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);

  const calculatePosition = (clientX: number, clientY: number) => {
    if (!circleRef.current) return;

    const rect = circleRef.current.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;
    
    const dx = clientX - centerX;
    const dy = clientY - centerY;
    
    // Calculate azimuth (angle around circle)
    let newAzimuth = Math.atan2(dy, dx) * (180 / Math.PI) + 90;
    if (newAzimuth < 0) newAzimuth += 360;
    
    // Calculate elevation (distance from center, normalized to 0-90)
    const distance = Math.sqrt(dx * dx + dy * dy);
    const maxRadius = rect.width / 2;
    let newElevation = 90 - (Math.min(distance, maxRadius) / maxRadius) * 90;
    
    onChange(newAzimuth, newElevation);
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    setIsDragging(true);
    calculatePosition(e.clientX, e.clientY);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      calculatePosition(e.clientX, e.clientY);
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  const handleGlobalMouseMove = (e: MouseEvent) => {
    if (isDragging) {
      calculatePosition(e.clientX, e.clientY);
    }
  };

  const handleGlobalMouseUp = () => {
    setIsDragging(false);
  };

  React.useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleGlobalMouseMove);
      document.addEventListener('mouseup', handleGlobalMouseUp);
      return () => {
        document.removeEventListener('mousemove', handleGlobalMouseMove);
        document.removeEventListener('mouseup', handleGlobalMouseUp);
      };
    }
  }, [isDragging]);

  // Convert azimuth and elevation to x,y position
  const angle = (azimuth - 90) * (Math.PI / 180);
  const radius = (1 - elevation / 90) * 100; // percentage
  const x = 50 + radius * Math.cos(angle) * 0.5;
  const y = 50 + radius * Math.sin(angle) * 0.5;

  return (
    <div className="bg-white/5 border border-white/10 rounded-lg p-3 space-y-3">
      <div 
        ref={circleRef}
        className="relative w-full aspect-square bg-white/5 rounded-full border border-white/20 cursor-crosshair"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
      >
        {/* Center dot */}
        <div className="absolute top-1/2 left-1/2 w-2 h-2 bg-white/30 rounded-full -translate-x-1/2 -translate-y-1/2" />
        
        {/* Elevation circles */}
        {[0.25, 0.5, 0.75, 1].map((r) => (
          <div
            key={r}
            className="absolute top-1/2 left-1/2 border border-white/10 rounded-full"
            style={{
              width: `${r * 100}%`,
              height: `${r * 100}%`,
              transform: 'translate(-50%, -50%)'
            }}
          />
        ))}
        
        {/* Light position indicator */}
        <div
          className="absolute w-4 h-4 bg-white rounded-full shadow-lg -translate-x-1/2 -translate-y-1/2 cursor-move"
          style={{
            left: `${x}%`,
            top: `${y}%`
          }}
        />
      </div>

      <div className="grid grid-cols-2 gap-2 text-xs">
        <div className="space-y-0.5">
          <div className="text-white/70">Azimuth</div>
          <div className="text-white">{Math.round(azimuth)}°</div>
        </div>
        <div className="space-y-0.5">
          <div className="text-white/70">Elevation</div>
          <div className="text-white">{Math.round(elevation)}°</div>
        </div>
      </div>

      <div className="text-white/50 text-xs">
        Drag to adjust light direction
      </div>
    </div>
  );
}