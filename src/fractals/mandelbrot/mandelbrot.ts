import { derivedRealIterations, FractalParams } from "@/hooks/use-store";
import canvasSize from "@/lib/canvas-size";
import { pixelToFractalCoordinate } from "@/lib/coordinates";

interface FractalIterationResult {
  iter: number;
  zr: number;
  zi: number;
}

function firePalette(iter: number, maxIterations: number) {
  if (iter === maxIterations) return [0, 0, 0];

  const ratio = iter / maxIterations;
  // From black to red to yellow to white
  if (ratio < 0.1) {
    const r = Math.round(ratio * 5 * 255);
    return [r, 0, 0];
  } else if (ratio < 0.5) {
    const g = Math.round((ratio - 0.2) * 3.33 * 255);
    return [255, g, 0];
  } else {
    const b = Math.round((ratio - 0.5) * 2 * 255);
    const r = 255;
    const g = 255;
    return [r, g, b];
  }
}

function compute(real: number, imag: number, maxIterations: number): FractalIterationResult {
  // Calculate Mandelbrot set iteration count
  let zr = 0;
  let zi = 0;
  let iter = 0;

  while (zr * zr + zi * zi < 4 && iter < maxIterations) {
    const newZr = zr * zr - zi * zi + real;
    zi = 2 * zr * zi + imag;
    zr = newZr;
    iter++;
  }
  return { iter, zr, zi };
}
function mandelbrot(canvas: HTMLCanvasElement, params: FractalParams) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const { left, top, width, height } = {
    left: 0,
    top: 0,
    ...canvasSize(canvas),
  };

  const buffer = new Uint8ClampedArray(width * height * 4); // 4 bytes per pixel for RGBA

  for (let x = 0; x < width; x++) {
    for (let y = 0; y < height; y++) {
      const { x: real, y: imag } = pixelToFractalCoordinate(
        { x: left + x, y: top + y },
        width,
        height,
        params.center,
        params.zoom
      );

      const maxIter = derivedRealIterations(params);

      const { iter } = compute(real, imag, maxIter);
      const [r, g, b] = firePalette(iter, maxIter);
      const index = (y * width + x) * 4;
      buffer[index] = r;
      buffer[index + 1] = g;
      buffer[index + 2] = b;
      buffer[index + 3] = 255;
    }
  }

  ctx.putImageData(new ImageData(buffer, width, height), left, top);
}

export default mandelbrot;
