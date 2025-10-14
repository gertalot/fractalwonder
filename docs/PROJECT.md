# Extreme zoom mandelbrot explorer

I want to create software to compute the mandelbrot set. requirements:

- as FAST as possible. Using multiple cores and GPU
- support progressive rendering in a web UI for interactive exploring
- potentially support distributed rendering to harness remote CPU or GPU power
- support extremely high precision numbers for extreme zoom levels (10^100 or more)

frontend should be a web ui, with a fast simple framework. backend should be something that an AI agent is really good at writing fast code for.

## ideas

- research perturbation theory, bivariate linear approximation, and series approximation
- use WebGPU and web workers for browser-based rendering
- backend-based rendering and distributed rendering is a potential addition for the future. First
  version will be purely browser client based

## How should the program look/feel

tiles should render as they complete. tile size should depend on zoom level / complexity. tiles should render in a spiral pattern from the center outwards so the most interesting tiles are rendered first.

for the interaction: I want the UI to be hidden by default and the canvas should be full screen. When the user moves the mouse or taps the screen, I want a full-width area to fade in at the bottom of the screen. This should display the current zoom and coordinates and iterations, and any other important information. there should be a menu to select the colour scheme, and a way to adjust settings for the chosen colour scheme. The UI should be dark themed, sleek, and minimal, and follow modern ux guidelines and aestethics. In the ui (when visible), there should be a small "i" icon on the bottom left that shows a menu with information and a github link. Next to it should be a "home" icon, that resets the fractal to its default parameters. On the right hand side there should be a "full screen" icon. This changes to an "exit full screen" icon when the UI is full screen.

when the user stops moving the mouse or after the user has tapped the screen, the UI should fade out again after 4 seconds.

While a render is in progress, a small circular progress indicator should show in the bottom left corner. when the ui is displayed, this should be hidden, and instead a linear progress bar should be shown in the ui, and the number of cores, threads, web workers, or distributed computation units should be displayed as a number. Again it should be sleek and minimal. When the render is finished, the progress indicators should not be shown. In the UI (if visible), the time the render took should be displayed.

I have three screenshots in the docs directory that show exactly the UI I want:

- ![ui visible](/docs/fractal-ui-visible.png)
- ![ui invisible, render in progress](/docs/fractal-progress-indicator-no-ui.png)
- ![ui visible, render in progress](/docs/fractal-progress-indicator-ui.png)

For the interactions: The user can interact in three ways: resizing the browser window, click-and-drag, or use the mouse wheel.

When any of these interactions occur, the existing render should terminate immediately to save CPU/GPU cycles.

There should be a "fast preview" of the user interaction using just the imageData of the canvas as it is currently (partially or completely) rendered, as follows:

- when the user resizes the canvas, the fractal's (and thus the image's) center should stay in the center of the canvas, so the image should be scaled and moved accordingly, while maintaining the correct aspect ratio. NOTE: the render should ONLY be cancelled if the canvas is ACTUALLY changing size. Simply clicking on the window resize handle and releasing should not cancel the current render.

- when the user clicks and holds the mouse, and the moves the mouse (aka dragging), any existing render should be
  terminated immediately, but ONLY if the user is actualy moving (dragging) the image. The fast preview functionality
  should display the current imageData (pixel data) and move it exactly the same amount as the mouse pointer is moving.
  That means the preview should accurately move along with the mouse so the user can see what is happening. NOTE: At
  extremely large zoom levels, moving the mouse, say 100x200 pixels, will result in an extremely tiny change in the
  fractal coordinates (easily 250+ decimal places). This extremely high precision must be fully supported.

- when the user uses the mouse wheel to zoom in/out, the zoom should feel natural, e.g. the same scroll distance
  of the mouse wheel should result in the same visible change in zoom, regardless of the zoom value (this could be
  anywhere from 1.00x to 10^100x or more)

- IMPORTANT: zoom operations should ALWAYS be centered on the current mouse pointer location, or in the case of
  a multi-touch zoom gesture, should be centered in the center of the multiple touches.

