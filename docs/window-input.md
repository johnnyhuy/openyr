# Window and framebuffer input

The native window starts at 1280x800 logical pixels and requests a 640x480
logical minimum. Its framebuffer follows the client logical dimensions, capped
at 1280x800 and bounded by physical pixels. This lets compact clients draw the
complete menu and sidebar instead of cropping a fixed framebuffer. Larger
clients retain the existing integer-scaled presentation through `pixels`. Map
zoom is a separate framebuffer-space camera setting. Input arrives in physical window
coordinates and must pass `Pixels::window_pos_to_pixel` before menu, sidebar,
radar or world handlers run.

A failed conversion returns signed coordinates outside the drawn frame. Those
coordinates are not valid targets. Nonfinite input, failed mappings and mapped
positions outside the half-open framebuffer bounds make the pointer inactive.
Pointer buttons and wheel events require both focus and an active in-frame
position. Empty or nonfinite menu wheel deltas have no effect. Keyboard menu
navigation remains available independently of pointer presence.

Window resize, scale-factor changes and cursor enter/leave invalidate the old
pointer position. Focus loss does the same and retains the existing cleanup of
held keys and command modes. Invalidation clears button presses, selection and
pan gestures, radar dragging, menu sliders/popups and map-list dragging. A
release after invalidation cannot become an order or activate a control, even
if a later valid movement lands on the old target. A new press is required.
Selection, queued valid commands and the current match are preserved.

Fresh valid motion restores pointer ownership. The same `Presence::active`
gate governs input and custom cursor eligibility; the system pointer remains
visible in margins or while inactive. Menu motion also updates the last valid
position, so a right drag immediately after entering a match uses a local delta
rather than an old menu-era coordinate. No simulation or save fields change.

## Verification

Shared-handler tests cover failed mappings, negative and far-edge coordinates,
nonfinite values, unfocused input, zero/invalid wheel deltas, stale releases,
menu activation and the first right-drag delta after menu entry. A retail EB1
case selects an actual owned unit through the guarded button route, rejects
stale/outside gestures and compares complete match save bytes unchanged.

The Linux live check runs Xorg, the app and capture in one task session. A
baseline release from `f7dae46` and the changed release both navigate through
Main Menu, Single Player, Skirmish and Customize Battle. Each window is resized
to 1600x1000 and 1920x1200. The pointer moves into the physical top margin, then
receives a wheel step. Cropped map-list hashes change before the fix and remain
identical after it at both sizes. A wheel step after fresh in-frame motion
changes the list in both releases. Screenshots, videos and JSON receipts record
the actual public X11 input and physical client sizes; no original executable
or game process memory is inspected.

This verifies Linux resized-window routing and rendering. Shared invalidation-handler checks do not establish physical macOS Retina delivery, focus behavior
or pointer confinement. Those checks and original cursor cadence/hotspots remain
open under #97/#98. The historical margin check above covers #223.

## Compact clients (#224)

Resize and DPI events update both the physical surface and bounded logical
framebuffer. Zero-sized minimized clients keep their last usable buffer;
invalid scale factors do not allocate. Drawing, menu clicks and slider drags,
sidebar hits and camera centering share the actual framebuffer dimensions,
including while the renderer temporarily owns the pixel buffer. Resize clears
old gestures and preserves map zoom. Forced clients below the requested minimum
remain bounded, but the supported usable minimum is 640x480 logical pixels.

Tests cover fractional and doubled scales, extreme dimensions, minimization,
menu activation/dragging and zoom preservation. Retail EB1 checks at 640x480,
1000x700 and 1280x800 traverse start/loading, select a sidebar category and an
owned world unit, then pause/resume through the guarded pointer handlers.
Complete match save bytes remain unchanged. Shared-compositor images document
each stage. Live Linux X11 before/after captures at 640x480 and 1000x700 verify
that the complete menus are visible and reachable through real pointer events.
Synthetic doubled-scale checks do not verify a physical Retina display. Native
macOS resize/DPI delivery remains a follow-up under #97.

## Keyboard focus synchronization

