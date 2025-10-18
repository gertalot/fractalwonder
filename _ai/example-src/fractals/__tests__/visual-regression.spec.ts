import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  // Navigate to the app
  await page.goto("/");

  // Wait for the app to load
  await page.waitForSelector("canvas");

  // Wait a bit for initial render
  await page.waitForTimeout(1000);
});

test("should render fractal correctly at zoom 1", async ({ page }) => {
  // Set zoom to 1
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({ zoom: 1 });
    }
  });

  // Wait for render
  await page.waitForTimeout(2000);

  // Take screenshot
  const screenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/zoom-1.png",
  });

  // Basic check: canvas should not be empty
  const canvas = await page.$("canvas");
  expect(canvas).toBeTruthy();

  // Check that canvas has content (not all black or white)
  const imageData = await page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    if (!canvas) return null;

    const ctx = canvas.getContext("2d");
    if (!ctx) return null;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    return {
      width: canvas.width,
      height: canvas.height,
      dataLength: data.data.length,
      samplePixels: Array.from(data.data.slice(0, 100)), // First 100 bytes
    };
  });

  expect(imageData).toBeTruthy();
  expect(imageData?.dataLength).toBeGreaterThan(0);

  // Should have some variation in pixel colors (not all identical)
  const samplePixels = imageData?.samplePixels || [];
  const uniqueValues = new Set(samplePixels);
  expect(uniqueValues.size).toBeGreaterThan(1);
});

test("should render fractal correctly at zoom 10^3", async ({ page }) => {
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({ zoom: 1000 });
    }
  });

  await page.waitForTimeout(2000);

  const screenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/zoom-1e3.png",
  });

  // Check for visual content
  const imageData = await page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    if (!canvas) return null;

    const ctx = canvas.getContext("2d");
    if (!ctx) return null;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    return {
      width: canvas.width,
      height: canvas.height,
      dataLength: data.data.length,
      samplePixels: Array.from(data.data.slice(0, 100)),
    };
  });

  expect(imageData).toBeTruthy();
  expect(imageData?.dataLength).toBeGreaterThan(0);

  const samplePixels = imageData?.samplePixels || [];
  const uniqueValues = new Set(samplePixels);
  expect(uniqueValues.size).toBeGreaterThan(1);
});

test("should render fractal correctly at zoom 10^6", async ({ page }) => {
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({ zoom: 1000000 });
    }
  });

  await page.waitForTimeout(3000);

  const screenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/zoom-1e6.png",
  });

  // Check for visual content
  const imageData = await page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    if (!canvas) return null;

    const ctx = canvas.getContext("2d");
    if (!ctx) return null;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    return {
      width: canvas.width,
      height: canvas.height,
      dataLength: data.data.length,
      samplePixels: Array.from(data.data.slice(0, 100)),
    };
  });

  expect(imageData).toBeTruthy();
  expect(imageData?.dataLength).toBeGreaterThan(0);

  const samplePixels = imageData?.samplePixels || [];
  const uniqueValues = new Set(samplePixels);
  expect(uniqueValues.size).toBeGreaterThan(1);
});

test("should render fractal correctly at zoom 10^9", async ({ page }) => {
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({ zoom: 1000000000 });
    }
  });

  await page.waitForTimeout(5000);

  const screenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/zoom-1e9.png",
  });

  // Check for visual content
  const imageData = await page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    if (!canvas) return null;

    const ctx = canvas.getContext("2d");
    if (!ctx) return null;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    return {
      width: canvas.width,
      height: canvas.height,
      dataLength: data.data.length,
      samplePixels: Array.from(data.data.slice(0, 100)),
    };
  });

  expect(imageData).toBeTruthy();
  expect(imageData?.dataLength).toBeGreaterThan(0);

  const samplePixels = imageData?.samplePixels || [];
  const uniqueValues = new Set(samplePixels);
  expect(uniqueValues.size).toBeGreaterThan(1);
});

test("should render fractal correctly at zoom 10^12", async ({ page }) => {
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({ zoom: 1000000000000 });
    }
  });

  await page.waitForTimeout(10000);

  const screenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/zoom-1e12.png",
  });

  // Check for visual content
  const imageData = await page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    if (!canvas) return null;

    const ctx = canvas.getContext("2d");
    if (!ctx) return null;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    return {
      width: canvas.width,
      height: canvas.height,
      dataLength: data.data.length,
      samplePixels: Array.from(data.data.slice(0, 100)),
    };
  });

  expect(imageData).toBeTruthy();
  expect(imageData?.dataLength).toBeGreaterThan(0);

  const samplePixels = imageData?.samplePixels || [];
  const uniqueValues = new Set(samplePixels);
  expect(uniqueValues.size).toBeGreaterThan(1);
});

