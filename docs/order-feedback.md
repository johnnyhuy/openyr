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
been calibrated against original-game acknowledgements. Radar orders,
structure services, placement feedback, persistent target/hover highlighting
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
