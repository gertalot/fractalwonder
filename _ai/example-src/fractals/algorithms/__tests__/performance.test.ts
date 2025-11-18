import { Decimal } from "decimal.js";
import { beforeEach, describe, expect, it } from "vitest";
import { createComplexHP } from "../../mandelbrot/hp-math";
import { calculateReferenceOrbit } from "../../mandelbrot/reference-orbit";
import { mandelbrotAlgorithm } from "../mandelbrot";
import { PerturbationMandelbrotAlgorithm } from "../perturbation-mandelbrot";

describe("Performance Benchmark Tests", () => {
  let perturbationAlgorithm: PerturbationMandelbrotAlgorithm;

  beforeEach(() => {
    perturbationAlgorithm = new PerturbationMandelbrotAlgorithm();
    Decimal.set({ precision: 100 }); // High precision for reference orbit
  });

  describe("perturbation vs standard algorithm performance", () => {
    it("should be ≥50x faster than standard algorithm at zoom 10^15", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(1e15);
      const maxIterations = 100;
      const testPixels = 1000; // Test 1000 pixels

      // Prepare perturbation algorithm
      perturbationAlgorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Time perturbation algorithm
      const perturbationStart = performance.now();
      for (let i = 0; i < testPixels; i++) {
        const offsetX = (Math.random() - 0.5) * 100;
        const offsetY = (Math.random() - 0.5) * 100;

        const scale = 4 / 1080 / zoom.toNumber();
        const real = center.x + offsetX * scale;
        const imag = center.y + offsetY * scale;

        perturbationAlgorithm.computePoint(real, imag, maxIterations);
      }
      const perturbationTime = performance.now() - perturbationStart;

      // Time standard algorithm (skip if too slow)
      const standardStart = performance.now();
      let standardTime = 0;
      let standardPixels = 0;

      for (let i = 0; i < testPixels; i++) {
        const offsetX = (Math.random() - 0.5) * 100;
        const offsetY = (Math.random() - 0.5) * 100;

        const scale = 4 / 1080 / zoom.toNumber();
        const real = center.x + offsetX * scale;
        const imag = center.y + offsetY * scale;

        mandelbrotAlgorithm.computePoint(real, imag, maxIterations);
        standardPixels++;

        // Stop if taking too long (more than 30 seconds)
        if (performance.now() - standardStart > 30000) {
          break;
        }
      }
      standardTime = performance.now() - standardStart;

      // Calculate performance metrics
      const perturbationPixelsPerSecond = (testPixels / perturbationTime) * 1000;
      const standardPixelsPerSecond = (standardPixels / standardTime) * 1000;
      const speedup = standardPixelsPerSecond > 0 ? perturbationPixelsPerSecond / standardPixelsPerSecond : 0;

      console.log(`Performance at zoom ${zoom.toExponential(1)}:`);
      console.log(
        `  Perturbation: ${perturbationPixelsPerSecond.toFixed(0)} pixels/sec (${perturbationTime.toFixed(
          1
        )}ms for ${testPixels} pixels)`
      );
      console.log(
        `  Standard: ${standardPixelsPerSecond.toFixed(0)} pixels/sec (${standardTime.toFixed(
          1
        )}ms for ${standardPixels} pixels)`
      );
      console.log(`  Speedup: ${speedup.toFixed(1)}x`);

      // Perturbation should be at least 50x faster
      if (standardPixelsPerSecond > 0) {
        expect(speedup).toBeGreaterThanOrEqual(50);
      }

      // Perturbation should process at least 30,000 pixels/sec
      expect(perturbationPixelsPerSecond).toBeGreaterThan(30000);
    });

    it("should complete 1920×1080 frame at zoom 10^15 in <60 seconds", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(1e15);
      const maxIterations = 100;
      const totalPixels = 1920 * 1080;
      const samplePixels = Math.floor(totalPixels / 100); // Test 1% of pixels

      perturbationAlgorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      const start = performance.now();
      for (let i = 0; i < samplePixels; i++) {
        const x = (i % 1920) - 960; // -960 to 959
        const y = Math.floor(i / 1920) - 540; // -540 to 539

        const scale = 4 / 1080 / zoom.toNumber();
        const real = center.x + x * scale;
        const imag = center.y + y * scale;

        perturbationAlgorithm.computePoint(real, imag, maxIterations);
      }
      const duration = performance.now() - start;

      const pixelsPerSecond = (samplePixels / duration) * 1000;
      const extrapolatedFullFrameTime = totalPixels / pixelsPerSecond;

      console.log(`Sample performance: ${pixelsPerSecond.toFixed(0)} pixels/sec`);
      console.log(`Extrapolated full frame time: ${extrapolatedFullFrameTime.toFixed(1)}s`);

      // Should complete full frame in under 60 seconds
      expect(extrapolatedFullFrameTime).toBeLessThan(60);

      // Should process at least 30,000 pixels/sec
      expect(pixelsPerSecond).toBeGreaterThan(30000);
    });

    it("should be ≥50x faster than full high-precision calculation", () => {
      const center = { x: -1.4845895199757433, y: -6.96e-8 };
      const zoom = new Decimal(1e15);
      const maxIterations = 100;
      const testPixels = 100; // Smaller test due to high-precision slowness

      // Prepare perturbation algorithm
      perturbationAlgorithm.prepareForRender({
        center,
        zoom,
        maxIterations,
      });

      // Time perturbation algorithm
      const perturbationStart = performance.now();
      for (let i = 0; i < testPixels; i++) {
        const offsetX = (Math.random() - 0.5) * 100;
        const offsetY = (Math.random() - 0.5) * 100;

        const scale = 4 / 1080 / zoom.toNumber();
        const real = center.x + offsetX * scale;
        const imag = center.y + offsetY * scale;

        perturbationAlgorithm.computePoint(real, imag, maxIterations);
      }
      const perturbationTime = performance.now() - perturbationStart;

      // Time full high-precision calculation (direct reference orbit for each point)
      const hpStart = performance.now();
      for (let i = 0; i < testPixels; i++) {
        const offsetX = (Math.random() - 0.5) * 100;
        const offsetY = (Math.random() - 0.5) * 100;

        const scale = 4 / 1080 / zoom.toNumber();
        const real = center.x + offsetX * scale;
        const imag = center.y + offsetY * scale;

        // Calculate full high-precision reference orbit for this point
        const centerHP = createComplexHP(real, imag);
        calculateReferenceOrbit(centerHP, maxIterations);
      }
      const hpTime = performance.now() - hpStart;

      const speedup = hpTime / perturbationTime;
      const perturbationPixelsPerSecond = (testPixels / perturbationTime) * 1000;
      const hpPixelsPerSecond = (testPixels / hpTime) * 1000;

      console.log(`High-precision comparison:`);
      console.log(
        `  Perturbation: ${perturbationPixelsPerSecond.toFixed(0)} pixels/sec (${perturbationTime.toFixed(1)}ms)`
      );
      console.log(`  High-precision: ${hpPixelsPerSecond.toFixed(0)} pixels/sec (${hpTime.toFixed(1)}ms)`);
      console.log(`  Speedup: ${speedup.toFixed(1)}x`);

      // Should be at least 50x faster than full high-precision
      expect(speedup).toBeGreaterThanOrEqual(50);
    });
  });

  describe("performance scaling with zoom", () => {
    it("should maintain performance across zoom levels", () => {
      const center = { x: -0.75, y: 0.1 };
      const zoomLevels = [1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15].map((z) => new Decimal(z));
      const maxIterations = 100;
      const testPixels = 1000;

      const performanceResults = [];

      for (const zoom of zoomLevels) {
        // Set precision based on zoom level
        const precision = Math.max(30, Math.ceil(Math.log10(zoom) * 2.5 + 20));
        Decimal.set({ precision });

        perturbationAlgorithm.prepareForRender({
          center,
          zoom,
          maxIterations,
        });

        const start = performance.now();
        for (let i = 0; i < testPixels; i++) {
          const offsetX = (Math.random() - 0.5) * 100;
          const offsetY = (Math.random() - 0.5) * 100;

          const scale = 4 / 1080 / zoom.toNumber();
          const real = center.x + offsetX * scale;
          const imag = center.y + offsetY * scale;

          perturbationAlgorithm.computePoint(real, imag, maxIterations);
        }
        const duration = performance.now() - start;

        const pixelsPerSecond = (testPixels / duration) * 1000;
        performanceResults.push({
          zoom,
          precision,
          pixelsPerSecond,
          duration,
        });

        console.log(`Zoom ${zoom.toExponential(1)}: ${pixelsPerSecond.toFixed(0)} pixels/sec (precision=${precision})`);
      }

      // Performance should not degrade too much at higher zoom levels
      const minPerformance = Math.min(...performanceResults.map((r) => r.pixelsPerSecond));
      const maxPerformance = Math.max(...performanceResults.map((r) => r.pixelsPerSecond));
      const performanceRatio = minPerformance / maxPerformance;

      // Should maintain at least 50% of performance across zoom levels
      expect(performanceRatio).toBeGreaterThan(0.5);

      // All zoom levels should achieve at least 20,000 pixels/sec
      for (const result of performanceResults) {
        expect(result.pixelsPerSecond).toBeGreaterThan(20000);
      }
    });
  });

  describe("memory usage", () => {
    it("should not leak memory during multiple renders", () => {
      const center = { x: -0.75, y: 0.1 };
      const zoom = new Decimal(1e12);
      const maxIterations = 100;
      const testPixels = 1000;
      const renderCount = 20;

      // Force garbage collection if available
      if (typeof global !== "undefined" && global.gc) {
        global.gc();
      }

      const initialMemory = process.memoryUsage?.() || { heapUsed: 0 };

      for (let render = 0; render < renderCount; render++) {
        perturbationAlgorithm.prepareForRender({
          center,
          zoom,
          maxIterations,
        });

        for (let i = 0; i < testPixels; i++) {
          const offsetX = (Math.random() - 0.5) * 100;
          const offsetY = (Math.random() - 0.5) * 100;

          const scale = 4 / 1080 / zoom.toNumber();
          const real = center.x + offsetX * scale;
          const imag = center.y + offsetY * scale;

          perturbationAlgorithm.computePoint(real, imag, maxIterations);
        }

        // Force garbage collection if available
        if (typeof global !== "undefined" && global.gc) {
          global.gc();
        }
      }

      const finalMemory = process.memoryUsage?.() || { heapUsed: 0 };
      const memoryIncrease = finalMemory.heapUsed - initialMemory.heapUsed;

      console.log(
        `Memory usage: ${(initialMemory.heapUsed / 1024 / 1024).toFixed(1)}MB → ${(
          finalMemory.heapUsed /
          1024 /
          1024
        ).toFixed(1)}MB`
      );
      console.log(`Memory increase: ${(memoryIncrease / 1024 / 1024).toFixed(1)}MB`);

      // Memory increase should be reasonable (< 100MB for 20 renders)
      expect(memoryIncrease).toBeLessThan(100 * 1024 * 1024);
    });
  });
});
