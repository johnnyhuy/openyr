# VXL voxel models and HVA animations

Vehicles, aircraft and turrets are voxel models (`.vxl`) animated by matching
`.hva` files (`HTNK.VXL` + `HTNK.HVA`; turrets `HTNKTUR.VXL`, barrels
`HTNKBARL.VXL`). Parsers: `crates/yr-formats/src/vxl.rs`, `hva.rs`.

## VXL layout

```
header (802 bytes)
  char[16] "Voxel Animation\0"
  u32 palette_count     1
  u32 limb_count
  u32 tailer_count      == limb_count
  u32 body_size
  u8  remap_start       player-colour range, 16 in retail
  u8  remap_end         31 in retail
  u8  palette[768]      6-bit RGB (unused at runtime; theater palette wins)

limb header (28 bytes) x limb_count
  char[16] name         NUL-padded, e.g. "DUMMY01"
  u32 number, u32 reserved, u32 reserved

body (body_size bytes)  span tables and span data for every limb

limb tailer (92 bytes) x limb_count
  u32 span_start_ofs    body-relative: i32[x*y] column start offsets
  u32 span_end_ofs      body-relative: i32[x*y] column end offsets
  u32 span_data_ofs     body-relative: start of column data
  f32 scale             1/12 in retail
  f32 transform[12]     row-major 3x4 (rotation | translation)
  f32 min_bounds[3], max_bounds[3]
  u8  size_x, size_y, size_z
  u8  normals_mode      2 = TS normal table (36), 4 = RA2 table (244)
```

Columns are indexed `x + y * size_x`. A start offset of `-1` means an empty
column. Otherwise, relative to `span_data_ofs`, the column is a sequence of
segments repeated until `z >= size_z`:

```
u8 skip                 empty voxels before this run
u8 count
(u8 colour, u8 normal) x count
u8 count                repeated, as a check
```

## HVA layout

```
char[16] name           free-form; retail holds Westwood's source path
u32 frame_count
u32 limb_count
char[16] limb_name x limb_count     matches VXL limb names by position
f32[12] transform x frame_count x limb_count   frame-major, row-major 3x4
```

The file size is exactly `24 + 16*limbs + 48*limbs*frames`. With no magic,
that identity is what `mix-dump scan` uses to recognise HVAs.

## Empirical notes (retail corpus)

- 221 VXL files, 220 HVA files. All parse, and every column's walk ends exactly on
  the byte its **end offset names: the end offset is the column's last byte**
  (the trailing duplicate count). The decoder enforces this.
- Limb counts: 213 single-limb, plus 2/3/4-limb models and one 13-limb model.
- Normals mode 2 (TS table) appears on 133 limbs, mode 4 on 113, so RA2 still
  ships TS-era voxels. The renderer needs both normal tables.
- `HTNK.VXL`: one limb `DUMMY01`, 46x34x12 grid, 4,816 voxels, scale 1/12,
  remap 16..31. `HTNK.HVA`: one frame, identity transform.
- HVA name fields leak Westwood's asset tree: `HTNK.HVA` holds
  `N:\RA2\ASSETS\C` (truncated at 16 bytes).

## Sources

- XCC Utilities `vxl_file`/`hva_file`, ModEnc "VXL" and "HVA".
- OpenRA `VxlReader.cs`/`HvaReader.cs` (GPL-3.0), cross-check only.
- Verified against retail data as above.