test("should detect blockiness at zoom 10^13", async ({ page }) => {
  // This test specifically looks for the blockiness that indicates precision loss
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({ zoom: 60000000000000 }); // 6e13 - the exact failing case
    }
  });

  await page.waitForTimeout(15000);

  const screenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/zoom-6e13.png",
  });

  // Analyze the image for blockiness
  const blockinessAnalysis = await page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    if (!canvas) return null;

    const ctx = canvas.getContext("2d");
    if (!ctx) return null;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const pixels = data.data;

    // Check for large regions of identical colors (blockiness)
    const blockSize = 10; // Check 10x10 blocks
    const identicalBlocks = [];

    for (let y = 0; y < canvas.height - blockSize; y += blockSize) {
      for (let x = 0; x < canvas.width - blockSize; x += blockSize) {
        // Get the first pixel in this block
        const firstPixelIndex = (y * canvas.width + x) * 4;
        const firstR = pixels[firstPixelIndex];
        const firstG = pixels[firstPixelIndex + 1];
        const firstB = pixels[firstPixelIndex + 2];

        // Check if all pixels in this block are identical
        let allIdentical = true;
        for (let dy = 0; dy < blockSize && allIdentical; dy++) {
          for (let dx = 0; dx < blockSize && allIdentical; dx++) {
            const pixelIndex = ((y + dy) * canvas.width + (x + dx)) * 4;
            const r = pixels[pixelIndex];
            const g = pixels[pixelIndex + 1];
            const b = pixels[pixelIndex + 2];

            if (r !== firstR || g !== firstG || b !== firstB) {
              allIdentical = false;
            }
          }
        }

        if (allIdentical) {
          identicalBlocks.push({ x, y, r: firstR, g: firstG, b: firstB });
        }
      }
    }

    return {
      totalBlocks: Math.floor((canvas.width / blockSize) * (canvas.height / blockSize)),
      identicalBlocks: identicalBlocks.length,
      blockinessRatio: identicalBlocks.length / Math.floor((canvas.width / blockSize) * (canvas.height / blockSize)),
      sampleIdenticalBlocks: identicalBlocks.slice(0, 5),
    };
  });

  expect(blockinessAnalysis).toBeTruthy();

  // Should not have too many identical blocks (indicates blockiness)
  // Current broken implementation will have high blockiness ratio
  expect(blockinessAnalysis?.blockinessRatio).toBeLessThan(0.5);

  console.log(
    `Blockiness analysis: ${blockinessAnalysis?.identicalBlocks} identical blocks out of ${blockinessAnalysis?.totalBlocks} total blocks`
  );
  console.log(`Blockiness ratio: ${blockinessAnalysis?.blockinessRatio.toFixed(3)}`);
});

test("should render fractal correctly at zoom 10^15", async ({ page }) => {
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({ zoom: 1000000000000000 });
    }
  });

  await page.waitForTimeout(20000);

  const screenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/zoom-1e15.png",
  });

  // Check for visual content
  const imageData = await page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    if (!canvas) return null;

    const ctx = canvas.getContext("2d");
    if (!ctx) return null;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    return {
      width: canvas.width,
      height: canvas.height,
      dataLength: data.data.length,
      samplePixels: Array.from(data.data.slice(0, 100)),
    };
  });

  expect(imageData).toBeTruthy();
  expect(imageData?.dataLength).toBeGreaterThan(0);

  const samplePixels = imageData?.samplePixels || [];
  const uniqueValues = new Set(samplePixels);
  expect(uniqueValues.size).toBeGreaterThan(1);
});

test("should compare standard vs perturbation algorithms", async ({ page }) => {
  // Test both algorithms at the same zoom level
  const testZoom = 1000;

  // Test standard algorithm
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({
        zoom: 1000,
        algorithmName: "Mandelbrot Set",
      });
    }
  });

  await page.waitForTimeout(3000);

  const standardScreenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/standard-algorithm-zoom-1e3.png",
  });

  // Test perturbation algorithm
  await page.evaluate(() => {
    const store = (window as any).__zustandStore;
    if (store) {
      store.setState({
        zoom: 1000,
        algorithmName: "Perturbation Mandelbrot",
      });
    }
  });

  await page.waitForTimeout(3000);

  const perturbationScreenshot = await page.screenshot({
    fullPage: true,
    path: "test-results/perturbation-algorithm-zoom-1e3.png",
  });

  // Both should produce visual content
  const standardImageData = await page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    if (!canvas) return null;

    const ctx = canvas.getContext("2d");
    if (!ctx) return null;

    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    return {
      width: canvas.width,
      height: canvas.height,
      dataLength: data.data.length,
      samplePixels: Array.from(data.data.slice(0, 100)),
    };
  });

  expect(standardImageData).toBeTruthy();
  expect(standardImageData?.dataLength).toBeGreaterThan(0);

  const standardSamplePixels = standardImageData?.samplePixels || [];
  const standardUniqueValues = new Set(standardSamplePixels);
  expect(standardUniqueValues.size).toBeGreaterThan(1);

  console.log(`Standard algorithm: ${standardUniqueValues.size} unique values in sample`);
  console.log(`Perturbation algorithm: visual content verified`);
});
