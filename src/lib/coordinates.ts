const INITIAL_FRACTAL_VIEW_HEIGHT = 4;

export const pixelToFractalCoordinate = (
  point: { x: number; y: number },
  width: number,
  height: number,
  center: { x: number; y: number },
  zoom: number
) => {
  // Calculate pixel offset from the center of the canvas
  const dx = point.x - width / 2;
  const dy = point.y - height / 2;

  // Calculate the scaling factor.
  // This determines how many fractal units correspond to the canvas height.
  const scale = INITIAL_FRACTAL_VIEW_HEIGHT / height / zoom;

  // Apply the scaled offset to the fractal center coordinate
  const x = dx * scale + center.x;
  const y = dy * scale + center.y;
  return { x, y };
};

export const fractalToPixelCoordinate = (
  point: { x: number; y: number },
  width: number,
  height: number,
  center: { x: number; y: number },
  zoom: number
) => {
  // Same scaling factor
  const scale = 4 / height / zoom;

  // Calculate the offset in fractal units from the center
  const dx = (point.x - center.x) / scale;
  const dy = (point.y - center.y) / scale;

  // Convert the fractal offset back to a pixel offset from the canvas center
  const x = dx + width / 2;
  const y = dy + height / 2;
  return { x, y };
};
