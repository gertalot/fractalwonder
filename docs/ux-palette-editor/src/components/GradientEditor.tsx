import React, { useState, useRef } from 'react';
import { ZoomIn, ZoomOut } from 'lucide-react';
import { ColorStop } from '../App';

interface GradientEditorProps {
  stops: ColorStop[];
  onChange: (stops: ColorStop[]) => void;
}

export function GradientEditor({ stops, onChange }: GradientEditorProps) {
  const [selectedStopIndex, setSelectedStopIndex] = useState<number | null>(null);
  const [showColorPicker, setShowColorPicker] = useState(false);
  const [midpoints, setMidpoints] = useState<{ [key: string]: number }>({});
  const [zoom, setZoom] = useState(1);
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  const handleBarClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const position = Math.max(0, Math.min(1, x / rect.width));
    
    // Add new stop
    const newStops = [...stops, { position, color: '#ffffff' }];
    onChange(newStops.sort((a, b) => a.position - b.position));
  };

  const handleStopClick = (e: React.MouseEvent, index: number) => {
    e.stopPropagation();
    setSelectedStopIndex(index);
    setShowColorPicker(true);
  };

  const handleStopDrag = (index: number, e: React.MouseEvent) => {
    e.preventDefault();
    const containerRect = containerRef.current?.getBoundingClientRect();
    if (!containerRect) return;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const x = moveEvent.clientX - containerRect.left;
      const position = Math.max(0, Math.min(1, x / containerRect.width));
      
      const newStops = [...stops];
      newStops[index] = { ...newStops[index], position };
      onChange(newStops);
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  const handleMidpointDrag = (leftIndex: number, e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const containerRect = containerRef.current?.getBoundingClientRect();
    if (!containerRect) return;

    const sortedStops = [...stops].sort((a, b) => a.position - b.position);
    const leftPos = sortedStops[leftIndex].position;
    const rightPos = sortedStops[leftIndex + 1].position;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const x = moveEvent.clientX - containerRect.left;
      const position = x / containerRect.width;
      
      // Constrain midpoint between the two stops
      const constrainedPos = Math.max(leftPos + 0.01, Math.min(rightPos - 0.01, position));
      
      // Calculate midpoint value (0-1 between the two stops)
      const midpointValue = (constrainedPos - leftPos) / (rightPos - leftPos);
      
      const key = `${leftIndex}`;
      setMidpoints(prev => ({ ...prev, [key]: midpointValue }));
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  const handleColorChange = (color: string) => {
    if (selectedStopIndex === null) return;
    const newStops = [...stops];
    newStops[selectedStopIndex] = { ...newStops[selectedStopIndex], color };
    onChange(newStops);
  };

  const handleDeleteStop = (index: number) => {
    if (stops.length <= 2) return; // Keep at least 2 stops
    const newStops = stops.filter((_, i) => i !== index);
    onChange(newStops);
    setSelectedStopIndex(null);
    setShowColorPicker(false);
  };

  const handleWheel = (e: React.WheelEvent) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      const delta = e.deltaY > 0 ? 0.9 : 1.1;
      setZoom(prev => Math.max(1, Math.min(10, prev * delta)));
    }
  };

  const handleZoomIn = () => {
    setZoom(prev => Math.min(10, prev * 1.2));
  };

  const handleZoomOut = () => {
    setZoom(prev => Math.max(1, prev / 1.2));
  };

  const renderGradient = () => {
    const sortedStops = [...stops].sort((a, b) => a.position - b.position);
    const gradientStops = sortedStops.map(stop => 
      `${stop.color} ${stop.position * 100}%`
    ).join(', ');
    return `linear-gradient(to right, ${gradientStops})`;
  };

  const getMidpointPosition = (leftIndex: number): number => {
    const sortedStops = [...stops].sort((a, b) => a.position - b.position);
    if (leftIndex >= sortedStops.length - 1) return 0;
    
    const leftPos = sortedStops[leftIndex].position;
    const rightPos = sortedStops[leftIndex + 1].position;
    const key = `${leftIndex}`;
    const midpointValue = midpoints[key] ?? 0.5;
    
    return leftPos + (rightPos - leftPos) * midpointValue;
  };

  const sortedStops = [...stops].sort((a, b) => a.position - b.position);
  const baseWidth = 320; // base width matches the panel
  const scaledWidth = baseWidth * zoom;

  return (
    <div className="space-y-2">
      {/* Zoom controls */}
      <div className="flex items-center justify-between px-1">
        <div className="text-white/50 text-xs">
          {zoom > 1 ? `Zoom: ${zoom.toFixed(1)}x` : 'Scroll to pan'}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={handleZoomOut}
            disabled={zoom <= 1}
            className="p-1 rounded hover:bg-white/10 text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            title="Zoom out (Ctrl + scroll)"
          >
            <ZoomOut className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={handleZoomIn}
            disabled={zoom >= 10}
            className="p-1 rounded hover:bg-white/10 text-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            title="Zoom in (Ctrl + scroll)"
          >
            <ZoomIn className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Scrollable gradient container */}
      <div 
        ref={scrollContainerRef}
        className="overflow-x-auto overflow-y-visible"
        onWheel={handleWheel}
      >
        <div className="relative" style={{ width: `${scaledWidth}px` }}>
          {/* Color stops above gradient */}
          <div className="relative h-6 mb-1">
            {stops.map((stop, index) => (
              <div
                key={index}
                className={`absolute top-0 w-3 h-3 cursor-move transition-all ${
                  selectedStopIndex === index ? 'ring-1 ring-white' : ''
                }`}
                style={{
                  left: `${stop.position * 100}%`,
                  transform: 'translateX(-50%)',
                  backgroundColor: stop.color,
                  border: '1px solid rgba(255, 255, 255, 0.3)'
                }}
                onClick={(e) => handleStopClick(e, index)}
                onMouseDown={(e) => handleStopDrag(index, e)}
              />
            ))}
            
            {/* Midpoint diamonds */}
            {sortedStops.slice(0, -1).map((_, leftIndex) => {
              const midPos = getMidpointPosition(leftIndex);
              return (
                <div
                  key={`mid-${leftIndex}`}
                  className="absolute top-0 w-2.5 h-2.5 bg-white/80 cursor-ew-resize border border-white/50"
                  style={{
                    left: `${midPos * 100}%`,
                    transform: 'translateX(-50%) rotate(45deg)',
                    marginTop: '1px'
                  }}
                  onMouseDown={(e) => handleMidpointDrag(leftIndex, e)}
                />
              );
            })}
          </div>
          
          {/* Gradient bar */}
          <div 
            ref={containerRef}
            className="relative h-8 rounded cursor-crosshair border border-white/20"
            style={{ background: renderGradient() }}
            onClick={handleBarClick}
          />
        </div>
      </div>

      {showColorPicker && selectedStopIndex !== null && (
        <div className="bg-white/5 border border-white/10 rounded p-2 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-white text-xs">Color Stop</span>
            <button
              onClick={() => handleDeleteStop(selectedStopIndex)}
              className="text-white/70 hover:text-white text-xs hover:bg-white/10 px-2 py-0.5 rounded transition-colors"
              disabled={stops.length <= 2}
            >
              Delete
            </button>
          </div>
          <div className="flex items-center gap-2">
            <input
              type="color"
              value={stops[selectedStopIndex].color}
              onChange={(e) => handleColorChange(e.target.value)}
              className="w-12 h-8 rounded cursor-pointer bg-transparent"
            />
            <input
              type="text"
              value={stops[selectedStopIndex].color}
              onChange={(e) => handleColorChange(e.target.value)}
              className="flex-1 bg-white/5 border border-white/20 rounded px-2 py-1 text-white text-xs outline-none focus:border-white/40"
            />
          </div>
          <div className="space-y-1">
            <div className="flex justify-between text-white text-xs">
              <span>Position</span>
              <span>{(stops[selectedStopIndex].position * 100).toFixed(1)}%</span>
            </div>
            <input
              type="range"
              min="0"
              max="1"
              step="0.001"
              value={stops[selectedStopIndex].position}
              onChange={(e) => {
                const newStops = [...stops];
                newStops[selectedStopIndex] = { 
                  ...newStops[selectedStopIndex], 
                  position: parseFloat(e.target.value) 
                };
                onChange(newStops);
              }}
              className="w-full accent-white"
            />
          </div>
        </div>
      )}

      <div className="text-white/50 text-xs px-1">
        Click gradient to add stops • Ctrl+scroll to zoom
      </div>
    </div>
  );
}
