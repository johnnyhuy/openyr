# Camera input

Arrow keys and optional edge scrolling move the world camera at 600 framebuffer
pixels per second, divided by world zoom. Diagonal movement has the same speed
as movement along one axis. Redraw timestamps drive this presentation state;
simulation ticks, RNG and match saves are unchanged. The first redraw establishes
the clock, and a single delayed redraw contributes at most 100 milliseconds.

The Controls page in both the front menu and paused match offers an edge-scroll
toggle. It starts off and persists in the existing interface preferences file.
Older preferences without the field still load with edge scrolling off. This
setting belongs to the interface and is not part of a match save.

When enabled, edge scrolling uses the outer 12 framebuffer pixels of the world
viewport. The right edge is immediately before the sidebar; the sidebar itself
and any surface letterboxing do not scroll. Corners move diagonally. A viewport
narrower or shorter than 24 pixels has no edge zone. Window coordinates use the
existing pixels surface-to-framebuffer conversion, so map zoom does not change
the edge width. This does not add pointer confinement on macOS.

Edges require a focused window, an ongoing match, and fresh pointer movement
inside the window after focus loss, window leave, menu entry or gesture
cancellation. Menus, left/right presses, box selection, right dragging and radar
dragging suppress edge movement. Arrow panning takes priority and cancels mouse
gestures; zooming also cancels gestures. Releasing a stale press after either
operation cannot issue a world order. Right dragging retains its own pixel-delta
movement and applies the same bounds.

Every live redraw constrains the visible world rectangle to the complete rendered
map raster, excluding sidebar width. If the viewport exceeds a raster dimension,
that axis stays centered with equal margins. Radar and team centering still
request the drawn ground position, including terrain height; the redraw clamps
requests near the boundary. Zoom retains the existing top-left anchor until
the bounds require adjustment. Bounds do not crop to LocalSize or the isometric
playable diamond, so authored border terrain and raster padding remain visible.
The rectangular bounds, speed, edge width, default, zoom anchor and stall cap are
native policies pending controlled original-game observation in #97/#98.

Asset-free tests cover equal one-second movement at 30/60/144 Hz, zooms
0.25/1/1.25/4, diagonal speed, delayed frames, oversized viewports, raster bounds,
sidebar exclusion, invalid pointer coordinates and preference compatibility.
The retail EB1 test drives the shared redraw/zoom/menu handlers, checks focus,
modal and gesture guards, and compares complete match save bytes before/after.
`OPENYR_CAMERA_INPUT_PROOF=<external-dir> cargo test -p yr-app --bin openyr
camera::tests::retail_camera_handlers -- --nocapture` exports shared-compositor
frames and camera/tick/hash receipts outside git. The off-raster request frame
shows the old unrestricted camera behavior before the new redraw constraint.
Physical macOS input, display resizing and Retina delivery remain in #97;
animated edge-arrow cursors and original calibration remain in #98.
