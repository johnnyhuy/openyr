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
234 differences. The full region has 468 remaining differences, all belonging
to those missing marker objects (#152). Its remaining-pixel RGBA FNV-1a is
`4ab7b3efc5ec5dc9`.

The Lake crop checks `(632,37,168,118)`. Its marker boxes are
`(668,49)-(686,67)` and `(698,126)-(716,144)`. The other 19,176 pixels have
RGBA FNV-1a `22dd8120185cff9d`. Returning from the map chooser also changed
25 final-heading-glyph pixels from blue 24 to blue 0. That newly observed state
is tracked in #153 and is excluded by starting this check below the heading.
It does not change the default-state heading evidence.

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
`screencapture -V` paths were skipped without retrying. Numbered markers,
modal heading colors, complete form/map-selection behavior and animation timing
remain in #152, #153 and #96. These checks do not establish full form parity.
