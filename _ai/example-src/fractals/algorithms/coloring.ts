/**
 * HSL to RGB conversion utility function.
 *
 * @param h - Hue (0-360 degrees)
 * @param s - Saturation (0-1)
 * @param l - Lightness (0-1)
 * @returns RGB tuple with values 0-255: [red, green, blue]
 */
function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  h = h % 360;
  if (h < 0) h += 360;

  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;

  let r = 0,
    g = 0,
    b = 0;

  if (h >= 0 && h < 60) {
    r = c;
    g = x;
    b = 0;
  } else if (h >= 60 && h < 120) {
    r = x;
    g = c;
    b = 0;
  } else if (h >= 120 && h < 180) {
    r = 0;
    g = c;
    b = x;
  } else if (h >= 180 && h < 240) {
    r = 0;
    g = x;
    b = c;
  } else if (h >= 240 && h < 300) {
    r = x;
    g = 0;
    b = c;
  } else if (h >= 300 && h < 360) {
    r = c;
    g = 0;
    b = x;
  }

  return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
}

/**
 * Smooth looping color scheme using HSL hue cycling.
 *
 * This creates a continuous rainbow effect that loops seamlessly:
 * - Points in the set (iter === maxIterations): Black
 * - Escaped points: Colors cycle through the spectrum based on iteration count
 * - Uses smooth iteration count for continuous gradients
 * - Hue cycles through 0-360 degrees multiple times for rich detail
 *
 * @param iter - Number of iterations before escape
 * @param maxIterations - Maximum possible iterations
 * @returns RGB tuple with values 0-255: [red, green, blue]
 */
export function smoothLoopingColorScheme(iter: number, maxIterations: number): [number, number, number] {
  // Points that never escaped are in the set - render as black
  if (iter === maxIterations) return [0, 0, 0];

  // Calculate smooth iteration count for continuous coloring
  // This reduces banding and creates smoother gradients
  const smoothIter = iter + 1 - Math.log(Math.log(Math.sqrt(2))) / Math.log(2);

  // Normalize to 0-1 range
  const normalized = smoothIter / maxIterations;

  // Cycle through hue spectrum multiple times for rich detail
  // Using 6 cycles gives good detail at extreme zoom levels
  const hue = (normalized * 6 * 360) % 360;

  // High saturation and medium lightness for vibrant colors
  const saturation = 0.9;
  const lightness = 0.5;

  return hslToRgb(hue, saturation, lightness);
}

/**
 * Fire color scheme that transitions from black → red → yellow → white.
 *
 * This classic color scheme creates a dramatic "fire" effect:
 * - Points in the set (iter === maxIterations): Black
 * - Low iterations (0-10%): Dark red to bright red
 * - Mid iterations (10-50%): Red to yellow (adding green)
 * - High iterations (50-100%): Yellow to white (adding blue)
 *
 * @param iter - Number of iterations before escape
 * @param maxIterations - Maximum possible iterations
 * @returns RGB tuple with values 0-255: [red, green, blue]
 */
export function fireColorScheme(iter: number, maxIterations: number): [number, number, number] {
  // Points that never escaped are in the set - render as black
  if (iter === maxIterations) return [0, 0, 0];

  const ratio = iter / maxIterations;

  // Black → Red (0-10%)
  if (ratio < 0.1) {
    const r = Math.round(ratio * 5 * 255); // Scale up to red
    return [r, 0, 0];
  }

  // Red → Yellow (10-50%)
  if (ratio < 0.5) {
    const g = Math.round((ratio - 0.1) * 2.5 * 255); // Add green
    return [255, Math.max(0, g), 0];
  }

  // Yellow → White (50-100%)
  const b = Math.round((ratio - 0.5) * 2 * 255); // Add blue
  return [255, 255, b];
}
