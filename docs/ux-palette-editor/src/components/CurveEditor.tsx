import React, { useRef, useEffect, useState } from 'react';

interface CurveEditorProps {
  points: { x: number; y: number }[];
  onChange: (points: { x: number; y: number }[]) => void;
  size: number;
}

export function CurveEditor({ points, onChange, size }: CurveEditorProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [draggingIndex, setDraggingIndex] = useState<number | null>(null);
  const [hoverIndex, setHoverIndex] = useState<number | null>(null);

  useEffect(() => {
    drawCurve();
  }, [points, hoverIndex]);

  const drawCurve = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear canvas
    ctx.clearRect(0, 0, size, size);

    // Draw grid
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
      const x = (i / 4) * size;
      const y = (i / 4) * size;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, size);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(size, y);
      ctx.stroke();
    }

    // Draw diagonal reference line
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(0, size);
    ctx.lineTo(size, 0);
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw curve
    const sortedPoints = [...points].sort((a, b) => a.x - b.x);
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.8)';
    ctx.lineWidth = 2;
    ctx.beginPath();
    sortedPoints.forEach((point, i) => {
      const x = point.x * size;
      const y = (1 - point.y) * size;
      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();

    // Draw points
    points.forEach((point, i) => {
      const x = point.x * size;
      const y = (1 - point.y) * size;
      
      ctx.fillStyle = hoverIndex === i ? 'rgba(255, 255, 255, 1)' : 'rgba(255, 255, 255, 0.9)';
      ctx.beginPath();
      ctx.arc(x, y, hoverIndex === i ? 6 : 5, 0, Math.PI * 2);
      ctx.fill();
      
      ctx.strokeStyle = 'rgba(0, 0, 0, 0.5)';
      ctx.lineWidth = 2;
      ctx.stroke();
    });
  };

  const getPointAtPosition = (clientX: number, clientY: number): number | null => {
    const canvas = canvasRef.current;
    if (!canvas) return null;

    const rect = canvas.getBoundingClientRect();
    const x = ((clientX - rect.left) / rect.width) * size;
    const y = ((clientY - rect.top) / rect.height) * size;

    for (let i = 0; i < points.length; i++) {
      const px = points[i].x * size;
      const py = (1 - points[i].y) * size;
      const distance = Math.sqrt((x - px) ** 2 + (y - py) ** 2);
      if (distance < 10) {
        return i;
      }
    }
    return null;
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    const pointIndex = getPointAtPosition(e.clientX, e.clientY);
    
    if (pointIndex !== null) {
      // Start dragging existing point
      setDraggingIndex(pointIndex);
    } else {
      // Add new point
      const canvas = canvasRef.current;
      if (!canvas) return;
      
      const rect = canvas.getBoundingClientRect();
      const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const y = Math.max(0, Math.min(1, 1 - (e.clientY - rect.top) / rect.height));
      
      const newPoints = [...points, { x, y }].sort((a, b) => a.x - b.x);
      onChange(newPoints);
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (draggingIndex !== null) {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const rect = canvas.getBoundingClientRect();
      let x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const y = Math.max(0, Math.min(1, 1 - (e.clientY - rect.top) / rect.height));

      // Constrain first and last points to x boundaries
      if (draggingIndex === 0) x = 0;
      if (draggingIndex === points.length - 1) x = 1;

      const newPoints = [...points];
      newPoints[draggingIndex] = { x, y };
      onChange(newPoints);
    } else {
      // Update hover state
      const pointIndex = getPointAtPosition(e.clientX, e.clientY);
      setHoverIndex(pointIndex);
    }
  };

  const handleMouseUp = () => {
    setDraggingIndex(null);
  };

  const handleMouseLeave = () => {
    setDraggingIndex(null);
    setHoverIndex(null);
  };

  const handleDoubleClick = (e: React.MouseEvent) => {
    const pointIndex = getPointAtPosition(e.clientX, e.clientY);
    if (pointIndex !== null && points.length > 2) {
      // Don't delete first or last point
      if (pointIndex !== 0 && pointIndex !== points.length - 1) {
        const newPoints = points.filter((_, i) => i !== pointIndex);
        onChange(newPoints);
      }
    }
  };

  return (
    <div className="bg-white/5 border border-white/10 rounded-lg p-4 space-y-2">
      <canvas
        ref={canvasRef}
        width={size}
        height={size}
        className="cursor-crosshair rounded"
        style={{ width: '100%', height: 'auto' }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        onDoubleClick={handleDoubleClick}
      />
      <div className="text-white/50 text-xs">
        Click to add points • Drag to move • Double-click to remove
      </div>
    </div>
  );
}