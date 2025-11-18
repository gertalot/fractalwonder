import { Decimal } from "decimal.js";
import { addHP, createComplexHP, magnitudeSquaredHP, multiplyHP } from "./hp-math";
import { ComplexHP, ReferenceOrbit } from "./types";

/**
 * Cache for reference orbits to avoid recalculation.
 * Key format: "real_imag_maxIterations"
 */
const orbitCache = new Map<string, ReferenceOrbit>();

/**
 * Generate cache key for a reference orbit.
 */
function getCacheKey(center: ComplexHP, maxIterations: number): string {
  return `${center.real.toString()}_${center.imag.toString()}_${maxIterations}`;
}

/**
 * Calculate high-precision reference orbit for perturbation theory.
 *
 * Iterates: Z₀ = 0, Zₙ₊₁ = Zₙ² + center
 * until |Z|² > 4 (escaped) or n = maxIterations (in set)
 *
 * This is the computationally expensive part of perturbation theory,
 * but it only needs to be calculated once per render for the reference point.
 *
 * @param center - High-precision complex number (typically viewport center)
 * @param maxIterations - Maximum number of iterations to compute
 * @returns Reference orbit containing all Z_n values and escape information
 */
export function calculateReferenceOrbit(center: ComplexHP, maxIterations: number): ReferenceOrbit {
  const cacheKey = getCacheKey(center, maxIterations);

  // Check cache first
  const cached = orbitCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  const points: ComplexHP[] = [];
  const escapeThreshold = new Decimal(4);

  // Z₀ = 0
  let z: ComplexHP = createComplexHP(0, 0);
  points.push(z);

  let escapeIteration = -1;

  // Iterate: Zₙ₊₁ = Zₙ² + center
  for (let n = 0; n < maxIterations; n++) {
    // Check escape condition: |Z|² > 4
    const magnitudeSq = magnitudeSquaredHP(z);
    if (magnitudeSq.greaterThan(escapeThreshold)) {
      escapeIteration = n;
      break;
    }

    // Zₙ₊₁ = Zₙ² + center
    z = addHP(multiplyHP(z, z), center);
    points.push(z);
  }

  const orbit: ReferenceOrbit = {
    points,
    escapeIteration,
    inSet: escapeIteration === -1,
  };

  // Cache the result
  orbitCache.set(cacheKey, orbit);

  return orbit;
}

/**
 * Clear the reference orbit cache.
 * Useful for memory management or when precision requirements change.
 */
export function clearOrbitCache(): void {
  orbitCache.clear();
}

/**
 * Get current cache statistics.
 * Useful for performance monitoring and debugging.
 */
export function getOrbitCacheStats(): { size: number; keys: string[] } {
  return {
    size: orbitCache.size,
    keys: Array.from(orbitCache.keys()),
  };
}
