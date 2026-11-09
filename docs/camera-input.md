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
Linux resized-window margin routing and compact-client framebuffer sizing are
checked in [window-input.md](window-input.md). Resize changes the viewport
bounds while preserving map zoom; initial match centering uses that viewport.
Physical macOS input and Retina delivery remain in #97;
original cursor calibration remains in #98. Native static green/slashed
edge arrows now use the same guarded, bounds-aware direction as camera motion
(`docs/formats/cursors.md`). A partially blocked corner moves along its
remaining axis at cardinal speed; a fully blocked arrow consumes left clicks
without a world command, as do green scroll arrows.

## Right-drag activation

Right dragging crosses its native click-jitter threshold when Manhattan
displacement from the press exceeds two framebuffer pixels (#259, #97).
Activation uses the press position rather than individual event deltas, so
one-pixel motion events can form a drag. The first pan includes all accumulated
press displacement; subsequent pans use incremental event deltas, divided by
world zoom and constrained by the existing raster bounds.

Once activated, a gesture stays a drag until release or invalidation, including
when returning to the press position or moving against a map boundary. Its
release preserves selection. Motion confined within the threshold remains a
right click and retains the existing mode/selection cancellation. Focus, leave,
resize, modal, zoom and keyboard-pan cleanup retain their existing policies.

Asset-independent handler regressions cover 27 layout/zoom/direction cases,
small-step out-and-back motion, click jitter, bounds and stale releases.
`OPENYR_SMALL_DRAG_PROOF=<directory>` exports four shared-compositor frames and
state receipts from a synthetic colored-grid fixture, without a session or
retail art. These check camera transformation and transient selection/queued
command preservation, not retail simulation/save reconstruction or physical
macOS/Retina delivery. Original-game threshold calibration remains open.

## Camera keys interrupt gestures before redraw

World arrow-key dispatch cancels existing pointer tracking immediately (#284,
#97), before the redraw handler computes movement. A mouse release arriving
between key-down and redraw cannot finish the old box selection or right click.
Held-arrow redraw cancellation remains in place. Camera speed, elapsed-time
policy, zoom and bounds are unchanged; selected IDs, queued intents and command
modes survive this pointer cleanup.

The existing focused/synthetic-key and suppressed-repeat guards run first.
Menu arrows retain menu navigation. Key-up clears the held key normally, and a
fresh pointer press can start a new gesture. This is a native event-ordering
policy, not original-game timing calibration.

Asset-independent native dispatch checks cover 48 arrow/gesture/layout cases,
stale releases before any redraw, focus synchronization, menu navigation and a
fresh box after key-up. `OPENYR_CAMERA_KEY_PROOF=<directory>` exports actual
fallback compositor frames and state receipts. The synthetic fixture has no
Session or retail art, so it does not establish retail command/save behavior,
physical Mac/Retina delivery, audio or original-game parity. Simulation and save
formats are unchanged.

## Held keyboard pan owns new world gestures

A nonzero held-arrow direction rejects new world Left/Right presses before
pointer routing (#286, #97). An ignored click, box or right drag cannot change
selection, queue an order, cancel a command mode or move the camera between
redraws. Mouse releases stay inert because no gesture was armed. Pointer motion
still updates hover position, while camera movement remains in the existing
time-based redraw path.

Input ownership and redraw use the same net direction. Opposing arrows with
zero net direction allow world gestures. Releasing the last effective arrow
allows a fresh press immediately. The sidebar and menus keep their existing
routes; keyboard panning does not disable production or menu navigation. This
is native interaction policy pending original-game comparison, not pointer
confinement or a change to camera speed, bounds, simulation or save formats.

Asset-independent handlers cover 60 arrow/layout/command-mode combinations,
new left-click/box and right-click/pan sequences before any redraw, opposing
keys, key-up recovery, sidebar routing and menu navigation.
`OPENYR_KEYBOARD_PAN_PROOF=<directory>` exports synthetic shared-compositor
frames and input receipts. The fixture has no Session, entities or retail art;
it preserves transient selection/queued-intent sentinels rather than verifying
retail commands or reconstructed saves. Physical Mac/Retina input, audio and
original-game parity remain unverified.

World feedback now follows the same held-key ownership (#298): a nonzero net
arrow direction suppresses actionable cursor kinds and prospective hover
brackets before redraw. A tracked right press suppresses these previews even
before it becomes a pan. Key-up restores the resolver result without clearing
selection, intents or command modes. Asset-independent native Session tests
check actual queued orders and unchanged encoded saves, extending the earlier
Session-free camera dispatch coverage. The cursor format notes describe the
synthetic visual proof and its limits.

## Base navigation

The public [RA2 manual, Basic Keyboard Functions, printed page 2](https://static.wikia.nocookie.net/cnc_gamepedia_en/images/3/3b/CCUC-RA2-pcMANOLeUS.pdf/revision/latest)
maps H to Center tactical map on base. OpenYR now uses a deliberate H to center
the bounded world viewport on the drawn ground center of its lowest-ID living
owned construction yard (#318). Terrain lift, zoom and sidebar width use the
existing camera projection. If no owned yard is available, the view stays put
and the feedback strip explains that no construction yard is available.

This intentionally changes the earlier native H binding. Harvest remains on
Ctrl+H with the same eligible-unit commands, acknowledgement and command-mode
cleanup. S still stops selected units. Menu name fields still receive ordinary
H text; repeats, synthesized/unfocused events, tracked Shift/Command chords
and ended matches cannot trigger base navigation. Ctrl+Shift+H and
Control+Command+H do not issue either action.

Base navigation cancels old pointer/edge tracking and team-recall timing. It
preserves selected units, assigned teams, queued valid commands, zoom and armed
placement/repair/sell/attack-move modes, as the existing camera keys do. An old
mouse release cannot issue an order or clear the retained selection. The notice
uses the existing native two-second feedback channel and has no world marker.
Simulation, match saves and camera speed are unchanged.

The manual establishes H's action. Choosing the oldest owned yard, having no
other-building fallback or yard cycling, notice wording/timing, raster bounds
and the Ctrl+H alias are native policies pending original comparison in #97.
Authored Session regressions cover elevated target projection at three layouts
and zooms, bounds/oversized views, multiple/destroyed/enemy yards, pointer
releases, mode preservation, input guards, Save text and actual Ctrl+H Harvest
commands with exact Session bytes. These do not establish retail pixels,
original-game timing, physical Mac/Retina delivery or audio-device behavior.
