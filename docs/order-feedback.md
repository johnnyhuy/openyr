# Tactical order feedback

World clicks briefly acknowledge move, attack-move and attack orders with a
message and a destination marker. Messages count the commands produced by the
same resolver that chooses the contextual cursor. Mixed selections count only
eligible living owned units. An unavailable action submits no commands and
shows a red marker with a short explanation. Selecting units produces no order
acknowledgement.

Successful menu loads of saved matches also use the feedback strip. They show
the restored tick and bounded filename stem, with no tactical marker, only after
the worker result commits. Failure, cancellation, restart and fresh setup do not
report success. The notice shares the two-second expiry and lifecycle cleanup;
it changes no commands, simulation or save data (`docs/formats/menus.md`).

Move markers use the requested destination, including terrain height. The
pathfinder can substitute a nearby standable goal; the marker does not promise
arrival at the exact cell. Attack markers follow the target's current ground
position or building center and disappear if the target is removed. A sent
order can still lose its target or route before the simulation applies it.

S sends Stop to living owned selected units. Ctrl+H sends Harvest to selected
harvesters; their existing economy loop decides whether to start or continue
work. Both keys show the number of submitted commands or explain the required
selection. OS repeat, key release, inactive matches, focus loss and menus do
not submit these orders. Deliberate presses cancel pending pointer gestures
and placement/service/attack-move modes, preventing a stale mouse release from
issuing an additional order.

Notices last two seconds of monotonic presentation time. The latest tactical
order replaces the previous one. Selection, team shortcuts, right-click
cancellation, Escape, service/sidebar/radar actions, focus loss, window resize
and menu entry clear it. Match replacement and returning to the front menu also clear it.
Expiry requests a redraw even if the simulation has not advanced. The text
uses the existing GAME.FNT feedback strip inside the world viewport. Markers
are drawn below sidebar/tooltips/cursors and keep a fixed 28x14 pixel outer
boundary independently of world zoom. Green means move, gold attack-move and
red attack or unavailable, with text distinguishing those last two cases.

These colors, shapes, text and duration are native policies. They have not
been calibrated against original-game acknowledgements. Original calibration
and physical macOS/Retina input remain under #97/#98 and related UI issues.
No original code or additional artwork is used by this slice.

`order_feedback::tests` drives the shared click/release and keyboard handlers on
retail EB1. It checks resolver-identical commands, mixed ownership/stale ids,
unavailable actions, repeat/focus/modal guards, stale-release cancellation,
expiry and lifecycle cleanup. Nine zoom/layout combinations check marker
dimensions and height-aware projection. A controlled moving target checks
tracking and removal. Full match save bytes remain identical before commands
are stepped; the resulting simulation hash matches the same commands applied
without feedback. `OPENYR_ORDER_PROOF=<external-dir> cargo test -p yr-app --bin
openyr order_feedback:: -- --nocapture` exports shared-compositor frames and
tick/hash/command receipts. The PNGs contain retail art and stay outside git.


Hover previews are separate from the two-second order notice. In an active
world viewport, green corner brackets identify the owned unit that the shared
resolver can select. Red brackets identify the enemy unit or building that the
selected armed units can attack. Shift-only additive selection suppresses the
attack preview, matching the actual click route; Control+Shift retains attack
resolution. Empty/stale/unarmed selections do not advertise an attack.

Each composition resolves the current pointer and current entity position. No
hover id is retained across movement, destruction, match replacement or saves.
Focus loss, invalid/margin/sidebar pointers, menus, finished matches, mouse
presses/drags, edge-scroll arrows and placement/service modes suppress previews.
The existing selected-unit marker remains independent. Brackets follow the
height-aware ground anchor and footprint extent at the current map zoom, with
six-pixel corner strokes clipped to the world viewport. They draw above world
art and below the sidebar, tooltips and cursor.

This is a native visibility policy for #97. The colors, footprint brackets and
immediate appearance are not calibrated original-game output. Original target-highlight calibration and
physical Retina input remain open. Retail hover tests compare preview ids with actual select/attack
click results at three window sizes and three zooms, cover inactive/modal/drag
and unavailable-selection guards, and compare complete match save bytes before
and after pointer-only interaction. Optional `OPENYR_HOVER_PROOF=<external-dir>`
exports isolated before/after overlays and integrated compositor images.


