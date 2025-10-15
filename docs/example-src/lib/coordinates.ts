import { Decimal } from "decimal.js";

const INITIAL_FRACTAL_VIEW_HEIGHT = 4;

export const pixelToFractalCoordinate = (
  point: { x: number; y: number },
  width: number,
  height: number,
  center: { x: number; y: number },
  zoom: Decimal
) => {
  // Use high precision arithmetic for extreme zoom levels
  const originalPrecision = Decimal.precision;
  Decimal.set({ precision: 300 });

  try {
    // Calculate pixel offset from the center of the canvas
    const dx = new Decimal(point.x).minus(new Decimal(width).div(2));
    const dy = new Decimal(point.y).minus(new Decimal(height).div(2));

    // Calculate the scaling factor using Decimal arithmetic for precision.
    // This determines how many fractal units correspond to the canvas height.
    const scale = new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT)
      .div(new Decimal(height))
      .div(zoom);

    // Apply the scaled offset to the fractal center coordinate
    const x = dx.times(scale).plus(new Decimal(center.x)).toNumber();
    const y = dy.times(scale).plus(new Decimal(center.y)).toNumber();
    return { x, y };
  } finally {
    Decimal.set({ precision: originalPrecision });
  }
};

export const fractalToPixelCoordinate = (
  point: { x: number; y: number },
  width: number,
  height: number,
  center: { x: number; y: number },
  zoom: Decimal
) => {
  // Use high precision arithmetic for extreme zoom levels
  const originalPrecision = Decimal.precision;
  Decimal.set({ precision: 300 });

  try {
    // Calculate scaling factor using Decimal arithmetic for precision
    const scale = new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT)
      .div(new Decimal(height))
      .div(zoom);

    // Calculate the offset in fractal units from the center
    const dx = new Decimal(point.x).minus(new Decimal(center.x)).div(scale);
    const dy = new Decimal(point.y).minus(new Decimal(center.y)).div(scale);

    // Convert the fractal offset back to a pixel offset from the canvas center
    const x = dx.plus(new Decimal(width).div(2)).toNumber();
    const y = dy.plus(new Decimal(height).div(2)).toNumber();
    return { x, y };
  } finally {
    Decimal.set({ precision: originalPrecision });
  }
};

// High-precision versions that work with Decimal objects
export const pixelToFractalCoordinateHP = (
  point: { x: number; y: number },
  width: number,
  height: number,
  center: { x: Decimal; y: Decimal },
  zoom: Decimal
): { x: Decimal; y: Decimal } => {
  const originalPrecision = Decimal.precision;
  Decimal.set({ precision: 300 });

  try {
    const dx = new Decimal(point.x).minus(new Decimal(width).div(2));
    const dy = new Decimal(point.y).minus(new Decimal(height).div(2));
    const scale = new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT)
      .div(new Decimal(height))
      .div(new Decimal(zoom));
    const x = dx.times(scale).plus(center.x);
    const y = dy.times(scale).plus(center.y);
    
    // Return Decimal objects to maintain precision in the conversion chain
    return { x, y };
  } finally {
    Decimal.set({ precision: originalPrecision });
  }
};

export const fractalToPixelCoordinateHP = (
  point: { x: number; y: number },
  width: number,
  height: number,
  center: { x: Decimal; y: Decimal },
  zoom: Decimal
) => {
  const originalPrecision = Decimal.precision;
  Decimal.set({ precision: 300 });

  try {
    const scale = new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT)
      .div(new Decimal(height))
      .div(new Decimal(zoom));
    const dx = new Decimal(point.x).minus(center.x).div(scale);
    const dy = new Decimal(point.y).minus(center.y).div(scale);
    const x = dx.plus(new Decimal(width).div(2)).toNumber();
    const y = dy.plus(new Decimal(height).div(2)).toNumber();
    return { x, y };
  } finally {
    Decimal.set({ precision: originalPrecision });
  }
};

// Ultra-high-precision versions that work entirely with Decimal objects
export const pixelToFractalCoordinateUltraHP = (
  point: { x: number; y: number },
  width: number,
  height: number,
  center: { x: Decimal; y: Decimal },
  zoom: Decimal
): { x: Decimal; y: Decimal } => {
  const originalPrecision = Decimal.precision;
  Decimal.set({ precision: 300 });

  try {
    const dx = new Decimal(point.x).minus(new Decimal(width).div(2));
    const dy = new Decimal(point.y).minus(new Decimal(height).div(2));
    const scale = new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT)
      .div(new Decimal(height))
      .div(new Decimal(zoom));
    const x = dx.times(scale).plus(center.x);
    const y = dy.times(scale).plus(center.y);
    
    // Return Decimal objects to maintain precision in the conversion chain
    return { x, y };
  } finally {
    Decimal.set({ precision: originalPrecision });
  }
};

export const fractalToPixelCoordinateUltraHP = (
  point: { x: Decimal; y: Decimal },
  width: number,
  height: number,
  center: { x: Decimal; y: Decimal },
  zoom: Decimal
) => {
  const originalPrecision = Decimal.precision;
  Decimal.set({ precision: 300 });

  try {
    const scale = new Decimal(INITIAL_FRACTAL_VIEW_HEIGHT)
      .div(new Decimal(height))
      .div(new Decimal(zoom));
    const dx = point.x.minus(center.x).div(scale);
    const dy = point.y.minus(center.y).div(scale);
    const x = dx.plus(new Decimal(width).div(2)).toNumber();
    const y = dy.plus(new Decimal(height).div(2)).toNumber();
    return { x, y };
  } finally {
    Decimal.set({ precision: originalPrecision });
  }
};
