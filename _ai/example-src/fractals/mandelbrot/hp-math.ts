import { Decimal } from "decimal.js";
import { ComplexHP, ComplexStd } from "./types";

/**
 * Add two high-precision complex numbers.
 * Returns: a + b
 */
export function addHP(a: ComplexHP, b: ComplexHP): ComplexHP {
  return {
    real: a.real.plus(b.real),
    imag: a.imag.plus(b.imag),
  };
}

/**
 * Multiply two high-precision complex numbers.
 * Formula: (a + bi) * (c + di) = (ac - bd) + (ad + bc)i
 * Returns: a * b
 */
export function multiplyHP(a: ComplexHP, b: ComplexHP): ComplexHP {
  // (a.real + a.imag*i) * (b.real + b.imag*i)
  // = a.real*b.real - a.imag*b.imag + (a.real*b.imag + a.imag*b.real)i
  const realPart = a.real.times(b.real).minus(a.imag.times(b.imag));
  const imagPart = a.real.times(b.imag).plus(a.imag.times(b.real));

  return {
    real: realPart,
    imag: imagPart,
  };
}

/**
 * Calculate the squared magnitude of a high-precision complex number.
 * Formula: |z|² = real² + imag²
 * Returns: |z|²
 */
export function magnitudeSquaredHP(z: ComplexHP): Decimal {
  return z.real.times(z.real).plus(z.imag.times(z.imag));
}

/**
 * Convert a high-precision complex number to standard precision.
 * Used when transitioning from reference orbit to delta orbit calculations.
 */
export function hpToStd(z: ComplexHP): ComplexStd {
  return {
    real: z.real.toNumber(),
    imag: z.imag.toNumber(),
  };
}

/**
 * Convert a standard precision complex number to high-precision.
 * Used when converting viewport coordinates to reference point.
 */
export function stdToHP(z: ComplexStd): ComplexHP {
  return {
    real: new Decimal(z.real),
    imag: new Decimal(z.imag),
  };
}

/**
 * Create a high-precision complex number from numeric values.
 * Accepts numbers or strings for precision preservation.
 */
export function createComplexHP(real: number | string, imag: number | string): ComplexHP {
  return {
    real: new Decimal(real),
    imag: new Decimal(imag),
  };
}
