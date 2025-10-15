import { Decimal } from "decimal.js";
import { FractalInterface, MandelbrotParams } from "../types";

export const mandelbrot: FractalInterface<MandelbrotParams> = {
  type: "mandelbrot",
  name: "Mandelbrot",
  description: "(tbd)",

  defaultParameters: {
    type: "mandelbrot",
    center: { x: new Decimal(-1), y: new Decimal(0) },
    zoom: new Decimal(1),
    maxIterations: 250,
    iterationScalingFactor: 1000,
  },
};

export default mandelbrot;