The window routes keyboard input through `keyboard_input.rs` before either
menu or world handlers (#255, #97). Winit 0.30 documents synthesized key presses
on focus gain and releases on focus loss on X11 and Windows
(`WindowEvent::KeyboardInput::is_synthetic`). These presses describe keys
already held; they are not deliberate menu choices or game commands.

Synthesized presses and all unfocused presses are ignored. The affected
physical key's subsequent repeats are suppressed until its release or a fresh,
nonrepeated focused press. Releases always clear held/suppressed keys, including
while a menu is visible or the window is unfocused. Ordinary focused navigation,
name editing, repeats, shortcuts and world commands retain their existing
routing. Selection, pending commands, simulation and save schemas are unchanged.

Asset-independent regressions exercise the native dispatch function directly.
A listed temporary `.oyrsave` file survives synthesized Enter with Yes selected,
then is deleted by deliberate Enter through the same menu/backend path. Other
checks cover F5/F9, command modes, camera keys, Escape, menu navigation/editing,
follow-on repeats and release cleanup across focus/modal changes. These are
handler tests, not physical desktop delivery or retail-save reconstruction.
Original-game behavior and physical macOS/Retina input remain unverified.

## Conflicting mouse gestures

A right press in the match cancels unfinished pointer gestures before routing
its ordinary world, sidebar, repair/sell or placement action (#263, #97).
This clears held left/radar gestures and immediately redraws any rubber band.
Existing selection remains until the route's existing cancellation policy
applies; already queued commands remain queued. Menus retain their own routing.

A world right-click/pan owns the pointer until release. Left presses during
that ownership cannot arm another world selection or activate sidebar/service/
placement controls. Either release order stays inert for that ignored left
press. A fresh left press after the right gesture ends starts normally.
Focus/leave/resize and other existing invalidation still discard stale releases.
This is native input policy, not original-game button-chord calibration.

Asset-independent handler checks cover three layouts and five right-action
routes, click/pan release order, recovery and queued-command sentinels.
`OPENYR_CANCEL_GESTURE_PROOF=<directory>` exports four actual shared-compositor
frames and state receipts from a synthetic canvas. It shows a rubber band
removed on right press, with no restoration by right/late-left release. There
is no session, entity or retail data in that fixture; it does not establish
retail command/save invariance or physical Mac/Retina delivery.

## High-resolution menu wheel input

Menu lists and scrollable dialogs accumulate finite vertical wheel motion into
whole rows (#268, #96, #97). One line is one row; 50 pixel units are one row,
matching the existing native sidebar conversion. Fractional rows carry between
wheel events, and multi-row events retain their magnitude. Each event is bounded
to 128 rows; zero and non-finite deltas are inert. Clamped whole-row overscroll
is discarded so it cannot delay the next reversal.

Page transitions, pointer/focus/surface invalidation, keyboard input and a new
menu pointer press discard the partial gesture. Scrolling does not change a
selected save path, editor name, list selection or confirmation target. The
fraction is transient UI state; simulation and save encoding are unchanged.

Asset-independent dispatch regressions cover pixel/line sequences, dialog and
save-list rows, list bounds and gesture isolation. Optional
`OPENYR_MENU_WHEEL_PROOF=<directory>` exports actual fallback compositor frames
and row-state receipts. These are synthetic handler checks without a Session;
they do not establish retail UI parity or physical Mac/Retina trackpad delivery.

## Keyboard interrupts menu pointer tracking

A deliberate focused key press reaching the menu handler cancels its pending
mouse button, Skirmish slider drag and map-list thumb drag (#274, #96, #97).
Keyboard navigation/editing then runs normally, including open-popup choices.
Later pointer motion or a stale release cannot continue the old gesture or
activate its button. A new pointer press starts a normal fresh gesture.

Synthesized/unfocused and suppressed repeat presses retain their existing
keyboard-dispatch guards. They do not reach this interruption path; actual
focus loss already clears pointer tracking. Menu values, selected paths,
configuration, simulation and save encoding do not change merely from cancelling
tracking. Existing explicit keyboard actions still run.

Asset-independent handlers verify temporary-file preservation when Tab moves
away from a held Delete-confirmation button, fresh deliberate deletion, both
sliders and map-thumb isolation at three sizes, and popup/synthetic-key behavior.
`OPENYR_KEYBOARD_TRACKING_PROOF=<directory>` exports actual fallback composer
frames and state receipts without a Session or retail art. These tests do not
establish physical Mac/Retina input delivery or original-game tracking parity.

## Box selection stays in the world viewport

Box picking and rubber-band painting share a rectangle intersected with the
visible world viewport (#278, #97). Its right boundary excludes the sidebar;
right/bottom viewport edges are half-open. Drag endpoints remain inclusive
inside that viewport. A drag over UI controls cannot paint across them or
select unit ground anchors hidden behind them. Dragging back into the world
continues the same gesture, and additive selection retains its existing policy.

The shared picker keeps owner filtering and height-aware projection, applies
camera/zoom before testing visibility, and only returns IDs. It does not mutate
the simulation, snapshots or command queue. Invalid/non-finite extents and
viewports without world pixels yield no rectangle; drawing loops stay bounded.

Asset-free tests check actual compositor sidebar pixels, reversed/degenerate
rectangles and fractional edge points. Synthetic Sim fixtures check visible,
UI-hidden and enemy anchors at three sizes/zooms, preserving full Sim snapshot
bytes and state hash. `OPENYR_SELECTION_BOX_PROOF=<directory>` exports fallback
frames and geometry/Sim receipts. These are component checks without a complete
Session or retail art, not retail save reconstruction, original selection
calibration or physical Mac/Retina input verification.

## Box picking follows exact unit positions

The box picker uses each unit's raw fixed-point x/y divided by 256, matching
the unit renderer before height-aware projection (#280, #97). It no longer
snaps moving or other subcell positions to the current cell center. A box over
the drawn ground anchor can select the unit; an isolated box over the old
cell-center anchor cannot. World-viewport clipping and owner filtering remain.

Synthetic Sim tests cover 216 size/map-scale/zoom/height/position cases and eight
actual movement ticks. Picking preserves complete post-tick Sim snapshot bytes
and state hashes. The earlier viewport fixture now centers on the raw position
of its integer-coordinate unit, retaining its hidden/enemy-anchor assertions.
`OPENYR_SUBCELL_SELECTION_PROOF=<directory>` exports coordinate and Sim receipts;
the terminal recording is the proof for this picking-only slice. No complete
retail Session, retail save reconstruction, original-game comparison or physical
Mac/Retina input is established. Simulation and save schemas are unchanged.

## Repair/sell mode switches cancel pointer tracking

Repair/sell mode changes use the shared pointer-cancellation path (#282, #97).
They clear the pan-active flag as well as held buttons, radar dragging and old
edge arming. A K/L press during a pan therefore cannot leave hover feedback or
a later selection box suppressed. The current pointer position remains valid;
a new pointer press can begin a normal gesture, and old releases stay inert.

Mode-toggle behavior, selected IDs and queued command intents are preserved.
Repeat, synthesized, unfocused and modal key guards retain their existing
routing. Native dispatch tests cover 72 mode/gesture/layout combinations and
an actual right-pan, service-on/off, fresh-box sequence in the shared composer.
`OPENYR_SERVICE_TRACKING_PROOF=<directory>` exports fallback frames and state
receipts. These fixtures have no Session or retail art; retail service-command,
hover-art, save reconstruction, original tracking and physical Mac/Retina checks
remain unverified. Simulation, recipes and save schemas are unchanged.

## Command keys stay outside world keyboard actions

Menu routing precedes the native Command/Super guard (#331). Supported menu
editing, including Command+A name replacement, still runs there. In a closed
match menu, Command-modified keys cannot submit Stop, arm tactical/service modes,
open save/load/pause, toggle mute, change zoom, start keyboard pan or select
units/categories/teams. Tracked Control/Shift combinations with Command retain
that isolation. Plain and documented Ctrl/Shift bindings keep their behavior.

Rejected keys stay tracked as held input and their repeats are suppressed until
release or a fresh deliberate press. Releasing Command alone cannot turn an old
repeat into a plain world action. A rejected repeated key is also removed from
active pan ownership; other held keys keep their existing policy. Fresh rejected
chords end old team/type press timing, including Command+T. Repeated, synthetic,
unfocused and modal events retain their earlier dispatch rules.

These key events preserve selection, pending orders, current pointer gestures,
armed modes, camera, tactical feedback, existing save/preference files and
encoded Sessions. The guard changes keyboard routing only; it does not block
pointer/wheel events or alter simulation/save schemas. Command isolation is a
native desktop policy, not original-game modifier calibration.

On unchanged production code, three regressions fail and the positive menu/plain
command case passes. Authored checks cover 60 layout/key/chord combinations,
27 actual save/preference/menu/camera cases, delayed pan/zoom repeats and fresh
recovery, documented Ctrl+H and menu Command+A replacement. Full Session bytes
and protected file bytes remain exact after the fix. Set
`OPENYR_COMMAND_WORLD_PROOF` with `cargo test -p yr-app --bin openyr
command_world -- --nocapture` for actual before/after shared-compositor frames
and pending-command receipts. Retail/original, physical Mac/Retina and
audio-device checks remain unverified under #97/#98.

## Held sidebar category ownership

The category-tab gesture introduced in #355 now participates in the shared
world-pointer, edge-scroll and match-wheel guards (#356). The origin tab owns
input until Left release or normal pointer cleanup, even when dragged into the
world and its held artwork is no longer visible. Such a drag cannot advertise
Select/Attack hover, pan at an edge, zoom the map or scroll a sidebar category.
Release restores normal routes without arming a world click; fresh clicks,
edge motion and wheel input work normally. No simulation, snapshot, input
binding, camera-speed or original timing policy changes.

On unchanged production code, two authored native tests fail on leaked Select
and edge-scroll cursors. Fixed handlers cover twelve layout/zoom/select/attack
sequences, six edge cases and 24 world/sidebar wheel cases, including exact
Session-byte, selection, camera and queued-intent preservation while held.
Existing tab tests retain pointer/focus/modal/right-click/shortcut cancellation
coverage. `OPENYR_SIDEBAR_OWNERSHIP_PROOF=<directory>` with
`cargo test -p yr-app --bin openyr sidebar_tab_ownership -- --nocapture` exports
held/released shared-compositor frames and cursor/state receipts. These are
authored fixtures, not retail or original-game comparisons. Physical
Mac/Retina delivery and original gesture calibration remain open in #97/#98.

## Alt/Option stays outside world shortcuts

The native modifier handler now retains Alt/Option state (#381). After menu
routing, Alt-modified presses use the same world-input guard as Command/Super.
They cannot issue orders, arm modes, select units or categories, pan/zoom,
open save/load/pause, toggle mute or write save/preferences files. Menu text
input retains its supplied text, including Option-produced characters; ordinary
and documented Ctrl/Shift world bindings retain their existing routes.

Rejected presses keep held-key/repeat suppression, so releasing Alt alone does
not turn a continuing repeat into a plain shortcut. A release or fresh deliberate
press restores the plain binding. Focus loss, menu transitions, completed loads
and the native folder dialog clear the tracked Alt state alongside other
modifiers. Selection, existing gestures, queued commands, simulation and save
encoding are unchanged. This is native desktop input policy; original-game
modifier behavior and physical Mac/Option delivery remain unverified in #97.

## Held arrows before desktop modifiers

If a camera arrow is already held when Alt/Option or Command/Super first
becomes active in a match, the modifier handler clears active keyboard input
immediately (#393). This does not depend on another arrow event or on operating
system key repeat. Already-held keys remain suppressed after modifier release
until key-up or a fresh press. Ordinary Control/Shift panning and menu editing
retain their existing routes; selection, queued orders and saved match bytes
are unchanged. This is native desktop input policy, without original-game or
physical Mac/Option calibration.

`OPENYR_WORLD_FEEDBACK_PROOF=<directory> cargo test -p yr-app --bin openyr
late_desktop_modifier_compositor -- --nocapture` exports authored checkerboard
frames and camera coordinates through the shared camera/compose handlers.
The stopped and stale-repeat frames remain byte-identical, fresh arrows move
the view again, and complete Session bytes are preserved. This proof needs no
retail assets and does not establish physical input delivery or original parity.

## Alt-modified menu activation

Alt/Option+Enter and Alt/Option+keypad Enter now remain outside menu activation
(#384). They cannot accept a focused save overwrite, load, permanent delete,
setup choice or navigation control. This check runs in the shared native
keyboard handler before menu dispatch. Rejected Enter presses retain repeat
suppression; releasing Alt alone cannot turn a held repeat into activation.
A key release or fresh deliberate plain Enter restores the ordinary action.

Other menu keys and text retain their existing routes, including Option-produced
characters and spaces in Save names. Simulation, saved-match encoding, pointer
input and plain Enter behavior are unchanged. This is a native input policy;
original-game modifiers and physical Mac/Option delivery remain uncalibrated.
