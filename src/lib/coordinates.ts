export const pixelToFractalCoordinate = (
  point: { x: number; y: number },
  width: number,
  height: number,
  center: { x: number; y: number },
  zoom: number
) => {
  const dx = point.x - width / 2;
  const dy = point.y - height / 2;
  const scale = 4 / height / zoom;
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
  const scale = 4 / height / zoom;
  const dx = (point.x - center.x) / scale;
  const dy = (point.y - center.y) / scale;
  const x = dx + width / 2;
  const y = dy + height / 2;
  return { x, y };
};
