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
