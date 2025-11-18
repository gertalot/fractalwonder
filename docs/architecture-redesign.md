# architecture redesign

I don't like the way the current type/trait/component abstractions work. I want something like this:

- `App` component has the `UI` and an `InteractiveCanvas`
- `InteractiveCanvas` component accepts a `canvasRenderer` which `App` supplies
  - it contains an HTML `canvas` element
- `TilingCanvasRenderer` is a thing with a `render` method. It also accepts:
  - `putImageDataCallback` - supplied by `InteractiveCanvas`; basically the `canvas` `putImageData` method. Used
    by the CanvasRenderer to update the image data during progressive renders
  - has a way to stop any (parallel / background) render in progress
  - On creation, accepts a `Colorizer`. This is called to convert "Data" (from a `Renderer`) to RGBA
  - Also accepts a `Renderer`.
  - both Colorizer and Renderer are supplied by App
- `Renderer` renders `Data`. We have a `PixelRenderer` and an `ImagePointComputer` combined, they implement
  iterating over x,y coordinates and computing the appropriate `Data` for each pixel.

`TilingCanvasRenderer` coordinates tiling, rendering, parallel computation, and colorizing. It also stores raw
ImagePointComputer values and handles optimisations to avoid re-computing when possible

Basically, what I want to do is:

- App gets created. This contains a UI component and an InteractiveCanvas component
- Either by default or changed by the UI component, from a list of possible ImagePointComputer entities, we select
  the one we want (e.g. MandelbrotImagePointComputer). This should give us an object or function with the
  ImagePointComputer trait.
- Either by default or changed by the UI, we get a Colorizer function.

NOTE: How can we change the Colorizer function without blowing away any computed data in the TilingCanvasRenderer
(since that stores data that was expensive to compute)?

NOTE: INITIALLY we want to use the `compute_point_color` function from the TestImageRenderer to render a checkerboard
pattern with circles and a vertical line. BUT INSTEAD OF RGBA values this should follow the NEW pattern where it
computes Data that can be colorized. For the test image we can just do something like:

- we have a PixelRenderer and give it a testImagePointComputer which returns TestImageData
- TestImageData just has a u8 or another small data type
- We have a TestImageColorizer that takes a u8 and returns RGBA

NOTABLY: We GET RID of the current TestImageRenderer component because it enforces totally the wrong component
hierarchy. The actual canvas should live in InteractiveCanvas and there should be a TiledCanvasRenderer that "hooks"
into the InteractiveCanvas, so:

- App
  - Renderer (actually a PixelRenderer)
      - ImagePointComputer (actually a TestImagePointComputer)
  - InteractiveCanvas
    - canvas element
    - Colorizer
    - Renderer
  - UI

NOTE: The UI should be able to update the type of renderer, AND the colorizer, AND it should be able to trigger a
re-render if its viewport, center, or zoom parameters have changed

NOTE: The UI should **also** react to changes to viewport, center, or zoom parameters, or other RendererInfo, so the UI
can reflect the current information to users.
