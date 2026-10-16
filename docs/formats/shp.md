# SHP sprites (TS/RA2 variant)

Palette-indexed sprite sheets: cameos, buildings, infantry, animations,
overlays, theater objects (trees, wakes). Index 0 is transparent. Parser:
`crates/yr-formats/src/shp.rs`.

## Layout

All integers little-endian.

```
file header (8 bytes)
  u16 zero          always 0 (distinguishes from TD/RA1 SHP)
  u16 width         canvas width shared by every frame
  u16 height        canvas height
  u16 frame_count

frame header (24 bytes) x frame_count
  u16 x, y          frame bounding box position inside the canvas
  u16 w, h          frame bounding box size
  u32 flags         low byte = compression (below)
  u8  radar[3], pad average colour for the radar/minimap
  u32 reserved
  u32 offset        absolute file offset of pixel data; 0 = empty frame
```

Compression (low byte of `flags`):

| Value | Encoding |
|---|---|
| 0, 1 | raw: `w * h` bytes |
| 2 | per row: `u16 len` (counts itself, always `w + 2`) then `w` raw bytes |
| 3 | per row: `u16 len` (counts itself) then zero-run RLE: `00 n` = `n` transparent pixels, any other byte = literal |

## Empirical notes (retail corpus)

`mix-dump scan` over all 16 retail archives decodes every frame of 5,450 SHP
files (83,626 non-empty frames) with zero errors:

| Compression | Frames |
|---|---|
| 0 | 2,452 |
| 1 | 5,296 |
| 2 | 252 |
| 3 | 75,626 |

- **Type 2 carries a length prefix on every row**, and every one equals
  `w + 2` (the decoder enforces this; 252 frames pass). OpenRA's loader reads a
  single prefix and then contiguous rows, which only works for one-row frames.
- **0x0 canvases are legal**: `ra2.mix/CONQUER.MIX/NULL.SHP` is 56 bytes, a
  0x0 canvas with two empty frames whose reserved fields hold `0xCC` (MSVC's
  uninitialised-stack fill), so the original tool wrote frame headers straight
  from uninitialised structs.
- Cameos are 60x48 single-frame raw SHPs (`language.mix/CAMEO.MIX/HTNKICON.SHP`).
  They are localised assets, so they live in `language.mix`, not `ra2.mix`.
- Odd frame dimensions occur; frames are not padded in storage.
- `expandmd01.mix/0x65ac42bd` (360 KB) is a single-frame 632x568 SHP.

## Sources

- XCC Utilities `shp_ts_file` (Olaf van der Spek) and ModEnc "SHP (TS)".
- OpenRA `ShpTSLoader.cs` (GPL-3.0), used as a cross-check only.
- Verified against retail data as above.
