# TMP isometric terrain templates (TS/RA2)

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
