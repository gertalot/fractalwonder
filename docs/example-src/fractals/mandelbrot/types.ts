// ABOUTME: High-precision type definitions for perturbation theory implementation
// ABOUTME: Defines complex numbers and reference orbit structures using decimal.js

import { Decimal } from "decimal.js";

/**
 * High-precision complex number using decimal.js for arbitrary precision arithmetic.
 * Used for reference orbit calculations at deep zoom levels.
 */
export type ComplexHP = {
  real: Decimal;
  imag: Decimal;
};

/**
 * Standard precision complex number using JavaScript's native number type.
 * Used for fast delta orbit calculations.
 */
export type ComplexStd = {
  real: number;
  imag: number;
};

/**
 * Reference orbit data structure.
 * Contains the high-precision orbit sequence for the reference point (typically viewport center).
 * Each element is a ComplexHP representing X_n in the perturbation theory formula.
 */
export type ReferenceOrbit = {
  /** Array of orbit points (X_0, X_1, X_2, ..., X_n) */
  points: ComplexHP[];
  /** Iteration at which the orbit escaped, or -1 if it did not escape */
  escapeIteration: number;
  /** Whether the orbit reached maxIterations without escaping (in the set) */
  inSet: boolean;
};

