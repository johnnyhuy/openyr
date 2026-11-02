# Front menu heading observations

The original renderer exports used by #134 and #139 provide sixteen Main Menu
headers and one Single Player header. Three additional stable Skirmish exports
at 08:26:30, 08:28:41 and 08:28:45 on 2026-11-01 provide a third heading.
The Skirmish proof retains header crops because the full form contains a player
name. The external bundle records full source hashes and crop hashes. No retail
image, font or game binary is committed.

## Glyphs and foreground

Raw `GAME.FNT` masks reproduce the observed heading geometry. Main Menu's final
`u` has 29 painted pixels; Single Player's final `r` has 16; Skirmish Game's
final `e` has 25. Every one of these pixels presents as RGB565 `(248,252,24)`.
Other heading glyphs present as `(248,252,0)`, including earlier occurrences of
`r` and `e`. Sixteen Main Menu observations have identical complete headers.

A final glyph foreground rule explains all three headings. A fixed screen
highlight does not explain the different positions, a glyph-specific rule
changes earlier letters, and a palette-wide change alters the preceding text.
The source header art is blank behind the labels. This is a compact description
of observed output, not the implementation of the original text renderer.

The native heading helper paints the prefix and final Unicode scalar at their
existing font advances. Fitting and centering still use the complete string.
Button labels and other text keep their existing drawing path. The existing
five complete enabled-button comparisons remain separate checks.

## Labels and placement

| Page | Retail CSF key | Observed text | Text origin | Checked rectangle | Original RGBA FNV-1a |
| --- | --- | --- | --- | --- | --- |
| Main | `GUI:MainMenu` | Main Menu | centered, y=9 | `(632,0,168,40)` | `e789aaaf10a7f8d1` |
| Single Player | `GUI:SinglePlayer` | Single Player | centered, y=9 | `(632,0,168,40)` | `29e7b0529ab9bf19` |
| Skirmish with preview | `GUI:SkirmishGame` | Skirmish Game | `(677,3)` | `(632,0,168,37)` | `af5382b7a6dc6151` |

The tall SDTP frame 0 and compact map frame 1 have different title wells.
The Skirmish text is 78 pixels wide. Searching translations of its raw glyph
mask finds a unique exact match at `(677,3)`. Native code previously selected
`GUI:Skirmish` and used y=9 for both frame shapes. The corrected header selects
the authored caption and follows the frame actually shown.

`retail_front_headings_match_original_renderer_captures` exercises actual
composition for the three pages, using the real authored EB1 preview for
Skirmish. It stores hashes, not artwork. `OPENYR_TITLE_PROOF` exports the
native frames outside git. The twenty specified capture regions contain
132,888 pixel samples, including repeated captures; all match after correction.

## Remaining preview and menu work

The original heading proof stopped above preview row 37. Extending it through
row 39 exposed eleven dark-blue SDTP pixels where the original control was
opaque black. Switching the top art
to SHELL2.PAL introduces 4,674 differences in the extended 168x40 comparison,
so it does not explain the control background. SHELL.PAL remains correct.
Public control measurements now establish the preview client at
`(644,37,145,113)` (#144). Painting its background resolves those eleven
pixels. The native heading regression now checks all 40 rows for its wide
preview fixture, with RGBA FNV-1a `1091ed8ace62e049`.
The [preview observations](front-preview.md) cover its raster and limits.

## Dialog refreshes

Fresh public UI observations on 2026-11-02 distinguish initial heading
presentation from a refreshed control (#153). The same Alamo setup and text
are used throughout the focus and repaint probes. Opening Skirmish paints the
final `e` with blue 24. Each of the following changes those same 25 ink pixels
to blue 0, without changing the prefix, glyph mask, position or frame:

- A verified focus round trip through a temporary helper-owned window.
- An isolated `RedrawWindow` request for the owned heading Static control.
- Returning from Choose Map, both Cancel and Use Map with unchanged selection.
- Accepting Little Big Lake, then restoring Alamo through the chooser.

Back followed by fresh Skirmish entry restores blue 24. A second focus probe
repeats the change. The final `e` in the chooser's `Customize Battle` heading also changes its 25 ink pixels
from blue 24 to blue 0 when a different map row is selected. Returning selection
to the original row retains blue 0. Merely opening the chooser retains blue 24.

| Header at `(632,0,168,37)` | Initial RGBA FNV-1a | Refreshed RGBA FNV-1a |
| --- | --- | --- |
| Skirmish Game | `af5382b7a6dc6151` | `86ad3335e6790da9` |
| Chooser (Customize Battle) | `03bbbbb008bd0f95` | `970ff46dc361ae9d` |

The probes use public
[RedrawWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-redrawwindow)
and [SetForegroundWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow)
APIs. They verify the complete target image, owner PID, current page and
control class. The first focus helper's immediate restore check returned false;
a later guarded PrintScreen verified owned foreground and captured the changed
heading. The corrected helper waits for observed foreground after destroying
its own temporary window and verifies both sides of a second round trip.
No original implementation, disassembly or process memory was inspected.

One transition snapshot temporarily found no unique dialog. Fresh reads of
the same process subsequently verified Skirmish. Choose Map's synchronous
button message timed out while the chooser remained open; a separate read
verified that actual page before continuation. No input was repeated because
of a timeout, and the original was not restarted during these probes.

The native model records heading refresh as presentation state. Opening a
page resets it, except returning from the chooser refreshes Skirmish. Actual
focus loss and a changed chooser row refresh the active supported heading.
Mouse and keyboard selection share the change signal. Initial asynchronous
preview loading and ordinary animation redraws do not change heading state.
This models the observed transitions, not the original renderer's internals.
Main Menu and Single Player retain their separately verified default styling;
their refresh behavior and other dialogs are not established by these probes.

`retail_skirmish_heading_tracks_dialog_refreshes` follows the actual app
focus, mouse, keyboard, chooser Cancel/Use Map and re-entry handlers. It checks
12 complete 168x37 headers, 74592 RGBA samples, without masks. The previous
renderer failed all seven refreshed states in the initial regression; initial
and re-entry states already matched. The proof includes that failing log,
passing output, source receipts, complete safe crops and pixel differences.

An earlier 08:25:09 export has an empty heading during a transition. It is
retained as an observation and excluded from the stable heading assertions.
These static checks do not establish transition timing, warning cadence,
focus decoration, uncaptured page layouts or localized heading styling.
Broader Skirmish form and menu work remain in #96.

The original renderer PrintScreen exports succeeded in the earlier capture
runs. Previously permission-blocked `screencapture -x` and `screencapture -V`
paths were skipped without retrying. Native images use the shared application
compositor; they do not constitute physical desktop timing evidence.
