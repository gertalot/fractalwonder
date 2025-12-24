import React, { useEffect, useRef, useState } from 'react';
import { Palette, ColorStop } from '../App';

interface MandelbrotCanvasProps {
  palette: Palette;
}

export function MandelbrotCanvas({ palette }: MandelbrotCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [viewport, setViewport] = useState({
    centerX: -0.5,
    centerY: 0,
    scale: 3.5
  });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [renderInfo, setRenderInfo] = useState({ iterations: 0, time: 0 });

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const handleResize = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
      renderMandelbrot();
    };

    handleResize();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  useEffect(() => {
    renderMandelbrot();
  }, [palette, viewport]);

  const hexToRgb = (hex: string): [number, number, number] => {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? [
      parseInt(result[1], 16),
      parseInt(result[2], 16),
      parseInt(result[3], 16)
    ] : [0, 0, 0];
  };

  const interpolateColor = (stops: ColorStop[], t: number): [number, number, number] => {
    if (stops.length === 0) return [0, 0, 0];
    if (stops.length === 1) return hexToRgb(stops[0].color);
    
    const sorted = [...stops].sort((a, b) => a.position - b.position);
    
    if (t <= sorted[0].position) return hexToRgb(sorted[0].color);
    if (t >= sorted[sorted.length - 1].position) return hexToRgb(sorted[sorted.length - 1].color);
    
    for (let i = 0; i < sorted.length - 1; i++) {
      if (t >= sorted[i].position && t <= sorted[i + 1].position) {
        const t0 = sorted[i].position;
        const t1 = sorted[i + 1].position;
        const local = (t - t0) / (t1 - t0);
        
        const c0 = hexToRgb(sorted[i].color);
        const c1 = hexToRgb(sorted[i + 1].color);
        
        return [
          Math.round(c0[0] + (c1[0] - c0[0]) * local),
          Math.round(c0[1] + (c1[1] - c0[1]) * local),
          Math.round(c0[2] + (c1[2] - c0[2]) * local)
        ];
      }
    }
    
    return hexToRgb(sorted[0].color);
  };

  const applyCurve = (value: number, curve: { x: number; y: number }[]): number => {
    if (curve.length < 2) return value;
    
    const sorted = [...curve].sort((a, b) => a.x - b.x);
    
    if (value <= sorted[0].x) return sorted[0].y;
    if (value >= sorted[sorted.length - 1].x) return sorted[sorted.length - 1].y;
    
    for (let i = 0; i < sorted.length - 1; i++) {
      if (value >= sorted[i].x && value <= sorted[i + 1].x) {
        const t = (value - sorted[i].x) / (sorted[i + 1].x - sorted[i].x);
        return sorted[i].y + (sorted[i + 1].y - sorted[i].y) * t;
      }
    }
    
    return value;
  };

  const renderMandelbrot = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    
    // Check for valid dimensions
    if (width <= 0 || height <= 0) return;
    
    const imageData = ctx.createImageData(width, height);
    const data = imageData.data;

    const maxIterations = 256;
    const startTime = performance.now();

    for (let py = 0; py < height; py++) {
      for (let px = 0; px < width; px++) {
        const x0 = viewport.centerX + (px - width / 2) * (viewport.scale / width);
        const y0 = viewport.centerY + (py - height / 2) * (viewport.scale / height);

        let x = 0;
        let y = 0;
        let iteration = 0;
        let xtemp;

        while (x * x + y * y <= 4 && iteration < maxIterations) {
          xtemp = x * x - y * y + x0;
          y = 2 * x * y + y0;
          x = xtemp;
          iteration++;
        }

        const pixelIndex = (py * width + px) * 4;

        if (iteration === maxIterations) {
          data[pixelIndex] = 0;
          data[pixelIndex + 1] = 0;
          data[pixelIndex + 2] = 0;
          data[pixelIndex + 3] = 255;
        } else {
          let value = iteration / maxIterations;
          
          if (palette.smooth) {
            const log_zn = Math.log(x * x + y * y) / 2;
            const nu = Math.log(log_zn / Math.log(2)) / Math.log(2);
            value = (iteration + 1 - nu) / maxIterations;
          }

          // Apply transfer curve
          value = applyCurve(value, palette.transferCurve);

          const [r, g, b] = interpolateColor(palette.stops, value);
          
          data[pixelIndex] = r;
          data[pixelIndex + 1] = g;
          data[pixelIndex + 2] = b;
          data[pixelIndex + 3] = 255;
        }
      }
    }

    ctx.putImageData(imageData, 0, 0);
    const endTime = performance.now();
    setRenderInfo({ iterations: maxIterations, time: Math.round(endTime - startTime) });
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 1.1 : 0.9;
    setViewport(v => ({ ...v, scale: v.scale * zoomFactor }));
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    setIsDragging(true);
    setDragStart({ x: e.clientX, y: e.clientY });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging || !canvasRef.current) return;
    
    const dx = e.clientX - dragStart.x;
    const dy = e.clientY - dragStart.y;
    
    setViewport(v => ({
      ...v,
      centerX: v.centerX - (dx * v.scale) / canvasRef.current!.width,
      centerY: v.centerY - (dy * v.scale) / canvasRef.current!.height
    }));
    
    setDragStart({ x: e.clientX, y: e.clientY });
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0 cursor-move"
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    />
  );
}