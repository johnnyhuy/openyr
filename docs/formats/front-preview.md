# Skirmish preview observations

On 2026-11-01 the installed RA2MD.exe launcher started the original game and
reached Main Menu. Direct gamemd.exe launches had exited zero without a usable
page. Public process metadata identified gamemd.exe as the launcher's child.
No original implementation code or process memory was inspected. The
launcher support and its synthetic bootstrap check are in `tools/wine-oracle`.
A later stopped-state launch still failed to expose any windows, with a Wine
window-driver diagnostic. Repeatable startup remains open in #148. This does
not invalidate the successful session's recorded UI and pixel observations.

## Control and image bounds

The original root reported a 1024x769 client and an owned 800x600 page. Coordinates
below are relative to that page, independent of the host window's size.
`GetWindowRect`, `GetClientRect`, `ClientToScreen`, control ID and style queries
identify Static control 1128 at `(644,37)-(789,150)`, right/bottom exclusive.
Its client is 145x113, and style `0x50000004` includes
[SS_BLACKRECT](https://learn.microsoft.com/en-us/windows/win32/controls/static-control-styles).
Rendered letterboxes are opaque black. The map chooser uses a different
146x114 control at `(643,37)`; it is not the Skirmish control.

The native preview previously used `(644,40,144,140)` without painting the
measured control background. The corrected black rectangle covers the entire
Skirmish client. It explains the eleven dark-blue SDTP pixels on row 37 that
survived the earlier heading correction (#144). SHELL.PAL stays unchanged.

| Map | Asset | Authored size | Observed image x,y,w,h |
| --- | --- | --- | --- |
| The Alamo | XMP29U2.MAP | 110x50 | 644,61,144,65 |
| Little Big Lake | XMP08T2.MAP | 70x67 | 657,37,118,112 |

Fit against the full client, floor the other dimension, and omit the last pixel
of the limiting dimension. Center the resulting raster in the full client,
rounding offsets down. Sampling then uses the final raster width and height
independently. For example, Alamo source y is `floor(y * 50 / 65)`.
A single floating scale for both axes differs on 1,947 of its 9,360 image pixels;
the independent axes differ only on the two numbered start markers (#151).
The portrait observation distinguishes a 118-pixel width from the initially
plausible 117-pixel result obtained by fitting against 144x112 first.
This describes observed output, not the original implementation.

The native helper validates dimensions and RGBA length before indexing and
leaves the measured background black when data is invalid. It handles both
limiting axes with integer arithmetic. Movie and larger prototype preview
rendering retain their existing paths. Menu transforms and input boxes are
unchanged; the small preview is not an input control.

## Pixel evidence and limits

Four independent Alamo crops agree over `(632,0,168,155)`. Outside the two
18x18 start-marker boxes, 25,392 pixels match native composition exactly.
The boxes are `(765,65)-(783,83)` and `(647,107)-(665,125)`. Each contains
234 differences. Before marker composition, the full region had 468 differences, all belonging
to those missing marker objects (#152). Its remaining-pixel RGBA FNV-1a is
`4ab7b3efc5ec5dc9`.

The Lake crop checks `(632,37,168,118)`. Its marker boxes are
`(668,49)-(686,67)` and `(698,126)-(716,144)`. The other 19,176 pixels have
RGBA FNV-1a `22dd8120185cff9d`. Returning from the map chooser also changed
25 final-heading-glyph pixels from blue 24 to blue 0. That newly observed state
is covered separately by the [heading lifecycle checks](front-headings.md)
(#153). This earlier preview check starts below the heading and retains its
default-state evidence.

`retail_skirmish_preview_matches_original_unoccluded_pixels` uses the actual
frontend compositor, retail art and each map's authored preview. It stores
hashes rather than artwork. `OPENYR_PREVIEW_PROOF` exports full native frames
outside git. Synthetic composition checks cover both limiting axes, control
edges, preview availability and malformed image dimensions/lengths.

The fresh original Alamo PNG has SHA256
`505b542cdb8163b4cdfd15c90fb68a8c3743a904444c5e34277eb52df39a8228`.
The Lake PNG has SHA256
`c62eb3e22e603056fdc496a7b933173f53e01c34d492911e9409075d24803e7b`.
Only safe right-panel crops are published. Full Skirmish forms contain a player
name and remain private. Original/native PNGs, helper source, UI metadata,
comparison scripts and source/crop hashes belong in the external proof bundle.

The original renderer's PrintScreen export supplied these static frames.
The public GDI BitBlt experiment returned false and produced no frame; it is
not timing evidence. Previously permission-blocked `screencapture -x` and
`screencapture -V` paths were skipped without retrying. Refreshed heading colors
are now calibrated separately (#153). Complete form behavior and animation
timing remain in #96.
These checks do not establish full form parity.

## Numbered marker artwork

The 18x18, one-frame SHP at `RA2MD.MIX/LOCALMD.MIX`, entry `3b7f5065`,
has 234 opaque pixels. Its filename remains unresolved. The source SHP SHA256
is `741614970e0b5459b932fb28f30ab2270878b679cf3cf07dfce896ac21736620`.
With SHELL.PAL and RGB565 presentation, its background agrees with the
observed Alamo and Lake marker backgrounds. SHELL2.PAL, SDBTNANM.PAL and
UNITTEM.PAL disagree on all 234 opaque pixels.

GAME.FNT supplies the numbers. Digits 1 and 2 have widths 4 and 5, and
22 and 26 ink pixels respectively. Placing each glyph at `(7,0)` within the
sprite, with packed RGBA `(248,236,96,255)`, exactly reproduces the observed
opaque marker pixels on both maps. Transparent-corner RGBA hashes are
`49aba43fcef760a5` for 1 and `bc89e15fafcd7f89` for 2. The native art check
also verifies eight decoded marker images with the same 234-pixel coverage.
The complete preview checks below also establish digits 3 through 8 in the
observed unselected Skirmish state. Other marker states remain uncalibrated.

An additional original chooser crop places Alamo's markers at `(648,107)`
and `(766,65)`, compared with `(647,107)` and `(765,65)` on the Skirmish page.
The chooser uses a different preview control. Placement must account for its
actual layout; it is not established by matching the shared sprite alone.
The preview loader retains the first eight actual `[Waypoints]` cell values,
plus map and local rectangles. Script waypoints such as 98 and 699 are not
player starts. Sparse entries retain their original number. The presentation
metadata is not added to simulation state or save data.


## Marker projection and composition

For authored thumbnails, project each raw start cell `(cx,cy)` using the map
width `mw` and local rectangle `(lx,ly,lw,lh)`:

```text
hw = lw - 1
hh = lh + (mw % 2)
rx = floor((cx - cy) / 2) + floor(mw / 2) - lx
ry = ceil((cx + cy) / 2) - floor(mw / 2) - ly - 1
marker_x = image_x + floor(rx * raster_width / hw - 1/8) - 9
scale_y = float64(raster_height) / hh
point_y = float32(ceil((cx + cy) / 2) * scale_y)
origin_y = float32((floor(mw / 2) + ly + 1) * scale_y)
marker_y = image_y + floor(point_y - origin_y) - 6
```

The fractional horizontal bias describes observed output. It does not claim
knowledge of the original implementation. The integer rule was recorded before
City, Bering and Heck captures. City and Bering matched, but Heck marker 7
was one pixel higher than that prediction.
Rounding each scaled absolute Y coordinate to single precision before
subtraction matches all six captured maps. A subsequent Death Trap prediction
matched all eight markers before that map was added to the regression. This
presentation-only float arithmetic stays outside simulation and save state.
A separate retail-data scan of 161 map sections found these projected start
coordinates and rectangle geometry equal to the stored `[Header]` values,
with no additional start entries beyond `NumberStartingPoints`.

Compose the unscaled 18x18 markers after the map raster. Clip to the full
145x113 control, including its black letterbox. Bering's lower markers extend
below its map image and remain visible. Invalid rectangles, malformed image
buffers and missing marker artwork leave the available preview intact. All
projection arithmetic does bounded work over at most eight markers.
Synthetic checks cover partial top/left and bottom/right markers, sparse
start 8, outlying cells, extreme rectangle values and malformed art.

Generated terrain previews retain the raw starts but do not draw these markers.
Their raster bounds differ from authored thumbnails; calibrating that projection
and the separate chooser layout remains part of #96. This slice changes only
the Skirmish presentation of authored thumbnails.

| Map | Asset | Starts | Raster x,y,w,h | Original region RGBA FNV-1a |
| --- | --- | --- | --- | --- |
| Alamo | XMP29U2.MAP | 2 | 644,61,144,65 | 2913314c62930011 |
| Little Big Lake | XMP08T2.MAP | 2 | 657,37,118,112 | 060d22eec9926af9 |
| Killer Instinct | XKiller.MAP | 3 | 644,55,144,76 | 562cd380ba327ba9 |
| City Under Siege | XEB1.MAP | 4 | 644,59,144,68 | 1aded70f160d841d |
| Bering Strait | XMP26S6.MAP | 6 | 644,67,144,52 | dd2be7a2b9f0dcbd |
| Heck Freezes Over | XMP32S8.MAP | 8 | 644,54,144,78 | 753cab68b2aac751 |
| Death Trap | XDeath.MAP | 8 | 644,57,144,73 | c93c6001335149b5 |

The Alamo hash covers `(632,0,168,155)`. The other hashes cover
`(632,37,168,118)`. Heading lifecycle comparisons are separate (#153).
The complete actual frontend composition, including every marker pixel and
transparent corner, matches each original region. Native frames are exported
at phase 0 with `OPENYR_MARKER_PROOF`; only safe crops are published.
