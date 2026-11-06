# Window and framebuffer input

The native window presents a fixed 1280x800 framebuffer. Window resizing and
DPI scaling change its physical presentation through `pixels`; map zoom is a
separate framebuffer-space camera setting. Input arrives in physical window
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
open under #97/#98. Framebuffer dimensions and UI scaling policy are unchanged.

Compact 640x480 and 1000x700 clients expose a separate fixed-framebuffer
cropping problem, tracked in #224. Those pixels are not physical margins,
and the failed initial margin experiment is not used as proof for #223.
