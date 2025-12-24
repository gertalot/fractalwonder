import React, { useState, useEffect, useRef } from 'react';
import { MandelbrotCanvas } from './components/MandelbrotCanvas';
import { BottomControlBar } from './components/BottomControlBar';
import { PaletteEditor } from './components/PaletteEditor';

export interface ColorStop {
  position: number; // 0-1
  color: string; // hex color
}

export interface Palette {
  id: string;
  name: string;
  stops: ColorStop[];
  histogram: boolean;
  smooth: boolean;
  use3D: boolean;
  transferCurve: { x: number; y: number }[];
  falloffCurve: { x: number; y: number }[];
  lighting: {
    ambient: number;
    diffuse: number;
    specular: number;
    shininess: number;
    strength: number;
    azimuth: number;
    elevation: number;
  };
}

const defaultPalette: Palette = {
  id: '1',
  name: 'Deep Blue',
  stops: [
    { position: 0, color: '#000428' },
    { position: 0.5, color: '#004e92' },
    { position: 1, color: '#00d4ff' }
  ],
  histogram: false,
  smooth: true,
  use3D: false,
  transferCurve: [
    { x: 0, y: 0 },
    { x: 1, y: 1 }
  ],
  falloffCurve: [
    { x: 0, y: 1 },
    { x: 1, y: 0 }
  ],
  lighting: {
    ambient: 0.3,
    diffuse: 0.6,
    specular: 0.3,
    shininess: 20,
    strength: 1.0,
    azimuth: 45,
    elevation: 45
  }
};

export default function App() {
  const [showControls, setShowControls] = useState(false);
  const [showPaletteEditor, setShowPaletteEditor] = useState(false);
  const [currentPalette, setCurrentPalette] = useState<Palette>(defaultPalette);
  const [workingPalette, setWorkingPalette] = useState<Palette>(defaultPalette);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const hideControlsTimeout = useRef<NodeJS.Timeout>();

  const handleMouseMove = () => {
    setShowControls(true);
    if (hideControlsTimeout.current) {
      clearTimeout(hideControlsTimeout.current);
    }
    hideControlsTimeout.current = setTimeout(() => {
      setShowControls(false);
    }, 2000);
  };

  const handleFullscreen = () => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen();
      setIsFullscreen(true);
    } else {
      document.exitFullscreen();
      setIsFullscreen(false);
    }
  };

  const handleOpenPaletteEditor = () => {
    setWorkingPalette({ ...currentPalette });
    setShowPaletteEditor(true);
  };

  const handleApplyPalette = () => {
    setCurrentPalette({ ...workingPalette });
    setShowPaletteEditor(false);
  };

  const handleCancelPalette = () => {
    setWorkingPalette({ ...currentPalette });
    setShowPaletteEditor(false);
  };

  useEffect(() => {
    const handleFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement);
    };
    document.addEventListener('fullscreenchange', handleFullscreenChange);
    return () => {
      document.removeEventListener('fullscreenchange', handleFullscreenChange);
    };
  }, []);

  return (
    <div 
      className="w-full h-screen overflow-hidden bg-black relative"
      onMouseMove={handleMouseMove}
    >
      <MandelbrotCanvas palette={currentPalette} />
      
      <BottomControlBar 
        visible={showControls && !showPaletteEditor}
        onOpenPalette={handleOpenPaletteEditor}
        onFullscreen={handleFullscreen}
        isFullscreen={isFullscreen}
      />

      <PaletteEditor
        visible={showPaletteEditor}
        palette={workingPalette}
        onChange={setWorkingPalette}
        onApply={handleApplyPalette}
        onCancel={handleCancelPalette}
      />
    </div>
  );
}
