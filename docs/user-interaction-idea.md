Let's design a feature "user interaction with realtime preview imagedata pan and zoom".
This is a modular function that I can use with a canvas. It responds to the following user interactions:

1. user drags (move the pointer while the button is pressed)
2. user zooms (uses the mouse wheel in either direction, measuring deltaY)
3. user changes the canvas size (e.g. resizes the window)
4. user changes the pixel density (e.g. moves window to a different display)

The behaviour is as follows:

1. dragging the pointer x,y pixels moves the image on the canvas x,y pixels IN REAL TIME
2. using the mouse wheel accumulates wheel events, resulting in a zoom factor, which scales the image on the canvas in
   REAL TIME
3. the "zoom" functionality is centered on the CURRENT location of the pointer
4. the canvas imagedata is updated in realtime (using an animation loop), so the image follows the pointer and scales
   with the wheel.
5. if the image is dragged 1000 pixels to the left, the "gap" on the right side during the preview should be the
   background color. But if **then** the image is dragged 1000 pixels to the right, the image preview should still
   remember what was shifted off the visible screen, and the end result should be that the full image is back where it
   was.
6. When the user stops a drag or zoom or resize motion, but starts a new one WITHIN 1.5 seconds, the user interactions
   with live previews will continue
7. if the user stops interacting for 1.5 seconds or longer, this counts as the end of the interaction. The final
   transformation matrix + x,y offset + scale factor are returned to the consumer, who can then trigger a full re-render
   of the image with the new parameters.
8. See @\_ai/user-interaction-plan.md and @\_ai/example-src/hooks/use-fractal-interaction.ts for inspiration.

architecture notes:

This feature must be implemented WITHOUT REFERENCE TO A PARTICULAR RENDERER OR IMAGE COORDINATES. All transformations
occur and are returned in pixel coordinates. That keeps this function fully modular and generic, and allows us to
provide realtime previews for pan/zoom for canvases with all sorts of data (fractals, maps, game worlds, etcetera)

The idea behind this feature is that we provide realtime interaction feedback to the user so they can pan and zoom the
image how they want, and only trigger a potentially very expensive re-render after they have stopped interacting.
