import { Decimal } from "decimal.js";
import { describe, expect, it } from "vitest";
import { addHP, createComplexHP, hpToStd, magnitudeSquaredHP, multiplyHP, stdToHP } from "./hp-math";
import { ComplexHP } from "./types";

describe("hp-math: High-Precision Complex Arithmetic", () => {
  describe("addHP", () => {
    it("should add two complex numbers correctly: (1+i) + (2+3i) = (3+4i)", () => {
      const a: ComplexHP = createComplexHP(1, 1);
      const b: ComplexHP = createComplexHP(2, 3);
      const result = addHP(a, b);

      expect(result.real.toNumber()).toBe(3);
      expect(result.imag.toNumber()).toBe(4);
    });

    it("should handle negative numbers: (-2+3i) + (5-7i) = (3-4i)", () => {
      const a: ComplexHP = createComplexHP(-2, 3);
      const b: ComplexHP = createComplexHP(5, -7);
      const result = addHP(a, b);

      expect(result.real.toNumber()).toBe(3);
      expect(result.imag.toNumber()).toBe(-4);
    });

    it("should maintain high precision for very small numbers", () => {
      const a: ComplexHP = createComplexHP("1e-50", "2e-50");
      const b: ComplexHP = createComplexHP("3e-50", "4e-50");
      const result = addHP(a, b);

      expect(result.real.toString()).toBe("4e-50");
      expect(result.imag.toString()).toBe("6e-50");
    });
  });

  describe("multiplyHP", () => {
    it("should multiply two complex numbers correctly: (2+i)*(3+4i) = (2+11i)", () => {
      const a: ComplexHP = createComplexHP(2, 1);
      const b: ComplexHP = createComplexHP(3, 4);
      const result = multiplyHP(a, b);

      // (2+i)*(3+4i) = 2*3 + 2*4i + i*3 + i*4i
      //               = 6 + 8i + 3i + 4i²
      //               = 6 + 11i - 4
      //               = 2 + 11i
      expect(result.real.toNumber()).toBe(2);
      expect(result.imag.toNumber()).toBe(11);
    });

    it("should handle multiplication by zero", () => {
      const a: ComplexHP = createComplexHP(2, 3);
      const b: ComplexHP = createComplexHP(0, 0);
      const result = multiplyHP(a, b);

      expect(result.real.toNumber()).toBe(0);
      expect(result.imag.toNumber()).toBe(0);
    });

    it("should handle purely imaginary multiplication: i * i = -1", () => {
      const a: ComplexHP = createComplexHP(0, 1);
      const b: ComplexHP = createComplexHP(0, 1);
      const result = multiplyHP(a, b);

      expect(result.real.toNumber()).toBe(-1);
      expect(result.imag.toNumber()).toBe(0);
    });

    it("should maintain high precision for large numbers", () => {
      const a: ComplexHP = createComplexHP("1.23456789012345678901234567890", "2.34567890123456789012345678901");
      const b: ComplexHP = createComplexHP("3.45678901234567890123456789012", "4.56789012345678901234567890123");
      const result = multiplyHP(a, b);

      // Just verify we maintain more precision than standard doubles (15-17 digits)
      const realStr = result.real.toString();
      const imagStr = result.imag.toString();

      // Both should have at least 20 significant digits
      expect(realStr.replace(/[.-]/g, "").length).toBeGreaterThanOrEqual(20);
      expect(imagStr.replace(/[.-]/g, "").length).toBeGreaterThanOrEqual(20);
    });
  });

  describe("magnitudeSquaredHP", () => {
    it("should calculate |3+4i|² = 25", () => {
      const z: ComplexHP = createComplexHP(3, 4);
      const result = magnitudeSquaredHP(z);

      expect(result.toNumber()).toBe(25);
    });

    it("should calculate |1+i|² = 2", () => {
      const z: ComplexHP = createComplexHP(1, 1);
      const result = magnitudeSquaredHP(z);

      expect(result.toNumber()).toBe(2);
    });

    it("should handle zero", () => {
      const z: ComplexHP = createComplexHP(0, 0);
      const result = magnitudeSquaredHP(z);

      expect(result.toNumber()).toBe(0);
    });

    it("should maintain precision at extreme zoom levels", () => {
      const z: ComplexHP = createComplexHP("1e-100", "2e-100");
      const result = magnitudeSquaredHP(z);

      // |z|² = (1e-100)² + (2e-100)² = 1e-200 + 4e-200 = 5e-200
      expect(result.toString()).toBe("5e-200");
    });
  });

  describe("hpToStd", () => {
    it("should convert high-precision to standard precision", () => {
      const hp: ComplexHP = createComplexHP(1.5, 2.5);
      const std = hpToStd(hp);

      expect(std.real).toBe(1.5);
      expect(std.imag).toBe(2.5);
      expect(typeof std.real).toBe("number");
      expect(typeof std.imag).toBe("number");
    });

    it("should lose precision beyond double precision limits", () => {
      const hp: ComplexHP = createComplexHP("1.1234567890123456789012345", "2.9876543210987654321098765");
      const std = hpToStd(hp);

      // Should be converted to double precision (loses digits beyond ~15-17)
      expect(typeof std.real).toBe("number");
      expect(typeof std.imag).toBe("number");
      // Exact value depends on rounding, but should be close
      expect(std.real).toBeCloseTo(1.123456789012346, 15);
      expect(std.imag).toBeCloseTo(2.987654321098765, 15);
    });
  });

  describe("stdToHP", () => {
    it("should convert standard precision to high-precision", () => {
      const std = { real: 1.5, imag: 2.5 };
      const hp = stdToHP(std);

      expect(hp.real).toBeInstanceOf(Decimal);
      expect(hp.imag).toBeInstanceOf(Decimal);
      expect(hp.real.toNumber()).toBe(1.5);
      expect(hp.imag.toNumber()).toBe(2.5);
    });

    it("should preserve the precision of the input number", () => {
      const std = { real: 0.1, imag: 0.2 };
      const hp = stdToHP(std);

      // Note: 0.1 and 0.2 are not exactly representable in binary floating point
      // So we convert them to Decimal, but they still have the same imprecision
      expect(hp.real).toBeInstanceOf(Decimal);
      expect(hp.imag).toBeInstanceOf(Decimal);
    });
  });

  describe("createComplexHP", () => {
    it("should create from numbers", () => {
      const z = createComplexHP(3, 4);

      expect(z.real).toBeInstanceOf(Decimal);
      expect(z.imag).toBeInstanceOf(Decimal);
      expect(z.real.toNumber()).toBe(3);
      expect(z.imag.toNumber()).toBe(4);
    });

    it("should create from strings for precision preservation", () => {
      const z = createComplexHP(
        "1.234567890123456789012345678901234567890",
        "9.876543210987654321098765432109876543210"
      );

      // Verify we maintain more than 15 digits (standard double precision)
      // Note: Decimal.js normalizes numbers, so trailing zeros may be removed
      const realStr = z.real.toString();
      const imagStr = z.imag.toString();

      // Verify precision is maintained (39+ digits, trailing zeros may be removed)
      expect(realStr.replace(/[.-]/g, "").length).toBeGreaterThanOrEqual(39);
      expect(imagStr.replace(/[.-]/g, "").length).toBeGreaterThanOrEqual(39);
      // Verify the value starts correctly
      expect(realStr).toMatch(/^1\.234567890123456789012345678901234567/);
      expect(imagStr).toMatch(/^9\.876543210987654321098765432109876543/);
    });

    it("should handle scientific notation strings", () => {
      const z = createComplexHP("1.5e-100", "2.5e+100");

      expect(z.real.toString()).toBe("1.5e-100");
      expect(z.imag.toString()).toBe("2.5e+100");
    });
  });

  describe("precision verification at 50+ decimal places", () => {
    it("should maintain at least 50 decimal places in calculations", () => {
      // Configure Decimal.js to use 60 decimal places for this test
      const originalPrecision = Decimal.precision;
      Decimal.set({ precision: 60 });

      // Use a high-precision constant (pi to 60 digits)
      const pi60 = "3.14159265358979323846264338327950288419716939937510582097494";
      const a: ComplexHP = createComplexHP(pi60, pi60);
      const b: ComplexHP = createComplexHP("2", "3");

      const result = addHP(a, b);

      // Verify we maintain precision in the result
      const realStr = result.real.toString();
      expect(realStr.length).toBeGreaterThan(50);

      // Restore original precision
      Decimal.set({ precision: originalPrecision });
    });
  });

  describe("type safety", () => {
    it("should not allow mixing ComplexHP and ComplexStd at type level", () => {
      // This is a compile-time check, but we can verify the types are distinct
      const hp: ComplexHP = createComplexHP(1, 2);
      const std = { real: 1, imag: 2 };

      // TypeScript should catch this error at compile time
      // @ts-expect-error - Cannot assign ComplexStd to ComplexHP
      const invalid: ComplexHP = std;

      // Verify runtime distinction
      expect(hp.real).toBeInstanceOf(Decimal);
      expect(typeof std.real).toBe("number");

      // Suppress unused variable warning
      expect(invalid).toBeDefined();
    });
  });
});
