# TMP isometric terrain templates (TS/RA2)

`mix-dump render` computes inspection bounds and pixel offsets in wide signed
coordinates (#397). A small template translated to `i32::MIN` or `i32::MAX`
keeps its pixels even when its rectangle endpoint exceeds i32. The composed
RGBA raster must have supported PNG dimensions and fit a checked 1 GiB budget;
allocation failure is reported before opening the destination. Widely separated
cells or extra rectangles return an inspection error instead of wrapping bounds
or attempting unbounded allocation. Empty templates retain a transparent 1x1
image. This inspection policy changes no parser, native renderer or save format.
Authored pixel/extra/output-preservation controls pass; current retail corpus
verification needs an accessible install.

Terrain tiles: a template is a small grid of isometric diamond cells (one
file per tile set entry, e.g. `CLEAR01.TEM`, `PROAD15.URB`). The extension
selects the theater: `.TEM` temperate, `.SNO` snow, `.URB` urban, `.DES`
desert, `.LUN` lunar, `.UBN` new urban. Parser: `crates/yr-formats/src/tmp.rs`.

## Layout

```
header (16 bytes)
  u32 cells_x, cells_y      template size in cells
  u32 tile_w, tile_h        diamond size: 60x30 in RA2 (48x24 in TS)
u32 cell_offset[cells_x * cells_y]   absolute; 0 = empty cell

cell (at its offset)
  i32 x, y                  pixel position relative to template origin
  u32 extra_ofs             cell-relative: extra overlay pixels
  u32 depth_ofs             cell-relative: depth diamond
  u32 extra_depth_ofs       cell-relative: extra overlay depth
  i32 extra_x, extra_y      extra overlay rectangle, template-relative
  u32 extra_w, extra_h
  u32 flags                 1 = has extra, 2 = has depth, 4 = has damaged variant
  i8  height                elevation level
  u8  terrain_type          land class (clear, water, road, rock, ...)
  u8  ramp_type             slope shape, 0 = flat
  u8  radar_left[3], radar_right[3], pad[3]
  u8  pixels[tile_w * tile_h / 2]   colour diamond (immediately after header)
  ...depth diamond, extra pixels, extra depth at their offsets
```

The diamond is packed scanline by scanline: rows widen by 4 px per row from
4 px at the top to the full width at row `tile_h/2 - 1`, then narrow by 4. For
60x30 that is 900 bytes, and the last row has width 0. `TmpFile::diamond_rows`
returns the `(row, x_start, width)` table.

## Empirical notes (retail corpus)

- 5,536 TMP files, all with 60x30 diamonds, parse without error.
- `ra2.mix/ISOTEMP.MIX/CLEAR01.TEM`: 1x1 cell, flat, depth present, no extra.
- `ra2.mix/ISOURB.MIX/PROAD15.URB`: 3x4 template, all 12 cells present.
- Tile-set filenames come from the theater INIs (`TEMPERATMD.INI` etc.):
  `[TileSetNNNN] FileName=clear TilesInSet=N` gives `clear01..clearNN`, with
  `a`..`g` suffixes for alternates. This recovers about 91% of TMP names.

## Sources

- XCC Utilities `tmp_ts_file`, ModEnc "TMP (TS)".
- OpenRA `TmpTSLoader.cs` (GPL-3.0), cross-check only.
- Verified against retail data as above.

## Terrain-loader recovery

Simulation terrain setup returns a filename-specific error when a referenced,
indexed template is missing or fails bounded TMP parsing (#293). It no longer
panics inside the template cache. Missing files report `terrain template
<name> is missing`; malformed files retain the parser's reason after the
filename. Successfully parsed templates are cached and reused as before.

This does not invent ground for unknown tile indexes, out-of-range subtiles or
authored empty TMP cells. Those records keep their existing sparse-grid policy.
Valid records retain TMP terrain class/ramp metadata and the map record's
height; clear-sentinel records still resolve through tile zero. No simulation
or save format changes.

Asset-independent in-memory tests cover missing, malformed and truncated TMPs,
repeated valid and clear-sentinel references, unresolved/empty subtiles, and
complete synthetic Sim save-byte/hash equivalence and roundtrip. Terminal proof
exercises the changed public `build_terrain` API directly. The separately
receipt-verified CLI smoke covers `mix-dump`, whose map-art path is distinct;
it does not establish complete Session reconstruction. Retail scans, physical
Mac/Retina, audio and original-game behavior remain unverified without data.

## Diamond geometry validation

The four-pixel row progression requires a width divisible by four and a height
of half that width (#386). Parsing accepts widths 4 through 256 with that
geometry. It rejects 2x1, 6x3 and other incompatible headers before reading cell
payloads. The former 2x1 acceptance underflowed the first row's horizontal origin
and made the release render CLI panic on a tiny authored template.

`diamond_rows` uses the same validation before allocation. Manually constructed
or modified public `TmpFile` dimensions outside this range return an empty row
table. Valid tables retain their centered first row, full-width midpoint,
zero-width final row and exactly `width * height / 2` packed pixels. TS 48x24,
RA2 60x30 and the existing 8x4 authored fixtures retain their layout. Invalid
headers now fail cleanly; no simulation or save fields change.
