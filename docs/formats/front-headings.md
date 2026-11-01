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

A fresh capture after returning from the map chooser paints Skirmish's final
`e` blue 0 instead of blue 24. Its 25 ink pixels change while the prefix,
mask and baseline stay fixed. This newly observed interaction state is #153;
the default-state captures and their matches remain valid. The native helper
still uses the default foreground rule, and full modal behavior remains open.

An earlier 08:25:09 export has an empty heading during a transition. It is
retained as an observation and excluded from the stable heading assertions.
These static checks do not establish transition timing, warning cadence,
focus decoration, uncaptured page layouts or localized heading styling.
Broader Skirmish form and menu work remain in #96.

The original renderer PrintScreen exports succeeded in the earlier capture
runs. Previously permission-blocked `screencapture -x` and `screencapture -V`
paths were skipped without retrying. Native images use the shared application
compositor; they do not constitute physical desktop timing evidence.
