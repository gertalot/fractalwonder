function canvasSize(canvas: HTMLCanvasElement | null) {
  if (canvas) {
    const dpr = window.devicePixelRatio || 1;
    // Use viewport dimensions instead of canvas getBoundingClientRect
    // to ensure canvas resizes when dev tools are opened/closed
    const width = window.innerWidth;
    const height = window.innerHeight;
    return { width: width * dpr, height: height * dpr };
  }
  return { width: 0, height: 0 };
}

export default canvasSize;
