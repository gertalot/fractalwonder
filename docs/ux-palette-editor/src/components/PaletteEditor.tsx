import React, { useState } from 'react';
import { X, Check, Copy, Trash2, ChevronDown, ChevronRight } from 'lucide-react';
import { Palette } from '../App';
import { GradientEditor } from './GradientEditor';
import { CurveEditor } from './CurveEditor';
import { LightingControl } from './LightingControl';

interface PaletteEditorProps {
  visible: boolean;
  palette: Palette;
  onChange: (palette: Palette) => void;
  onApply: () => void;
  onCancel: () => void;
}

export function PaletteEditor({ visible, palette, onChange, onApply, onCancel }: PaletteEditorProps) {
  const [isEditingName, setIsEditingName] = useState(false);
  const [paletteExpanded, setPaletteExpanded] = useState(true);
  const [lightEffectsExpanded, setLightEffectsExpanded] = useState(true);

  const handleNameClick = () => {
    setIsEditingName(true);
  };

  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onChange({ ...palette, name: e.target.value });
  };

  const handleNameBlur = () => {
    setIsEditingName(false);
  };

  return (
    <div 
      className={`fixed top-0 right-0 h-full bg-black/90 backdrop-blur-md border-l border-white/10 transition-transform duration-300 overflow-y-auto ${
        visible ? 'translate-x-0' : 'translate-x-full'
      }`}
      style={{ 
        width: '380px',
        fontFamily: 'system-ui, -apple-system, sans-serif'
      }}
    >
      <div className="p-4 space-y-3">
        {/* Header */}
        <div className="space-y-3">
          {isEditingName ? (
            <input
              type="text"
              value={palette.name}
              onChange={handleNameChange}
              onBlur={handleNameBlur}
              autoFocus
              className="w-full bg-white/5 border border-white/20 rounded-lg px-3 py-1.5 text-white outline-none focus:border-white/40 text-sm"
            />
          ) : (
            <h2 
              onClick={handleNameClick}
              className="text-white cursor-pointer hover:text-gray-200 transition-colors"
            >
              {palette.name}
            </h2>
          )}
          
          <div className="flex gap-2">
            <button
              onClick={onCancel}
              className="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg border border-white/20 text-white hover:text-gray-200 hover:bg-white/10 transition-all text-sm"
            >
              <X className="w-3.5 h-3.5" />
              Cancel
            </button>
            <button
              onClick={onApply}
              className="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg bg-white/20 text-white hover:text-gray-200 hover:bg-white/30 transition-all text-sm"
            >
              <Check className="w-3.5 h-3.5" />
              Apply
            </button>
          </div>
          
          <div className="flex gap-2">
            <button
              className="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg border border-white/10 text-white hover:text-gray-200 hover:bg-white/10 transition-all text-sm"
            >
              <Copy className="w-3.5 h-3.5" />
              Duplicate
            </button>
            <button
              className="flex-1 flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg border border-white/10 text-white hover:text-gray-200 hover:bg-white/10 transition-all text-sm"
            >
              <Trash2 className="w-3.5 h-3.5" />
              Delete
            </button>
          </div>
        </div>

        {/* Palette Section */}
        <div className="border border-white/10 rounded-lg overflow-hidden">
          <button
            onClick={() => setPaletteExpanded(!paletteExpanded)}
            className="w-full flex items-center justify-between px-3 py-2 bg-white/5 hover:bg-white/10 transition-colors text-white text-sm"
          >
            <span>Palette</span>
            {paletteExpanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </button>
          
          {paletteExpanded && (
            <div className="p-3 space-y-3">
              {/* Options */}
              <div className="space-y-1">
                <label className="flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 cursor-pointer transition-colors">
                  <input
                    type="checkbox"
                    checked={palette.histogram}
                    onChange={(e) => onChange({ ...palette, histogram: e.target.checked })}
                    className="w-3.5 h-3.5 rounded accent-white"
                  />
                  <span className="text-white text-sm">Histogram Equalization</span>
                </label>
                
                <label className="flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 cursor-pointer transition-colors">
                  <input
                    type="checkbox"
                    checked={palette.smooth}
                    onChange={(e) => onChange({ ...palette, smooth: e.target.checked })}
                    className="w-3.5 h-3.5 rounded accent-white"
                  />
                  <span className="text-white text-sm">Smooth Coloring</span>
                </label>
              </div>

              {/* Gradient Editor */}
              <div className="space-y-2">
                <div className="text-white text-xs opacity-70 px-1">Color Gradient</div>
                <GradientEditor
                  stops={palette.stops}
                  onChange={(stops) => onChange({ ...palette, stops })}
                />
              </div>

              {/* Transfer Curve */}
              <div className="space-y-2">
                <div className="text-white text-xs opacity-70 px-1">Transfer Curve</div>
                <CurveEditor
                  points={palette.transferCurve}
                  onChange={(points) => onChange({ ...palette, transferCurve: points })}
                  size={320}
                />
              </div>
            </div>
          )}
        </div>

        {/* Light Effects Section */}
        <div className="border border-white/10 rounded-lg overflow-hidden">
          <button
            onClick={() => setLightEffectsExpanded(!lightEffectsExpanded)}
            className="w-full flex items-center justify-between px-3 py-2 bg-white/5 hover:bg-white/10 transition-colors text-white text-sm"
          >
            <span>Light Effects</span>
            {lightEffectsExpanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
          </button>
          
          {lightEffectsExpanded && (
            <div className="p-3 space-y-3">
              {/* 3D Toggle */}
              <label className="flex items-center gap-2 px-2 py-1 rounded hover:bg-white/5 cursor-pointer transition-colors">
                <input
                  type="checkbox"
                  checked={palette.use3D}
                  onChange={(e) => onChange({ ...palette, use3D: e.target.checked })}
                  className="w-3.5 h-3.5 rounded accent-white"
                />
                <span className="text-white text-sm">3D Lighting</span>
              </label>

              {/* 3D Options */}
              {palette.use3D && (
                <>
                  {/* Falloff Curve */}
                  <div className="space-y-2">
                    <div className="text-white text-xs opacity-70 px-1">3D Falloff Curve</div>
                    <CurveEditor
                      points={palette.falloffCurve}
                      onChange={(points) => onChange({ ...palette, falloffCurve: points })}
                      size={320}
                    />
                  </div>

                  {/* Lighting Parameters */}
                  <div className="space-y-2">
                    <div className="text-white text-xs opacity-70 px-1">Lighting Parameters</div>
                    <div className="space-y-2">
                      <div className="flex items-center gap-2">
                        <div className="text-white text-xs w-20">Ambient</div>
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.01"
                          value={palette.lighting.ambient}
                          onChange={(e) => onChange({
                            ...palette,
                            lighting: { ...palette.lighting, ambient: parseFloat(e.target.value) }
                          })}
                          className="flex-1 accent-white"
                        />
                        <div className="text-white text-xs w-10 text-right">{palette.lighting.ambient.toFixed(2)}</div>
                      </div>

                      <div className="flex items-center gap-2">
                        <div className="text-white text-xs w-20">Diffuse</div>
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.01"
                          value={palette.lighting.diffuse}
                          onChange={(e) => onChange({
                            ...palette,
                            lighting: { ...palette.lighting, diffuse: parseFloat(e.target.value) }
                          })}
                          className="flex-1 accent-white"
                        />
                        <div className="text-white text-xs w-10 text-right">{palette.lighting.diffuse.toFixed(2)}</div>
                      </div>

                      <div className="flex items-center gap-2">
                        <div className="text-white text-xs w-20">Specular</div>
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.01"
                          value={palette.lighting.specular}
                          onChange={(e) => onChange({
                            ...palette,
                            lighting: { ...palette.lighting, specular: parseFloat(e.target.value) }
                          })}
                          className="flex-1 accent-white"
                        />
                        <div className="text-white text-xs w-10 text-right">{palette.lighting.specular.toFixed(2)}</div>
                      </div>

                      <div className="flex items-center gap-2">
                        <div className="text-white text-xs w-20">Shininess</div>
                        <input
                          type="range"
                          min="1"
                          max="128"
                          step="1"
                          value={palette.lighting.shininess}
                          onChange={(e) => onChange({
                            ...palette,
                            lighting: { ...palette.lighting, shininess: parseFloat(e.target.value) }
                          })}
                          className="flex-1 accent-white"
                        />
                        <div className="text-white text-xs w-10 text-right">{palette.lighting.shininess.toFixed(0)}</div>
                      </div>

                      <div className="flex items-center gap-2">
                        <div className="text-white text-xs w-20">Strength</div>
                        <input
                          type="range"
                          min="0"
                          max="2"
                          step="0.01"
                          value={palette.lighting.strength}
                          onChange={(e) => onChange({
                            ...palette,
                            lighting: { ...palette.lighting, strength: parseFloat(e.target.value) }
                          })}
                          className="flex-1 accent-white"
                        />
                        <div className="text-white text-xs w-10 text-right">{palette.lighting.strength.toFixed(2)}</div>
                      </div>
                    </div>
                  </div>

                  {/* Light Direction */}
                  <div className="space-y-2">
                    <div className="text-white text-xs opacity-70 px-1">Light Direction</div>
                    <LightingControl
                      azimuth={palette.lighting.azimuth}
                      elevation={palette.lighting.elevation}
                      onChange={(azimuth, elevation) => onChange({
                        ...palette,
                        lighting: { ...palette.lighting, azimuth, elevation }
                      })}
                    />
                  </div>
                </>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
