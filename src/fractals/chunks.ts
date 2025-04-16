export interface RenderChunk {
  startX: number;
  startY: number;
  width: number;
  height: number;
}

type ChunkOptions = {
  preferredNumber: number;
  minSize: number;
  maxSize: number;
};

/**
 * This function divides an area (e.g. a canvas) of width x height pixels into
 * smaller chunks for rendering.
 *
 * @param width of the area to divide into chunks
 * @param height of the area to divide into chunks
 * @param options for the chunk size calculation. the defaults are pretty sensible
 * @returns an array of chunks
 */
export function createChunks(
  width: number,
  height: number,
  options: ChunkOptions = { preferredNumber: 250, minSize: 20, maxSize: 1000 }
): RenderChunk[] {
  const chunks: RenderChunk[] = [];
  const chunkSize = calculateOptimalChunkSize(width, height, options);

  // Create chunks in a spiral pattern starting from the center
  const centerX = Math.floor(width / 2);
  const centerY = Math.floor(height / 2);

  const addChunkIfValid = (x: number, y: number, w: number, h: number) => {
    if (x + w > 0 && y + h > 0 && x < width && y < height) {
      const startX = Math.max(0, x);
      const startY = Math.max(0, y);
      const endX = Math.min(x + w, width);
      const endY = Math.min(y + h, height);

      const visibleWidth = endX - startX;
      const visibleHeight = endY - startY;

      if (visibleWidth > 0 && visibleHeight > 0) {
        chunks.push({
          startX,
          startY,
          width: visibleWidth,
          height: visibleHeight,
        });
      }
    }
  };

  // Start with a center chunk
  addChunkIfValid(centerX - chunkSize / 2, centerY - chunkSize / 2, chunkSize, chunkSize);

  // Add chunks in expanding spiral
  let layer = 1;
  while (layer * chunkSize < Math.max(width, height)) {
    // Top row
    for (let x = centerX - layer * chunkSize; x < centerX + layer * chunkSize; x += chunkSize) {
      addChunkIfValid(x, centerY - layer * chunkSize, chunkSize, chunkSize);
    }

    // Right column
    for (let y = centerY - layer * chunkSize + chunkSize; y < centerY + layer * chunkSize; y += chunkSize) {
      addChunkIfValid(centerX + layer * chunkSize - chunkSize, y, chunkSize, chunkSize);
    }

    // Bottom row
    for (let x = centerX + layer * chunkSize - 2 * chunkSize; x >= centerX - layer * chunkSize; x -= chunkSize) {
      addChunkIfValid(x, centerY + layer * chunkSize - chunkSize, chunkSize, chunkSize);
    }

    // Left column
    for (let y = centerY + layer * chunkSize - 2 * chunkSize; y > centerY - layer * chunkSize; y -= chunkSize) {
      addChunkIfValid(centerX - layer * chunkSize, y, chunkSize, chunkSize);
    }

    layer++;
  }

  return chunks;
}

function calculateOptimalChunkSize(width: number, height: number, options: ChunkOptions): number {
  const totalPixels = width * height;
  const targetChunkPixels = totalPixels / options.preferredNumber;
  const chunkSize = Math.floor(Math.sqrt(targetChunkPixels));
  return Math.max(options.minSize, Math.min(options.maxSize, chunkSize));
}
