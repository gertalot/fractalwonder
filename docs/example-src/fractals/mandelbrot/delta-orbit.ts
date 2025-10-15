// ABOUTME: Fast standard-precision delta orbit calculation for perturbation theory
// ABOUTME: Computes perturbation from reference orbit using native JavaScript doubles

import type { ReferenceOrbit } from "./types";
import { ComplexStd } from "./types";

// Cache for pre-converted reference orbits (high-precision -> standard precision)
// Key is the reference orbit object itself (using WeakMap for automatic cleanup)
const convertedOrbitCache = new WeakMap<ReferenceOrbit, ComplexStd[]>();

/**
 * Pre-convert a reference orbit to standard precision for performance.
 * This is cached internally so repeated calls with the same reference orbit are fast.
 */
function getStandardPrecisionOrbit(referenceOrbit: ReferenceOrbit, maxLength: number): ComplexStd[] {
  let cached = convertedOrbitCache.get(referenceOrbit);
  
  if (!cached || cached.length < maxLength) {
    // Convert to standard precision
    const length = Math.min(referenceOrbit.points.length, maxLength);
    cached = new Array(length);
    for (let i = 0; i < length; i++) {
      const pt = referenceOrbit.points[i];
      cached[i] = { real: pt.real.toNumber(), imag: pt.imag.toNumber() };
    }
    convertedOrbitCache.set(referenceOrbit, cached);
  }
  
  return cached;
}

/**
 * Calculate delta orbit using standard precision arithmetic.
 * 
 * This is the performance-critical component of perturbation theory.
 * Uses native JavaScript doubles (not decimal.js) for speed.
 * 
 * Algorithm:
 * - Δ₀ = ΔC (the offset from reference point)
 * - Δₙ₊₁ = 2·Xₙ·Δₙ + Δₙ² + ΔC
 * - Check escape: |Xₙ + Δₙ|² > 4
 * 
 * Where Xₙ is the reference orbit point (from high-precision calculation).
 * 
 * @param referenceOrbit - Pre-computed high-precision reference orbit
 * @param deltaC - Offset from reference point (in standard precision)
 * @param maxIterations - Maximum iterations to compute
 * @returns Escape iteration (0-based), or -1 if didn't escape
 */
export function calculateDeltaOrbit(
  referenceOrbit: ReferenceOrbit,
  deltaC: ComplexStd,
  maxIterations: number
): number {
  // Get cached standard-precision reference orbit (or convert if not cached)
  const maxRefIteration = referenceOrbit.points.length - 1;
  const xRefArray = getStandardPrecisionOrbit(referenceOrbit, referenceOrbit.points.length);

  // Δ₀ = 0 (starts at zero)
  let deltaReal = 0;
  let deltaImag = 0;
  
  let iteration = 0;
  let refIteration = 0;

  // Iterate up to maxIterations
  while (iteration < maxIterations) {
    // Compute next delta: Δₙ₊₁ = 2·Xₙ·Δₙ + Δₙ² + ΔC
    const xRef = xRefArray[refIteration];
    
    // 2·Xₙ·Δₙ
    const term1Real = 2 * (xRef.real * deltaReal - xRef.imag * deltaImag);
    const term1Imag = 2 * (xRef.real * deltaImag + xRef.imag * deltaReal);

    // Δₙ²
    const term2Real = deltaReal * deltaReal - deltaImag * deltaImag;
    const term2Imag = 2 * deltaReal * deltaImag;

    // Combine: 2·Xₙ·Δₙ + Δₙ² + ΔC
    deltaReal = term1Real + term2Real + deltaC.real;
    deltaImag = term1Imag + term2Imag + deltaC.imag;
    
    refIteration++;
    iteration++;

    // Calculate combined orbit: Z_n = X_n + Δ_n
    const xRefNext = xRefArray[refIteration];
    const zReal = xRefNext.real + deltaReal;
    const zImag = xRefNext.imag + deltaImag;

    // Check escape condition: |Z_n|² > 4
    const magnitudeSq = zReal * zReal + zImag * zImag;
    if (magnitudeSq > 4) {
      return iteration; // Escaped at this iteration
    }

    // Check for rebasing condition: |z| < |dz| OR we've run out of reference orbit
    const zMag = Math.sqrt(magnitudeSq);
    const deltaMag = Math.sqrt(deltaReal * deltaReal + deltaImag * deltaImag);
    
    if (zMag < deltaMag || refIteration >= maxRefIteration) {
      // REBASE: Set delta to current z value and restart from beginning of reference orbit
      deltaReal = zReal;
      deltaImag = zImag;
      refIteration = 0;
    }
  }

  // Didn't escape within maxIterations
  return -1;
}

/**
 * Calculate delta orbit with optional iteration data output.
 * Returns both escape iteration and final delta value.
 * Useful for debugging and visualization.
 */
export function calculateDeltaOrbitWithData(
  referenceOrbit: ReferenceOrbit,
  deltaC: ComplexStd,
  maxIterations: number
): { escapeIteration: number; finalDelta: ComplexStd } {
  // Get cached standard-precision reference orbit (or convert if not cached)
  const maxRefIteration = referenceOrbit.points.length - 1;
  const xRefArray = getStandardPrecisionOrbit(referenceOrbit, referenceOrbit.points.length);

  let deltaReal = 0;
  let deltaImag = 0;
  let escapeIteration = -1;
  
  let iteration = 0;
  let refIteration = 0;

  while (iteration < maxIterations) {
    const xRef = xRefArray[refIteration];
    
    const term1Real = 2 * (xRef.real * deltaReal - xRef.imag * deltaImag);
    const term1Imag = 2 * (xRef.real * deltaImag + xRef.imag * deltaReal);
    const term2Real = deltaReal * deltaReal - deltaImag * deltaImag;
    const term2Imag = 2 * deltaReal * deltaImag;

    deltaReal = term1Real + term2Real + deltaC.real;
    deltaImag = term1Imag + term2Imag + deltaC.imag;
    
    refIteration++;
    iteration++;

    const xRefNext = xRefArray[refIteration];
    const zReal = xRefNext.real + deltaReal;
    const zImag = xRefNext.imag + deltaImag;
    const magnitudeSq = zReal * zReal + zImag * zImag;

    if (magnitudeSq > 4) {
      escapeIteration = iteration;
      break;
    }

    const zMag = Math.sqrt(magnitudeSq);
    const deltaMag = Math.sqrt(deltaReal * deltaReal + deltaImag * deltaImag);
    
    if (zMag < deltaMag || refIteration >= maxRefIteration) {
      deltaReal = zReal;
      deltaImag = zImag;
      refIteration = 0;
    }
  }

  return { escapeIteration, finalDelta: { real: deltaReal, imag: deltaImag } };
}

