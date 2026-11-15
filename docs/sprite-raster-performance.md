# Sprite raster clipping

The shared window/headless sprite blitter previously walked the complete scaled
canvas before rejecting offscreen pixels (#426). Its inner loop still visited
the entire scaled width for every visible row. Large valid sparse SHP canvases
and zoomed layers therefore cost source-area work even when little or none of
the sprite was visible. Cache byte limits do not bound this per-draw work.

The blitter now intersects each scaled axis with the destination dimensions
before iteration. Empty intersections return before either loop. Subtraction
uses i64 so saturated i32 origins and u32 viewport limits remain representable.
The loops retain offsets from the original rounded sprite origin, preserving
nearest-neighbor sample phase at clipped edges and fractional scales. Each
pass visits at most viewport-width times viewport-height candidate pixels.
Nonfinite anchors/scales and nonpositive scales leave pixels and shadow coverage
untouched. Sprite decoding, cache policy and painter order are unchanged.

A destination-driven oracle checks 4,200 authored anchor/offset/scale/pass cases,
including all edges, transparent destinations, partial body coverage, effect
alpha and preexisting unioned shadows. Every case compares full RGBA buffers and
coverage masks through two draws. Exhaustive small-axis checks and extreme
integer bounds verify iteration limits without allocating large buffers.

```sh
cargo test -p yr-app --bin openyr sprite_blit_tests -- --nocapture
```

An isolated optimized Rust probe extracts the actual blitter from baseline
`5ab328c` and the changed source, with identical caller, sprite and viewport
buffers. Each reported value is the median of five batches of 100 draws, on
this run's Mac. The scale is 4x and the viewport is 256x192; corner cases expose
64x64 pixels. All eight complete before/after RGBA buffers match.

| Native canvas | Position | Before, ns/draw | After, ns/draw |
| --- | --- | ---: | ---: |
| 64x64 | Origin visible | 73,216 | 68,527 |
| 64x64 | Bottom-right corner visible | 11,287 | 5,822 |
| 64x64 | Offscreen horizontally | 18,580 | 4 |
| 64x64 | Offscreen vertically | 116 | 4 |
| 1024x1024 | Origin visible | 333,723 | 67,803 |
| 1024x1024 | Bottom-right corner visible | 96,684 | 5,462 |
| 1024x1024 | Offscreen horizontally | 271,482 | 4 |
| 1024x1024 | Offscreen vertically | 1,682 | 4 |

These are blitter timings, not whole-match frame rates or a CI timing threshold.
The proof payload carries the extraction script, source and executable hashes,
compiler commands and raw output. Retail EB1 comparisons use separately built,
source-verified application copies, as described in `runtime-proof.md`.
Original-game animation cadence, overlay depth and physical Retina/input
calibration remain separate work under #6, #97 and #102.
