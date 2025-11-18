import { Decimal } from "decimal.js";
import type { Point } from "../../fractals/types";
import { createComplexHP } from "./hp-math";
import { ComplexHP } from "./types";

/**
 * Calculate required precision based on zoom level.
 *
 * Empirical formula from research:
 * decimal_places = max(30, ceil(log10(zoom) * 2.5 + 20))
 *
 * This ensures sufficient precision in the reference orbit to maintain
 * sub-pixel accuracy in delta calculations.
 *
 * Examples:
 * - Zoom 1: 30 decimal places (minimum)
 * - Zoom 10^15: ~60 decimal places
 * - Zoom 10^30: ~100 decimal places
 * - Zoom 10^100: ~270 decimal places
 */
export function calculateRequiredPrecision(zoom: Decimal): number {
  if (zoom.lte(1)) {
    return 30; // Minimum precision
  }
  const logZoom = zoom.log(10).toNumber();
  return Math.max(30, Math.ceil(logZoom * 2.5 + 20));
}

/**
 * Convert a Point (standard precision) to ComplexHP (high-precision).
 *
 * Uses strings to preserve maximum precision from the number representation.
 * Configures Decimal.js precision based on zoom level if provided.
 *
 * @param point - Standard precision Point {x, y}
 * @param zoom - Optional zoom level for automatic precision configuration
 * @returns High-precision complex number
 */
export function pointToHP(point: Point, zoom?: Decimal): ComplexHP {
  // Configure precision if zoom provided
  if (zoom !== undefined) {
    const requiredPrecision = calculateRequiredPrecision(zoom);
    Decimal.set({ precision: requiredPrecision });
  }

  // Convert using string representation to preserve precision
  return createComplexHP(point.x.toString(), point.y.toString());
}

/**
 * Convert FractalParams center to high-precision complex number.
 * Automatically configures precision based on zoom level.
 *
 * @param center - Center point from FractalParams
 * @param zoom - Zoom level from FractalParams
 * @returns High-precision complex number with appropriate precision
 */
export function centerToHP(center: Point, zoom: Decimal): ComplexHP {
  return pointToHP(center, zoom);
}
