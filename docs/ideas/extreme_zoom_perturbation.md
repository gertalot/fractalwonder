# Extreme zoom levels, perturbation theory, and progressive rendering

Let's explore a combination of perturbation theory and progressive rendering.

The high-level idea is as follows:

On the main thread, we have a render function that basically coordinates work and sends it to worker threads
for pure computation.

First, on the computation (worker thread) side, we have a function that renders a tile. The worker receives:
- tile pixel rect (x,y, width, height)
- viewport in fractal space (center, zoom)
- max iterations
- nearest reference orbit
- bits of precision for accuracy

the render function computes the tile, tracks the maximum error, and if it exceeds a threshold, sends a
message back to the main thread saying the error threshold was exceeded. Otherwise it sends back the
result of the computed data for the tile.

The main thread's render (coordination) function looks like this:

1. The render function is called, with these parameters:
   - the viewport (zoom and center) in fractal space
   - the canvas rect in pixels
   - maximum number of iterations
   - a colorizer function
2. the render function computes high-precision reference orbit for the center and four corners of the viewport
   and stores them in a quadtree of reference orbits
3. the render function creates a queue of tiles, with a configurable tile size (e.g. smaller tiles at higher zooms)
4. the render function loops over the queue of tiles until it is empty:
   - it takes the first tile off the queue, finds the nearest reference function, and sends the work to the worker
   - if a worker comes back with a successfully computed tile, it stores the tile data
   - if a worker comes back with an "error threshold exceeded" message, the main thread render function computes
     extra reference orbits by subdividing the quadtree for the location of the tile and computing new reference
     orbits at the subdivision points. It also puts the tile back on the queue

Reference orbits and mandelbrot compute mathematics follow perturbation theory.

to compute required bits of precision in main thread:

```rs
fn required_precision(viewport_width: f64, canvas_width: usize, max_iter: usize) -> usize {
    let delta = viewport_width / (canvas_width as f64);
    let bits_for_spacing = (-delta.log2()).ceil() as usize;
    let bits_for_iterations = (max_iter as f64).log2().ceil() as usize;
    let safety_margin = 32; // adjust as needed
    bits_for_spacing + bits_for_iterations + safety_margin
}

// In main thread
let precision = required_precision(viewport.width, canvas.width, max_iter);
worker.send(TileRequest {
    tile,
    reference_orbit,
    precision,
});
```

