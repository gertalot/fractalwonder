// ABOUTME: Single-threaded Mandelbrot rendering (legacy, being replaced by parallel renderer)
// ABOUTME: Uses the new modular algorithm system but renders synchronously on main thread

import { fireColorScheme } from "@/fractals/algorithms/coloring";
import { mandelbrotAlgorithm } from "@/fractals/algorithms/mandelbrot";
import { derivedRealIterations, FractalParams } from "@/hooks/use-store";
import canvasSize from "@/lib/canvas-size";
import { pixelToFractalCoordinate } from "@/lib/coordinates";
function computeMandelbrot(canvas: HTMLCanvasElement, params: FractalParams) {
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
      const fractalCoord = pixelToFractalCoordinate(
        { x: left + x, y: top + y },
        width,
        height,
        { x: params.center.x.toNumber(), y: params.center.y.toNumber() },
        params.zoom
      );
      const real = fractalCoord.x;
      const imag = fractalCoord.y;

      const maxIter = derivedRealIterations(params);

      const { iter } = mandelbrotAlgorithm.computePoint(real, imag, maxIter);
      const [r, g, b] = fireColorScheme(iter, maxIter);
      const index = (y * width + x) * 4;
      buffer[index] = r;
      buffer[index + 1] = g;
      buffer[index + 2] = b;
      buffer[index + 3] = 255;
    }
  }

  ctx.putImageData(new ImageData(buffer, width, height), left, top);
}

export default computeMandelbrot;
