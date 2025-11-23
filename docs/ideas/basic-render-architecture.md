# Basic render architecture
 
 You are an EXPERT software architect, with deep knowledge and experience of Rust, Leptos, and elegant, clean
 abstractions. This document describes a basic render pipeline architecture, based on a lot of work we have
 done previously.

 ## Structure

 We consider the following separate crates:

 - fractalwonder-ui
   - Leptos web app
   - runs on main thread
   - manages app, UI, state, config, and user interaction
   - manages pushing pixels onto an HTMLCanvasElement
   - manages a web worker pool
   - can import fractalwonder-core BUT **NOT** fractalwonder-compute
- fractalwonder-core
  - core mathematical and geometrical types, dealing with pixel and fractal space, arbitrary precision floats,
    and transforms
  - can **NOT** import from fractalwonder-compute and fractalwonder-ui
- fractalwonder-compute
  - pure render computation pipeline
  - runs in web worker threads
  - deals PURELY in fractal space, NOT in canvas pixel space
  - ALL COMPUTATION USES BIGFLOAT and supports arbitrary precision
  - can import from fractalwonder-core but **NOT** from fractalwonder-ui

NOTE: Many of the abstractions below are already implemented in _archive. We must be careful copying/pasting these as
they were not built for arbitrary precision, but they still contain extremely valuable architectural patterns that we
want to use.

## UI layer architecture

On the "main thread" side, the render pipeline deals with taking the results of computations and converting that to
RGB values that are pushed onto an HTML canvs.

core abstractions:

- InteractiveCanvas
  - leptos component that has an HTMLCanvasElement and a CanvasRenderer (an abstract Trait)
  - has use_canvas_interaction hook to manage user interaction (pan/zoom). This hook supports realtime interaction
    previews.
- CanvasRenderer
  - abstraction that takes a Renderer (producing arbitrary data per pixel) and a Canvas and converts results
    to RGB values for an HTMLCanvasElement
- Colorizer
  - function that takes data (which is separate per type of fractal we compute) for one pixel and returns an RGBA value
- Config
  - application config data structure that contains
    - a list of fractal types we support, e.g. TestImage and Mandelbrot
    - Each fractal type has a name, description, and a viewport (center coordinate, width, height) that represents
      the fractal space visible in a canvas at 1x zoom, e.g. for Mandelbrot this typicall is center (-0.5,0), widht
      width and height 4.0.
    - Each fractal we support has a list of Colorizer functions:
      - display name
      - ID (a string)
      - reference to a colorizer function that takes Data specific to a particular fractal (e.g. Mandelbrot or
        TestImage)
- App
  - the main app component.
  - Contains the UI and the InteractiveCanvas. 
  - maintains state (e.g. stores the last rendered viewport and other info per supported fractal type in local
    storage)
- UI
  - automatically shows/hides
- TilingProgressiveWebWorkerCanvasRenderer
  - manages a web worker pool to run computation on worker threads
  - manual message passing from/to workers to main thread
  - Creates a queue of tiles, radiating from the center outwards
  - tile size is adaptive: larger for lower zoom, smaller for higher zoom
  - rendering cancels immediately on interaction by killing and recreating worker threads

## Compute Layer architecture

This is the core abstract compute rendering engine. It works PURELY in fractal space coordinates and supports arbitrary
precision by ONLY using BigFloat numbers and arithmetic operations.

- Renderer
  - a trait that takes a viewport and other parameters and returns a list of Data (relevant calculated values for
    points)
- PixelRenderer
  - an abstraction that implements a Renderer by iterating over x,y values and computing data for each point
- PointComputer
  - a trait that takes x,y values and other parameters and computes Data for a point. For example, this is
    what is used to implement the mandelbrot formula

