# Tactical order feedback

World clicks briefly acknowledge move, attack-move and attack orders with a
message and a destination marker. Messages count the commands produced by the
same resolver that chooses the contextual cursor. Mixed selections count only
eligible living owned units. An unavailable action submits no commands and
shows a red marker with a short explanation. Selecting units produces no order
acknowledgement.

Move markers use the requested destination, including terrain height. The
pathfinder can substitute a nearby standable goal; the marker does not promise
arrival at the exact cell. Attack markers follow the target's current ground
position or building center and disappear if the target is removed. A sent
order can still lose its target or route before the simulation applies it.

S sends Stop to living owned selected units. H sends Harvest to selected
harvesters; their existing economy loop decides whether to start or continue
work. Both keys show the number of submitted commands or explain the required
selection. OS repeat, key release, inactive matches, focus loss and menus do
not submit these orders. Deliberate presses cancel pending pointer gestures
and placement/service/attack-move modes, preventing a stale mouse release from
issuing an additional order.

Notices last two seconds of monotonic presentation time. The latest tactical
order replaces the previous one. Selection, team shortcuts, right-click
cancellation, Escape, service/sidebar/radar actions, focus loss and menu entry
clear it. Match replacement and returning to the front menu also clear it.
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
immediate appearance are not calibrated original-game output. Original target-highlight calibration, service/placement hover previews and
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
