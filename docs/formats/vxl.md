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

## Shared projection and pose measurements (#73)

`yr-render::voxels` now loads each VXL with its matching HVA, including
`TUR` and `BARL` attachments. Frame zero supplies the pose, matched by
case-insensitive limb name. Missing, malformed, unmatched or non-finite
HVA transforms fall back to the limb's VXL base transform. Invalid bounds,
zero dimensions and non-finite projected points are skipped.

Retail measurements from the installed MIX cascade on 2026-10-30:

| Part | Grid | Minimum bounds | Maximum bounds |
|---|---|---|---|
| MTNK body | 59x32x14 | -27.893934, -15.767818, 0.336604 | 30.337467, 15.979401, 13.892900 |
| MTNKTUR | 25x25x17 | -13.159321, -12.136850, 11.607287 | 11.506207, 12.298412, 28.395008 |
| MTNKBARL | 27x12x3 | 7.646459, -5.543928, 14.028087 | 33.883360, 5.728433, 16.870064 |

All three HVA poses are identity. Their boxes already place the turret
above the hull and the barrel forward of the turret. Recentering each grid
and adding guessed body-height offsets destroys those authored placements.

ROBO provides evidence for translation units. Its VXL base translation is
`(-0.064227805, 0.15023422, 0.24052116)`. Its first HVA translation is
`(-0.7707336, 1.8028107, 2.8862538)`, with a VXL scale of `0.083333336`.
Multiplying the HVA translation by that scale reproduces the base to less
than 0.00001 per axis. The renderer therefore treats HVA as a replacement
pose with scaled translation, rather than composing two copies of the
base pose. This is an inference from retail data, not a Wine parity claim.
SHAD's first rotor has HVA translation `(-421.56384, -0.7522948, 342.47903)`;
after scaling, its offset is `(-35.13032, -0.06269123, 28.53992)`.
Independent centering previously overlaid the rotor limbs on the fuselage.

The current sampling policy is
`local = min + (voxel_index + 0.5) * (max - min) / grid_size`, followed by
`rotation * local + translation`. The fixed orthographic camera uses a
45-degree azimuth and 30-degree elevation, one pixel per model unit, and
sorts all parts together by camera depth. It keeps the authored origin as
the ground anchor instead of normalizing every facing to 60 pixels wide.
Live canvases grow around that origin, capped at 512x512 before allocation;
two-pixel samples fill projection gaps. All six sampled retail models
(MTNK, HTNK, CMIN, ROBO, SHAD, ZEP) fit without clipped body pixels at eight
facings and match the map renderer at the same ground anchor.

Voxel-centre sampling, camera calibration and sample coverage remain
presentation approximations pending Wine crops. Retail normal tables/VPL lighting,
independent turret facing and voxel shadows remain
follow-ups. No parser layout changed in this slice.

## HVA playback (#78)

Live voxel sprites now select every authored HVA frame. Each body, turret
and barrel resolves its frame independently as
`floor(absolute_sim_tick / 2) % actual_frame_count`. Missing HVA data uses
the VXL base pose. Unmatched limbs, absent transforms and non-finite
transforms fall back per limb, preserving the other parts. Static map
objects keep frame zero. Movement and firing do not restart the voxel
clock; a loaded match resumes its phase from the saved absolute tick.

The two-tick cadence at 15 ticks/s is a presentation choice, not verified
retail timing. Wine comparison remains in #6. No interpolation, turret
aiming, airborne height or deployment-state selection is added here.

The installed `RA2.MIX/LOCAL.MIX/SHAD.HVA` is 360 bytes: two frames and
three limbs named `CYLINDER18`, `CYLINDER09` and `DUMMY01`. In frame one,
both rotor matrices have first-row rotation values approximately
`(0.809017, 0.587785, 0)` and second-row values
`(-0.587785, 0.809017, 0)`, a 36-degree rotation from frame zero. Their
translations stay within 0.00004 source units of frame zero. The fuselage
matrix is identical in both frames. This provides an observable motion
fixture without inventing new geometry or attachment offsets.

`retail_black_hawk_rotors_animate_around_a_fixed_fuselage` checks both
poses at eight facings against separately loaded one-frame models holding
the corresponding retail transforms. Ground-aligned canvases expose
anchor drift. `OPENYR_HVA_DUMP=<external-dir>` exports these pose canvases.
MTNK, CMIN and ROBO retain one-frame art at all tested ticks. Synthetic
tests cover unequal part lengths, large clocks and malformed later poses.

`SpriteBank::get_animated_at` takes the absolute tick shared by live and
headless drawing. Its keys hold resolved per-part frames rather than
ticks. Parsed voxel models are retained per type, and a FIFO caps decoded
unit sprites at 512 entries, including failed decodes. Coprime part
lengths cannot create an unbounded sprite cache or require a common loop
period. Repeated ticks, movement changes, wrapping and eviction are
checked by `retail_voxel_cache_uses_resolved_poses_and_stays_bounded`.

## Geometry-derived lighting (#82)

The shared map/live voxel renderer now shades palette and house colours
under one light fixed above and to the left in camera space. This is a
clean-room presentation approximation. It does not decode the retail
indexed normal tables, apply VPL lookups or claim Wine parity. No original
normal vectors or executable code were copied into the renderer.

When a model loads, each limb builds a sparse occupancy set from its
decoded voxels. For each voxel, the negative density gradient over the
26 neighbours supplies an outward surface normal. Neighbours at squared
grid distances 1, 2 and 3 contribute integer weights 6, 3 and 2. Symmetric
interiors therefore cancel exactly. Dividing each gradient component by
the authored cell spacing handles unequal grid dimensions. Cache storage
follows the actual decoded voxel count; it never allocates a dense volume
from a limb header. The cached normals do not change with animation ticks.

Each selected HVA/VXL pose applies its inverse transpose to the normals,
including scale, shear and reflection, without translation. The camera
then rotates them with the unit facing. The light direction in projected
camera coordinates is the normalized vector `(-0.45, -0.65, 0.61)`.
Brightness is `0.62 + 0.38 * max(dot(normal, light), 0)`, rounded to an
8-bit multiplier and applied after palette/house remapping. Alpha, camera
depth, sampling coverage and ground anchors remain unchanged. Isolated
voxels, symmetric thin surfaces, invalid geometry and singular poses use
unlit colours rather than an arbitrary normal. These constants are artistic
choices pending the Wine comparison, not discovered format fields.

Synthetic cube tests check all six outward faces, stretched corners,
translation independence, scale/shear/reflection, fixed-light facing and
pose changes, malformed values and palette/remap shading. The retail pose
test exports MTNK, HTNK, CMIN, ROBO, SHAD and ZEP at eight facings through
`OPENYR_VOXEL_DUMP=<external-dir>`. Before/after canvases from the installed
MIX cascade on 2026-10-31 have identical alpha masks at all 48 views;
77,247 pixels receive shading. The existing retail tests also check
shared map/live pixels and ground anchors, animated rotor poses and
bounded sprite caching. Proof crops stay outside git.

## Sources

- XCC Utilities `vxl_file`/`hva_file`, ModEnc "VXL" and "HVA".
- OpenRA `VxlReader.cs`/`HvaReader.cs` (GPL-3.0), cross-check only.
- Verified against retail data as above.
