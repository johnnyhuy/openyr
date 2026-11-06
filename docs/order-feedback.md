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
been calibrated against original-game acknowledgements. Structure services,
placement feedback, persistent ordered-target highlighting
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
immediate appearance are not calibrated original-game output. Persistent
ordered-target highlights, service/placement previews and physical Retina input
remain open. Retail hover tests compare preview ids with actual select/attack
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