Powered-radar move and attack-move orders reuse the same message, destination
marker and two-second expiry as world orders. The notice counts the command
requests submitted by the existing radar path; it does not promise pathfinding
success or change its selected-id command sequence. The marker uses the snapped
command destination and terrain-height projection. Offscreen destinations leave
the acknowledgement text visible without moving the camera. Radar orders keep
selection and map zoom. Empty-selection radar navigation and right-click radar
cancellation clear the notice; inactive, opening/unpowered and modal radar
input produces no new order notice. Other existing notice cleanup still applies.

Radar tests use normal retail production/placement to enable AMRADR, then drive
the shared guarded pointer route at 640x480, 1000x700 and 1280x800, with three
map zooms. Commands, selection, order-time camera position and complete match
save bytes are compared. Stepping the same command with and without feedback
produces identical complete snapshots. Optional
`OPENYR_RADAR_ORDER_PROOF=<external-dir>` exports visible/expired compositions,
input geometry and a controlled powered-radar save for live checks; these stay
outside git. The fixture disables AI only for observation. Radar acknowledgement
appearance and timing remain native policy pending original-game calibration.


Repair/sell clicks use the command resolved by the existing service policy.
A repair start, repair cancellation or sale request displays its corresponding
order-sent text and a green target marker. An unavailable target displays
"Cannot repair here" or "Cannot sell here" with a red ground marker. These are
acknowledgements of queued requests, not promises of completed repairs, refunds
or command acceptance on a later tick. A sold/removed building loses its entity
marker while the short message can remain until expiry. The service mode stays
active as before; right-click cancellation on the world or sidebar clears the
mode and notice without discarding already queued commands or selection.

The existing two-second notice lifecycle, fixed UI-pixel marker and measured
font rendering apply. Focus loss and menu entry clear the notice. Inactive,
modal or stale post-resize clicks cannot create a service notice or command.
No repair recipe, price, cadence, ownership/eligibility rule, command ordering
or save field changes. Original wrench/sale animation, sound loops and
acknowledgement styling remain uncalibrated under #86/#97/#98.

Retail tests use a controlled half-health GAPOWR snapshot with AI disabled for
observation. At three layouts and three zooms, guarded repair/sell input
matches the existing cursor resolver and command sequence, preserves selection,
zoom and complete save bytes, and retains queued commands through cancellation.
Equivalent live/reference command steps yield identical complete snapshots
after presentation events are consumed. Full-health/enemy targets, active-job
cancellation, expiry, focus/menu/resize guards and sidebar cancellation are
checked. `OPENYR_SERVICE_ORDER_PROOF=<external-dir>` exports shared compositions
and the controlled native save for live checks; the save stays outside proof
uploads and all derived images stay off main.


Structure-placement clicks now reuse the same brief feedback lifecycle. A
queued `PlaceProduced` request displays "Placement order sent" and a green
marker at the requested footprint center. Rejected clicks display "Cannot
place here" and a red marker at the clicked cell. Placement rules, READY queue
ownership, existing command ordering and automatic exit from placement mode
after a queued request are unchanged. The notice acknowledges a request, not
confirmation that the simulation accepted it on its next tick.

A rejected click retains the READY item and placement mode. World right-click
cancellation clears the mode/notice and redraws immediately without removing
already queued work or selection. Existing focus/menu cleanup and pointer
invalidation prevent inactive/modal/stale clicks from queuing orders. Even an
outdated placement slot or a mode whose READY item disappeared cannot announce
a queued placement. Markers use the existing terrain-height projection and
fixed UI-pixel geometry; their appearance and two-second lifetime are native
policy pending original-game calibration.

Retail placement tests produce a GAPOWR normally, then exercise READY cameo
pickup and world clicks at three layouts and zooms. Occupied ground rejects
placement; accepted commands match the unchanged session resolver and consume
the READY item only when stepped. Complete save bytes, selection, zoom, queued
command prefixes and equivalent post-step snapshots are checked alongside
cancellation, expiry, stale-mode and focus/menu/resize guards.
`OPENYR_PLACEMENT_ORDER_PROOF=<external-dir>` exports shared images and a
controlled native READY save with AI disabled for live observation. The save
stays outside proof uploads; derived art stays off main.


Sidebar production clicks now reuse the same brief feedback lifecycle. A
queued build displays "Build order sent". Holding the factory head displays
"Production held", resuming it displays "Production resumed", and cancelling
displays "Build order cancelled". A left-click that queues nothing because
prerequisites are unmet or the construction queue is occupied displays
"Cannot build here". Picking up a READY structure displays "Select a location
to place the structure" until the existing placement acknowledgement takes
over on the world click. Right-clicks that hold or cancel nothing stay silent.
The notices carry no world marker, since the cameo itself is the target.

