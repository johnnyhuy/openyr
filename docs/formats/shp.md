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
- Building SHPs store body frames first, then the same number of shadow
  frames (`GACNST.SHP`: 3 body states, normal and two damage levels, then 3
  shadows). Shadow pixels are coverage masks: draw them as translucent black,
  not via the palette. Visual check: `mix-dump render` (the cameo, the snow
  Construction Yard and the urban road templates render correctly).
- Infantry SHPs start with the 8-frame standing ("Ready") sequence, one
  frame per screen octant, **counter-clockwise from screen up**: frame 0
  faces up (back to the camera), 1 up-left, 2 left, 3 down-left, 4 down
  (facing the camera), 5 down-right, 6 right, 7 up-right. The shadow
  frames follow the body frames as for buildings (`frames / 2` offset).
  Observed on `RA2.MIX/CONQUER.MIX/GI.SHP` (`mix-dump render` with
  `UNITTEM.PAL`); used by `yr-render::sprites`.

## Infantry action sequences

`artmd.ini` was extracted from `ra2md.mix/LOCALMD.MIX/ARTMD.INI`.
The rules `E1` image resolves to `GI`, whose art section names
`GISequence`. `GGI` names `GuardianGISequence`. The sequence values are
`start,count,facing_stride[,fixed_facing]`:

| Sequence | GI | Guardian GI |
|---|---|---|
| Ready | `0,1,1` | `0,1,1` |
| Walk | `8,6,6` | `8,6,6` |
| FireUp | `164,6,6` | `204,6,6` |
| Idle1 | `56,15,0,S` | `56,15,0,S` |
| Idle2 | `71,15,0,E` | `71,15,0,E` |

The body frame is `start + facing * facing_stride + phase`, with facing
counter-clockwise from screen up as above. A zero stride selects the
same frames for every facing; the idle art already depicts the fixed
direction. The shadow is at `body_frame + total_frames / 2`.
`infantry_animation_matches_retail_sequence_canvases` in
`crates/yr-render/tests/retail.rs` compares every body/shadow coverage
pixel and every non-remap body colour against the independent SHP
canvases for both types, all facings, and every walk/fire/idle phase.
`OPENYR_ANIMATION_DUMP=<external-dir>` exports those sprites as PNGs.

The live window and `--frames` share the following prototype policy:

- Two simulation ticks per animation frame at 15 ticks/s. This cadence
  is a presentation choice, not an oracle-verified timing rule.
- Actual position changes select looping Walk. A blocked or stopped
  unit returns to Ready even if it still has a movement target.
- A newly observed shot starts FireUp. It plays once, returns to Ready,
  and restarts on the next shot. Movement takes priority.
- A stationary unit occasionally plays Idle1 or Idle2, alternating in
  90-tick intervals, then returns to its current Ready facing.
- Clocks live in `yr-app::animation`, outside the simulation hash.
  Repeated redraws without another simulation tick do not advance them.
- Checked arithmetic validates the complete sequence, across all eight
  facings, against the actual SHP body-frame count. Invalid or absent
  action data falls back to Ready, then a safe standing frame. The
  decoded cache is keyed by the resulting frame, not an unbounded tick.

Prone, deployed, death and paradrop sequences are not wired to sim
states yet. Building animation overlays/bibs, adjacent-building sorting
and Wine side-by-side validation remain under #6.

## Sources

- XCC Utilities `shp_ts_file` (Olaf van der Spek) and ModEnc "SHP (TS)".
- OpenRA `ShpTSLoader.cs` (GPL-3.0), used as a cross-check only.
- Verified against retail data as above.
