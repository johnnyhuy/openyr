# Original front-menu animation observations

The local retail version 1.001 renderer exported 13 menu images during a
bounded capture sequence on 2026-11-01. Their visible movie phases and warning
frames were matched to the user's retail data. No original implementation code
was inspected or copied. Decoded artwork and capture files stay outside git.
The external proof for #135 and #136 carries the raw images, hashes, scripts,
input receipts and native before/after compositions.

## Captured states

The sequence requested 80 PrintScreen presses over 18,463 milliseconds using
ordinary Win32 SendInput calls. Each press used a 20 ms down/up gap and a
180 ms pause. Every request checked the verified game process/window and
foreground state. A host watcher recorded completed PNG exports with monotonic
observation times and SHA256 hashes. It collected 13 images before stopping.
The reference movie has 431 source frames at a declared 15 fps. Warning source
indices below refer to the 91-frame SDWRNANM.SHP.

| Capture | Movie frame | Warning frame |
| --- | --- | --- |
| 0 | 311 | 89 |
| 1 | 319 | 5 |
| 2 | 326 | 11 |
| 3 | 337 | 21 |
| 4 | 345 | 27 |
| 5 | 351 | 33 |
| 6 | 357 | 37 |
| 7 | 365 | 42 |
| 8 | 365 | 42 |
| 9 | 366 | 44 |
| 10 | 371 | 48 |
| 11 | 371 | 48 |
| 12 | 371 | 48 |

Warning crops match their decoded source frames exactly at `(670,48)`, after
RGB565 truncation. The prior `(670,45)` anchor was three pixels too high.
The earlier #134 main capture also matches warning frame 22 at this anchor.
Eleven unique observed warning phases have regression fingerprints covering
`(660,40,112,68)`, including the surrounding display rim. Phase 89 is
also checked after one and two complete 91-frame wraps. These checks exercise
the shared native compositor.

Four earlier #134 captures add movie frames 16, 18, 206 and 224. Together,
17 captured images cover 14 unique movie phases.

## Movie color conversion

The visible movie rectangle is `(0,0,632,568)`. The status strip covers the
lower two rows of its 632x570 canvas. Matching movie phase before measuring
color is essential; a visually similar adjacent frame is a different input.
All fourteen unique captured phases have FNV-1a fingerprints of their original RGBA
rectangles in the existing retail decoding/composition test.

The prior summed 8-bit BT.601 calculation differed on 1,235,453 pixel samples
across these 17 images. The total includes repeated captures. A numerical
experiment with the independently verified YUV planes distinguished summed
rounding, per-term flooring and per-term truncation toward zero. Identical YUV
inputs gave identical original output colors in the checked reference, so a
spatial dither was not needed to explain those observations.

The compatible conversion uses separate signed integer contributions. With
`c=Y-16`, `d=U-128`, `e=V-128`, and `T(n)=n/65536` truncated toward zero:

```text
luma = max(0, T(76309*c))
r = luma + T(104597*e)
g = luma - T(25675*d) - T(53279*e)
b = luma + T(132202*d)
```

Each result clamps to 0..255 before the front compositor packs RGB565. The
implementation builds five bounded 256-entry tables from these coefficients;
it does not ship asset-derived lookup data. The luma term at `Y=89` and
`Y=162`, and the signed blue term at `U=70`, distinguish the rounding cases.
Neutral `Y=235` yields 254 before packing, which presents as the same
`(248,252,248)` display white. The existing black/white check covers that
actual packed target, and truncated-plane validation remains intact.

The captured phases supplied constraints and cross-checks for this compact
model. Earlier captures expose sub-black Y=14 samples whose luma term must
clamp to zero before adding chroma. The model matches every visible movie
pixel in all 17 captured images. This describes observed output,
not the implementation of the original converter. The independent full-clip
YUV and RGB comparisons remain separate checks in [bink.md](bink.md).

## Timing and capture limits

This sequence includes frozen repeated images, irregular exports and exports
that continued after the request sequence. It does not establish normal
wall-clock playback cadence. No warning frame-rate adjustment is justified
by these samples. OpenYR's provisional warning clock remains 15 fps. Original
warning cadence, button transitions, focus decoration and movie/shell
synchronization remain in #96. Pixels for uncaptured original movie phases
have not been checked against the original renderer.

The original renderer's hotkey exported these images. Previously blocked
macOS `screencapture -x` and `screencapture -V` paths were skipped without
retrying. Native proof images use the same compositor as the application.
A sequence of matched captured phases demonstrates pixel output at those
phases; it does not demonstrate live native desktop timing.