- fast previews of moving and scaling the imageData should **ALWAYS** feel smooth and accurate for the user, even
  at extreme zoom levels (10^100+). That means that interactions should work in pixel values and precisely track
  the correct x,y and scale values in pixel space. This should then be **FULLY ACCURATELY** be mapped to center
  coordinates and zoom value in fractal space (which could be EXTREMELY small or large values).

- Interactions start as soon as the mouse is down AND has moved, or when the window has changed size, or when the mouse
  wheel has been used. Interactions STOP when the user has not interacted for 1.5 seconds. That lets the user perform
  multiple drag and zoom operations before a new render starts. Merely moving the mouse without pressing buttons does
  not count as interacting in this sense.

## Colouring algorithms

For now I want predefined colour algorithms. Note that I want the computation step to be separate from the colouring step. The computation step should result in the data that is needed for advanced colouring schemes for each pixel (e.g. smooth, orbit trap, potential, distance estimation, or field-line–based methods). the key is to store enough intermediate data per pixel to allow flexible post-processing without recomputation of the core iteration loop.

We need iteration count, escape radius flag (escaped yes/no), final z value, smoothed iteration value (mu), initial
derivative magnitude, and other values needed for advanced colouring algorithms.

## persistence

I want the current parameters to be stored in local storage. I also want the URL to reflect the current parameters (in a
compressed encoded way as URL parameters) so I can simply bookmark the page and come back to it later. If the parameters
don't exist, local storage values should be used. If that doesn't exist either, default values should be used.

## distributed rendering

out of scope, but the tech stack should be such that we can easily run the core mathematical code on the browser or the
backend (e.g. compile to web assembly or native targets). We will use Rust.

## performance

performance: as fast as possible, but I know renders can be slow and that's ok. I don't want to set hard limits.
resolution: initially just the size of the browser canvas, which could be anything, including 4K or larger. hardware:
should work on modern hardware with a modern browser. Should use all CPU cores and potentially GPU if available.

## testing

From experience I can tell you that this code is going to be VERY tricky to get right, especially in these areas:

1. the user interactions and accurate, smooth, fast imageData-based previews
2. the mathematics that calculate the mandelbrot set at EXTREME zoom levels (and therefore EXTREMELY SMALL real and imaginary coordinates, with 250+ decimal places)

VERY COMPREHENSIVE test suites are required that can test ALL aspects of the mathematics, ESPECIALLY where values in
pixel space are converted to fractal space and back.

## test/implementation considerations

- Coordinate representation: How should we store viewport center coordinates internally (Rust arbitrary precision types, string representation, mantissa-exponent pairs)? — correct, we need to research this

- Pixel-to-fractal mapping: When the user drags 100 pixels and we need to calculate the coordinate change at 10^100 zoom, how should this conversion work to maintain precision? — correct, we need to research this and implement it correctly

- Zoom calculations: When zoom changes from mouse wheel, how do we calculate the new center coordinates while keeping the mouse position fixed in fractal space? — what do you mean by this?

- Precision loss detection: Should the system detect when precision is insufficient and warn/adjust? — DEFINITELY. Also the code should be EXPLICIT and OBVIOUS about which variables/values are in pixel space (where arbitrary precision is not important), or in fractal space (where high precision is VERY important), and if it runs in the GPU (that does NOT natively support arbitrary precision)

- Data interchange: How do coordinates get passed between Rust (WASM), JavaScript UI, and URL parameters without precision loss? — correct, we need to research this.

## test precision

Zooming with the center of the zoom on the mouse location is one of the calculations that concerns me. In another project I have been working for days on getting this right and it's still not working, so I want to be extremely thorough here and ensure we have absolutely bomb-proof test suites. In my experience, AI agents tend to write tests that superficially do something useful, but are realistically designed to easily pass in an effort to make me happy. That is the OPPOSITE approach that I need. I want the test cases to be constructed from the ground up to test the mathematics PROPERLY from first principles. I EXPECT the tests to fail until we nail the code.