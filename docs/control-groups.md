# Unit teams

The original [Yuri's Revenge manual](https://gamepedia.cursecdn.com/cnc_gamepedia_en/8/8e/Red_Alert_2_-_Yuri%27s_Revenge_-_English.pdf),
printed page 3, documents Ctrl+1-9 to create teams, 1-9 to select them and
two presses to center the tactical map. OpenYR routes these keys through its
live match handler. Recall uses the existing selection audio, markers and
world-order resolver.

Ctrl+number replaces that team's membership with the current living owned
units. Number recalls the team's surviving owned units. A second deliberate
press within 400 milliseconds centers the world viewport on the bounding
box of their drawn ground positions. Terrain height, zoom and sidebar width
are included. Holding the key does not repeatedly select or center it.

Shift+number adds the team's units to the current selection. Empty recall
keeps the current surviving selection; Ctrl+number with no selected units
clears that team. A unit may belong to multiple saved team selections.
These policies, the 400-millisecond interval, bounding-box center and
three-second feedback overlay are native choices pending controlled
original-game observation. Only the three basic shortcut meanings above
are established by the manual.

The top-left feedback uses the installed GAME.FNT and framebuffer coordinates,
independent of world zoom. It fits inside the world viewport. Recall and
assignment cancel pending mouse gestures, placement, repair/sell and
attack-move mode so releasing a previous gesture cannot issue an order.
Changing selection, other command keys, menu entry or focus loss interrupts
double-press detection and dismisses feedback. Pause and focus loss preserve
the teams. Menu keys never assign or recall teams. Teams reset on successful
match replacement, restart, load or return to the front menu; a failed or
cancelled load preserves them with the retained match. Team selections are
UI state and do not change simulation hashes, RNG or match saves.

Synthetic tests run without retail data and cover all nine slots, replacement,
additive recall, ownership/death pruning, empty teams and double-press timing.
The retail application test drives the shared handler and compositor on EB1,
checks command eligibility and full save-byte invariance, guards OS repeat,
modal input and focus loss, and checks centering at zoom 0.25, 1 and 4.
`OPENYR_GROUP_PROOF=<external-dir>` exports those frames and their tick,
selection, feedback and state hash. No game art is committed. These captures
exercise the same handlers and compositor as the window; physical macOS
keyboard delivery and Retina exercise remain in #97.

## Visible type selection

The public [Red Alert 2 manual, Basic Keyboard Functions, printed page 1](https://static.wikia.nocookie.net/cnc_gamepedia_en/images/3/3b/CCUC-RA2-pcMANOLeUS.pdf/revision/latest)
maps `T` to Type select. OpenYR now handles a deliberate T during an ongoing
match (#316). It expands the living owned types represented in the current
selection to matching living owned units whose drawn ground anchors are inside
the world viewport. It uses the same terrain-height projection and half-open
right/bottom bounds as box selection, excluding the sidebar. Already-selected
living owned units remain selected even when offscreen; invalid, dead and
nonowned IDs are pruned. Mixed selections expand all represented kind IDs.

T cancels old pointer gestures, placement, repair/sell and attack-move modes,
interrupts team-recall timing and shows a short unit-count notice through the
existing two-second feedback channel. An empty eligible selection shows a
request to select a unit. Queued valid commands, assigned teams, camera, sim
hashes and save bytes are not changed. A changed nonempty selection uses the
existing selection-audio route. OS repeats, synthetic/unfocused events, tracked
Shift/Control/Command chords, menus and ended matches cannot perform this action.
Save/Skirmish editors still receive ordinary text T events through menu routing.

The manual also documents the double press: one press selects the units on
screen of the same type, and a double press selects them across the
battlefield. OpenYR follows that split. A second deliberate T within
400 milliseconds (the team double-press window) expands the represented
living owned types to the whole map when the types and the selection still
match the first press exactly. A selection change through clicks, boxes,
team recall or right-click cancellation interrupts the window, as do focus
loss, menu input and match replacement. A third press inside the window
stays global; a press after the window expands the viewport again and
re-primes the double press.

The manual establishes the shortcut mapping and the screen/battlefield
split, not these detailed policies. The 400-millisecond window, the exact
type-and-selection match, retained offscreen seeds, mixed-type behavior,
notice wording/timing and audio triggering are native choices pending
original-game comparison. Full type-selection calibration remains in #97.
No original executable was run.

Authored Session checks cover three layouts at zoom 1, 2 and 4, plus sidebar
and bottom-edge geometry at zoom 0.25, 1 and 4. Native input/compositor checks
verify expansion, ownership/type filtering, retained offscreen seeds, stale-ID
pruning, gesture cleanup, pending-order preservation and unchanged Session bytes.
Double-press checks add whole-map expansion at two layouts, a third press
staying global, and viewport-only results after a selection change, focus
loss or an expired 400-millisecond window.
Retail sprite/cursor pixels, physical Mac/Retina input, selection audio and
original-game timing remain unverified without the install and native session.

## Consecutive type presses

The native type-selection window now ends on a fresh non-T key press, an
accepted Left/Right press, or finite nonzero wheel input routed to the world
or sidebar (#321). A fresh rejected Shift/Control/Command+T also ends the
previous plain-T sequence. For example T, H, T starts a new viewport expansion
after base navigation rather than adding offscreen matches from the old view.
Valid wheel attempts count even at a zoom/scroll limit.

T release, OS repeat, synthesized/unfocused key guards, rejected pointer input,
zero/nonfinite wheels and pointer motion alone do not become intervening
commands. Consecutive deliberate T presses still use the existing 400 ms window
and exact type/selection match. Previously selected offscreen seeds retain the
existing first-press policy. This changes only transient UI timing state;
selection submits no extra orders and preserves camera and Session bytes.

These are native interruption rules, not a claim about original timing or
physical input delivery. Regressions reproduce the old three-unit global
result for T, H, T and compare the fixed two-unit viewport result. Eighteen
key sequences, twelve accepted pointer/wheel cases, three modifier chords and
seven positive guard cases use authored Sessions and shared composition.
Logical 50/60 ms test timestamps control the second press/capture; they do not
measure original-game runtime. Retail and physical Mac/Retina checks remain open.