Production rules, queue ordering, payment, refunds and saved state are
unchanged. The notice acknowledges the queued request, not completion of the
build. Existing focus, menu and pointer cleanup still clears it, and expiry
requests a redraw. Appearance and timing remain native policy pending
original-game calibration under #86 and #97.

Retail tests queue, hold, resume and cancel a Rhino on EB1 through the
sidebar handlers, compare the queued commands and pre-step save bytes, cover
the occupied construction queue and READY pickup, and check expiry.
`OPENYR_PRODUCTION_ORDER_PROOF=<external-dir>` exports shared-compositor
frames and tick, hash, message and pending-command receipts. The frames
contain retail art and stay outside git.


Selected living owned armed units now show red corner brackets around their
current living hostile attack targets, even after pointer leave or expiry of
the brief order notice. The renderer reads the simulation's existing attack
field each composition and draws each target once. This includes existing
automatic engagements as well as explicit attacks; it does not infer active
combat from a pending UI command. Position updates, selection changes, target
removal and restored attack state take effect without a UI target cache.

Current-target brackets share the hover preview's terrain-height projection,
footprint extent, six-pixel corners and world-viewport clipping. Large zoomed
footprints can put their corners outside the visible viewport. Focus loss,
menus, finished matches and placement/service modes suppress current-target
brackets. Fresh pointer motion is not required to restore them when the active
context returns. Pending Stop/Move commands change highlights only when the
simulation changes its attack state; normal automatic reacquisition remains
unchanged. Save/load uses existing attack fields and existing selection policy.

This is a native visibility policy for #97, not measured original-game target
highlight appearance. No command, combat rule, simulation field, save schema,
cursor sheet or timer changed. Retail tests cover unit/building targets, mixed
and stale selection, deduplication, three layouts/zooms, context guards,
restoration and removal. Two validated in-cell native poses isolate position
following from a retail guard's parked movement policy. Read/draw passes keep
complete match bytes unchanged; equivalent Stop steps keep snapshots identical.
`OPENYR_CURRENT_TARGET_PROOF=<external-dir>` exports shared compositions and a
controlled stationary durable-target save for live checks. The fixture disables
AI and uses ordinary native spawn/attack commands; its save stays outside proof
uploads and derived art stays off main.


Repair and sell modes now preview the building under the pointer. Green
corner brackets use the same footprint extent as the other hover previews
when the shared service resolver would queue a command for that building,
including cancelling an active repair. Red brackets mark a building the
resolver rejects, such as a full-health repair target or an enemy structure.
Ground, units and empty cells show no service preview. The placement ghost
already covers placement-mode hovering, so no extra placement bracket is
drawn.

The preview is recomputed from the current pointer cell on each composition.
No building id is retained across movement, destruction or match replacement.
Focus loss, menus, finished matches, mouse presses and drags, edge-scroll
arrows and placement mode suppress it. Current attack-target brackets stay
hidden while a service mode is active, as before. Reading the preview queues
no command and changes no simulation or save state.

This is a native visibility policy for #97, not measured original-game
output. Retail tests damage a GAPOWR through the public snapshot format,
then check both modes against the actual click commands at three layouts
and three zooms, covering full-health, damaged and enemy buildings, a unit
under the pointer, lifecycle guards and complete save-byte invariance.
Optional OPENYR_HOVER_PROOF=<external-dir> exports before, preview and
composed frames for the repair case at each layout.

## Attack-move key activation

