import React from 'react';
import { Home, Palette, Settings, Info, Maximize, Minimize } from 'lucide-react';

interface BottomControlBarProps {
  visible: boolean;
  onOpenPalette: () => void;
  onFullscreen: () => void;
  isFullscreen: boolean;
}

export function BottomControlBar({ visible, onOpenPalette, onFullscreen, isFullscreen }: BottomControlBarProps) {
  return (
    <div 
      className={`absolute bottom-0 left-0 right-0 bg-black/70 backdrop-blur-sm transition-opacity duration-300 ${
        visible ? 'opacity-100' : 'opacity-0 pointer-events-none'
      }`}
      style={{ fontFamily: 'system-ui, -apple-system, sans-serif' }}
    >
      <div className="flex items-center justify-between px-6 py-3">
        <div className="flex items-center gap-4">
          <button 
            className="p-1.5 rounded-lg text-white hover:text-gray-200 hover:bg-white/10 transition-all"
            title="Info"
          >
            <Info className="w-4 h-4" />
          </button>
          
          <button 
            className="p-1.5 rounded-lg text-white hover:text-gray-200 hover:bg-white/10 transition-all"
            title="Home"
          >
            <Home className="w-4 h-4" />
          </button>
          
          <button 
            onClick={onOpenPalette}
            className="p-1.5 rounded-lg text-white hover:text-gray-200 hover:bg-white/10 transition-all"
            title="Color Palette"
          >
            <Palette className="w-4 h-4" />
          </button>
          
          <button 
            className="p-1.5 rounded-lg text-white hover:text-gray-200 hover:bg-white/10 transition-all"
            title="Settings"
          >
            <Settings className="w-4 h-4" />
          </button>
        </div>
        
        <div className="text-white text-sm">
          <span className="opacity-70">Ready</span>
        </div>
        
        <button 
          onClick={onFullscreen}
          className="p-2 rounded-lg text-white hover:text-gray-200 hover:bg-white/10 transition-all"
          title={isFullscreen ? "Exit Fullscreen" : "Fullscreen"}
        >
          {isFullscreen ? <Minimize className="w-5 h-5" /> : <Maximize className="w-5 h-5" />}
        </button>
      </div>
    </div>
  );
}