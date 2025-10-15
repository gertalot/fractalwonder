// ABOUTME: Tests for color scheme functions
// ABOUTME: Validates smooth looping color scheme and HSL conversion

import { describe, expect, it } from "vitest";
import { fireColorScheme, smoothLoopingColorScheme } from "./coloring";

describe("Color Scheme Tests", () => {
  describe("smoothLoopingColorScheme", () => {
    it("should return black for points in the set", () => {
      const result = smoothLoopingColorScheme(1000, 1000);
      expect(result).toEqual([0, 0, 0]);
    });

    it("should return valid RGB values for escaped points", () => {
      const result = smoothLoopingColorScheme(100, 1000);
      
      // All values should be between 0 and 255
      expect(result[0]).toBeGreaterThanOrEqual(0);
      expect(result[0]).toBeLessThanOrEqual(255);
      expect(result[1]).toBeGreaterThanOrEqual(0);
      expect(result[1]).toBeLessThanOrEqual(255);
      expect(result[2]).toBeGreaterThanOrEqual(0);
      expect(result[2]).toBeLessThanOrEqual(255);
    });

    it("should create smooth color transitions", () => {
      const maxIter = 1000;
      const colors = [];
      
      // Test a range of iteration counts
      for (let iter = 1; iter <= 100; iter += 10) {
        const color = smoothLoopingColorScheme(iter, maxIter);
        colors.push(color);
      }
      
      // Colors should vary smoothly (not all identical)
      const uniqueColors = new Set(colors.map(c => `${c[0]},${c[1]},${c[2]}`));
      expect(uniqueColors.size).toBeGreaterThan(1);
      
      // Should have reasonable color variation
      const rValues = colors.map(c => c[0]);
      const gValues = colors.map(c => c[1]);
      const bValues = colors.map(c => c[2]);
      
      const rRange = Math.max(...rValues) - Math.min(...rValues);
      const gRange = Math.max(...gValues) - Math.min(...gValues);
      const bRange = Math.max(...bValues) - Math.min(...bValues);
      
      // At least one color channel should have significant variation
      expect(Math.max(rRange, gRange, bRange)).toBeGreaterThan(50);
    });

    it("should loop colors at extreme zoom levels", () => {
      const maxIter = 10000;
      
      // Test that colors cycle through the spectrum
      const color1 = smoothLoopingColorScheme(100, maxIter);
      const color2 = smoothLoopingColorScheme(2000, maxIter);
      const color3 = smoothLoopingColorScheme(4000, maxIter);
      const color4 = smoothLoopingColorScheme(6000, maxIter);
      
      // Colors should be different (cycling through spectrum)
      const colors = [color1, color2, color3, color4];
      const uniqueColors = new Set(colors.map(c => `${c[0]},${c[1]},${c[2]}`));
      expect(uniqueColors.size).toBeGreaterThan(2);
    });

    it("should handle edge cases", () => {
      // Very low iteration count
      const lowIter = smoothLoopingColorScheme(1, 1000);
      expect(lowIter[0]).toBeGreaterThanOrEqual(0);
      expect(lowIter[0]).toBeLessThanOrEqual(255);
      
      // Very high iteration count (but not max)
      const highIter = smoothLoopingColorScheme(999, 1000);
      expect(highIter[0]).toBeGreaterThanOrEqual(0);
      expect(highIter[0]).toBeLessThanOrEqual(255);
    });
  });

  describe("fireColorScheme", () => {
    it("should return black for points in the set", () => {
      const result = fireColorScheme(1000, 1000);
      expect(result).toEqual([0, 0, 0]);
    });

    it("should return valid RGB values for escaped points", () => {
      const result = fireColorScheme(100, 1000);
      
      // All values should be between 0 and 255
      expect(result[0]).toBeGreaterThanOrEqual(0);
      expect(result[0]).toBeLessThanOrEqual(255);
      expect(result[1]).toBeGreaterThanOrEqual(0);
      expect(result[1]).toBeLessThanOrEqual(255);
      expect(result[2]).toBeGreaterThanOrEqual(0);
      expect(result[2]).toBeLessThanOrEqual(255);
    });

    it("should create fire-like color progression", () => {
      const maxIter = 1000;
      
      // Low iterations should be red
      const lowIter = fireColorScheme(50, maxIter);
      expect(lowIter[0]).toBeGreaterThan(lowIter[1]); // More red than green
      expect(lowIter[1]).toBeGreaterThanOrEqual(lowIter[2]); // More green than blue
      
      // Mid iterations should be yellow (red + green)
      const midIter = fireColorScheme(300, maxIter);
      expect(midIter[0]).toBe(255); // Max red
      expect(midIter[1]).toBeGreaterThan(0); // Some green
      expect(midIter[2]).toBe(0); // No blue
      
      // High iterations should be white (all colors)
      const highIter = fireColorScheme(800, maxIter);
      expect(highIter[0]).toBe(255); // Max red
      expect(highIter[1]).toBe(255); // Max green
      expect(highIter[2]).toBeGreaterThan(0); // Some blue
    });
  });
});