A deliberate A press arms one attack-move action (#276, #97). OS repeats do
not re-arm it after an order or cancellation. Activation discards unfinished
pointer gestures and placement/repair/sell modes, retaining selection and
already queued commands. A stale mouse release cannot consume or cancel the
new mode; a fresh pointer action uses the existing command route. Control+A
in the Save editor remains a name-selection shortcut, through menu routing.

Asset-independent keyboard/gesture checks cover conflicting modes, late
releases, Escape/right cancellation, repeats, focus/synthetic guards and fresh
activation. Shared radar-handler fixtures verify AttackMove then ordinary Move
command intents. Optional `OPENYR_ATTACK_MOVE_KEY_PROOF=<directory>` exports
fallback compositor frames and state receipts from a synthetic canvas.
These fixtures have no Session: commands are inspected but not stepped, and
retail radar power/input, save reconstruction, original shortcut parity and
physical Mac/Retina behavior remain unverified. Simulation and save schemas
are unchanged.

Picking up a ready structure from its actual sidebar cameo replaces an armed
A attack-move action (#302). Placement success, invalid-placement recovery and
cancellation cannot restore the previous action. A deliberate new A press
still replaces placement, while ordinary production clicks keep their current
mode policy. This is native mode ownership, not measured original behavior.

Asset-independent native Session tests cover 27 layout/zoom/outcome combinations,
real placement and subsequent Move intents, fresh A activation and ordinary
unit production. They preserve full encoded saves, simulation hashes, selection
and already queued intents. `OPENYR_PLACEMENT_MODE_PROOF=<directory>` exports
actual synthetic compositor frames and cursor-kind/state receipts. The fixture
has no retail cursor sheet; it does not verify retail pixels, original-game
parity or physical Mac/Retina delivery.

## Unit selection and health

Selected units retain their health pips without the native white corner/square
outline (#343), as requested in the classic-UI review. Green/red contextual
hover and attack previews still use their existing shared raster and bounds.
Box-selection feedback and selection/command semantics are unchanged. Health is a four-pixel-high black frame with
two-pixel pips, one-pixel separators and a darker lower edge. Green, yellow and
red indicate the existing above-50%, above-25% and remaining health bands.
Zero health has no colored pip; positive fractions retain at least one pip.
Both overlays clip to the world viewport before the sidebar. Corner length,
bar height and pip pitch stay in UI pixels; footprint width follows map zoom,
with health width bounded to 127 pixels. Selected and damaged objects retain
their existing health visibility. Resolved hover previews also show health
(#335), including healthy unselected owned units, hostile attack targets and
repair/sell structure previews. Hover health uses the same target and guards as
the existing brackets, including non-actionable service-target previews. It
disappears when that preview ends; persistent attack-target brackets alone do
not reveal a healthy target's bar. No separate hit test or selection mutation
is introduced. This presentation does not change simulation or save formats.

The public [Red Alert 2 manual, printed pages 5-6](https://static.wikia.nocookie.net/cnc_gamepedia_en/images/3/3b/CCUC-RA2-pcMANOLeUS.pdf/revision/latest)
backs left-click selection/orders, right-click deselection/cancellation,
box selection and green movement/red targeting cursor roles. The existing
retail cursor sheet remains the source of cursor artwork when data is supplied.
The new status geometry, pip colors and health placement are a native
classic-style revision, not a pixel-calibrated original-game reproduction.
Public Steam store screenshots inspected for this revision hide the interaction
HUD and cannot establish selection/health geometry. Hotspots, animation cadence,
original health visibility/thresholds and physical Mac/Retina input remain open
under #97/#98. No original executable was run.

Synthetic shared-compositor checks exercise selected status at three layouts
and three map zooms, Attack/Select cursor kinds, right-click deselection and
left-click reselection with identical encoded Session bytes and no queued
commands. Raster tests cover empty/full and damage-band boundaries, clipping
and malformed/nonfinite inputs. Without retail files these checks do not
verify cursor artwork, sprite overlap or physical input delivery.

Command-modified keyboard input is rejected by the shared world dispatcher
before tactical order handling (#331). Command+S therefore cannot append Stop
or replace an armed action/gesture. Menu editing routes first, and documented
plain S/Ctrl+H behavior remains available. Rejected held-key repeats cannot
become world pan/zoom after Command release. The native isolation policy and
protected-file/Session checks are documented in `docs/window-input.md`; no
original-game Command semantics or physical Mac delivery are claimed.

Authored Session/compositor checks for hover health cover 45 layout/zoom/target
cases: owned selection, hostile units and structures, sell and full-health
unavailable repair previews. Thirteen guards cover pointer gestures, keyboard
pan, placement, menus, focus/leave, edge/sidebar ownership, Shift selection,
empty selection and ended matches. Exact existing pip raster comparisons,
hover/away captures and retained selected/damaged visibility preserve camera,
selection, pending orders and full encoded Session bytes. Set
`OPENYR_HOVER_HEALTH_PROOF` with `cargo test -p yr-app --bin openyr hover_health
-- --nocapture` for authored fallback frames and state receipts. This native
visibility extension is not original-game health-policy calibration.
