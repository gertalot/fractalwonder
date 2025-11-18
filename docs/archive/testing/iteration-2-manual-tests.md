# Iteration 2 Manual Browser Tests

**Feature:** Progressive Rendering (Single-Threaded)

**Date:** 2025-11-17

## Test Environment
- Browser: Chrome/Firefox/Safari
- URL: http://localhost:8080 (trunk serve)

## Test Checklist

### Progressive Display
- [ ] Tiles appear one by one during render (not all at once)
- [ ] Progress is visible (tiles gradually fill the canvas)
- [ ] No blank screen while waiting for render

### UI Responsiveness
- [ ] Can click dropdown menus during render
- [ ] Can hover over UI elements during render
- [ ] Mouse movements are smooth during render
- [ ] No perceptible lag in UI interactions

### Cancellation
- [ ] Pan (click-drag) immediately stops current render
- [ ] Zoom (scroll wheel) immediately stops current render
- [ ] New render starts immediately after cancellation
- [ ] Cancellation happens within 100ms (feels instant)

### Render Quality
- [ ] Tiles align correctly (no gaps or overlaps)
- [ ] Colors are consistent across tile boundaries
- [ ] Full canvas is eventually rendered
- [ ] Cached re-colorization works (change color scheme without re-computation)

### Edge Cases
- [ ] Resize browser window → renders correctly
- [ ] Very small viewport → renders correctly
- [ ] Rapid pan/zoom → no crashes, renders stay responsive
- [ ] Switch renderer during active render → cancels and restarts

## Test Procedure

1. **Start dev server:** `trunk serve`
2. **Open browser:** Navigate to http://localhost:8080
3. **Initial render:** Wait for default view to render
   - Observe: Tiles appear progressively
   - Verify: UI responsive checkbox
4. **Test pan:** Click and drag during render
   - Observe: Render stops immediately
   - Verify: New render starts at new position
5. **Test zoom:** Scroll wheel during render
   - Observe: Render stops immediately
   - Verify: New render starts at new zoom level
6. **Test UI:** Open dropdown menu during render
   - Observe: Menu opens without delay
   - Verify: Menu stays open, render continues
7. **Test rapid interaction:** Pan/zoom quickly multiple times
   - Observe: Each interaction cancels previous render
   - Verify: No crashes, UI stays responsive

## Pass Criteria

- ✓ All checklist items pass
- ✓ No console errors
- ✓ Subjective feel: UI is responsive and smooth
