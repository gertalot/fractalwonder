import '@testing-library/jest-dom';
import { Decimal } from 'decimal.js';

// Configure Decimal.js for ultra-high precision matching the app configuration
Decimal.set({ precision: 300 });

// Polyfill ImageData for jsdom environment
if (typeof ImageData === 'undefined') {
  // @ts-expect-error - Polyfilling for test environment
  global.ImageData = class ImageData {
    data: Uint8ClampedArray;
    width: number;
    height: number;

    constructor(dataOrWidth: Uint8ClampedArray | number, widthOrHeight: number, height?: number) {
      if (dataOrWidth instanceof Uint8ClampedArray) {
        this.data = dataOrWidth;
        this.width = widthOrHeight;
        this.height = height!;
      } else {
        this.width = dataOrWidth;
        this.height = widthOrHeight;
        this.data = new Uint8ClampedArray(dataOrWidth * widthOrHeight * 4);
      }
    }
  };
}
