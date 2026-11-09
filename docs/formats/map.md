# Maps (`.map` / `.mpr` / `.yrm` / `.mmx`)

A map is an INI text file whose terrain is packed binary carried as base64
in numbered INI lines. Parsed by `yr_formats::MapFile`
(`crates/yr-formats/src/map.rs`); compression in
`crates/yr-formats/src/compress.rs`; tile artwork resolution in
`crates/yr-formats/src/tileset.rs`.

## Evidence

All figures below were measured on the retail install
(`~/Games/RA2YR/.../Westwood/RA2`) on 2026-10-17:

- 234 maps across `multimd.mix`, `mapsmd03.mix`, `expandmd01.mix`,
  `maps01.mix`, `maps02.mix` parse with zero errors
  (test `all_retail_maps_parse_and_align`).
- Coordinate conventions cross-checked against the parsed cells: all
  92,491 `[Terrain]` entries, all 143,068 non-empty overlay cells and all
  13,969 placed structures in the original three-archive probe land on an
  iso tile under the `(x, y)` reading below; the swapped reading does not
  (probe since replaced by the retail test above).
- Temperate tile-set totals match map usage exactly (below).

## Pack sections (general)

`[IsoMapPack5]`, `[OverlayPack]` and `[OverlayDataPack]` share a container:
their INI lines are numbered (`1=`, `2=`, ...), concatenated in numeric
order and base64-decoded (standard alphabet, whitespace ignored). The
decoded bytes are a chunk stream of `u16le in_size, u16le out_size,
in_size bytes`; each chunk decompresses independently to exactly
`out_size` bytes and the results are concatenated. `[IsoMapPack5]` chunks
are LZO1X ("format 5"); overlay chunks are Westwood LCW ("format 80").
Both decoders require exact input consumption and exact output size.

## `[IsoMapPack5]`: terrain cells

Decompressed layout: a sequence of 11-byte entries followed by a 4-byte
zero trailer. Every retail map satisfies `(len - 4) % 11 == 0` with a zero
trailer; the parser rejects any other shape. Entry layout:

| Offset | Type | Meaning |
| ------ | ---- | ------- |
| 0 | u16le | cell x |
| 2 | u16le | cell y |
| 4 | i32le | global tile number; -1 (and 65535) = clear cell |
| 8 | u8 | subtile: cell index inside the tile's TMP template |
| 9 | u8 | height level (0–12 in retail) |
| 10 | u8 | reserved; 0 in all 3,923,804 retail entries |

Zero-coordinate entries are padding and are dropped. Retail maps contain
no duplicate `(x, y)` cells, which the parser uses as a size cap (at most
one entry per 512×512 grid cell).

## Overlay packs

`[OverlayPack]` and `[OverlayDataPack]` each decompress to exactly
512×512 = 262,144 bytes (all 468 retail packs do; the parser rejects other
sizes). Cell `(x, y)` is at `x + y * 512`. `0xFF` in `[OverlayPack]` means
no overlay; the matching `[OverlayDataPack]` byte is the overlay frame.

## Other sections

- `[Map]`: `Theater=` (e.g. `TEMPERATE`), `Size=` and `LocalSize=` as
  `x,y,width,height`.
- `[Terrain]`: key is `y * 1000 + x` (so `key % 1000 = x`), value is the
  terrain object type (trees etc.).
- `[Structures]`, `[Units]`, `[Infantry]`, `[Aircraft]`: numbered lines
  `owner,type,health,x,y,...`; the facing field (0–255) sits at index 5
  for structures/units/aircraft and 7 for infantry.

## From tile number to artwork

The theater INI lists `[TileSet0000]`, `[TileSet0001]`, ... (scan stops at
the first missing number). A set's files are `<FileName>01`,
`<FileName>02`, ..., `<FileName>NN` (`TilesInSet = NN`) with the theater's
tile extension. Global tile numbers are assigned cumulatively in set
order; sets with `TilesInSet = 0` occupy no numbers. The map cell's
subtile byte indexes `TmpFile::tiles` of that template.

Per-theater files (resolved through the `yr-assets` VFS; YR `*md.ini`
wins over the RA2 original where both exist):

| Theater | Tile-set INI | Palette | Ext |
| ------- | ------------ | ------- | --- |
| TEMPERATE | `temperatmd.ini` / `temperat.ini` | `isotem.pal` | `.TEM` |
| SNOW | `snowmd.ini` / `snow.ini` | `isosno.pal` | `.SNO` |
| URBAN | `urbanmd.ini` / `urban.ini` | `isourb.pal` | `.URB` |
| DESERT | `desertmd.ini` | `isodes.pal` | `.DES` |
| LUNAR | `lunarmd.ini` | `isolun.pal` | `.LUN` |
| NEWURBAN | `urbannmd.ini` | `isoubn.pal` | `.UBN` |

Check: the temperate sets total 838 files (tiles 0–837), and 837 is
exactly the largest tile used by the 96 retail temperate maps. The retail
test `tileset_index_resolves_a_real_map` resolves every tile of
`EB1.MMX` to a readable TMP whose template contains the referenced
subtile.

## Rendering geometry (verified)

`mix-dump render-map <gamedir> <map> <out.png> [scale]` draws terrain:
cell `(x, y)` at level `h` has its 60×30 diamond centred at
`((x - y) * 30, (x + y) * 15 - h * 15)`, painted in increasing `x + y`
order; a template's extra overlay is placed relative to the tile's
diamond inside the template. Rendered retail maps show continuous roads,
cliff faces aligning with their tops, and shorelines closing, confirming
the 15 px-per-level step and the extra placement.

Objects are drawn in the same painter order, SHP canvas centred on the
cell centre:

- Overlay cells: the overlay type (index into `rulesmd.ini`
  `[OverlayTypes]`) names an SHP under the theater extension
  (`TIB01.TEM`, `GEM01.TEM`, ...); `[OverlayDataPack]`'s byte is the
  frame (ore/gem growth stage, via `TIB01` has 24 frames of increasing
  density). Overlays whose rules section has `Tiberium=yes` (ore
  `TIB*`, gems `GEM*`) index the theater *object* palette
  (`TEMPERAT.PAL`, `SNOW.PAL`, `URBAN.PAL`, `DESERT.PAL`, `LUNAR.PAL`,
  `URBANN.PAL`): rendered with `ISO*.PAL` they decode as rainbow noise,
  with the object palette `TIB01.URB` is gold ore and `GEM01.URB`
  multicoloured gems (eyeballed, EB1.MMX, 2026-10-19). Other overlays
  (walls, fences, props) are still drawn with `ISO*.PAL`, unverified.
- `[Terrain]` objects (`TREE01` etc.): same theater-extension SHPs,
  frame 0, theater palette.
- Placed infantry/aircraft: `<Image>.SHP` (`Image=` from `rulesmd.ini`,
  defaulting to the type ID) with the unit palette, frame from facing
  (`facing / 32`). Infantry art sequences need `artmd.ini` sequence data
  for correct poses; frame selection here is approximate.
- Structures are drawn as real SHPs: frame 0 plus its shadow frame
  (the second half of the SHP), centred on the art `Foundation=`
  footprint, with palette indices 16–31 (the player-colour ramp)
  remapped to the owner house colour, the owner's `Color=` name
  resolved through the `[Colors]` HSV table in `rulesmd.ini`.
- Voxel vehicles are drawn from their VXL body + `TUR`/`BARL` parts:
  orthographic iso projection rotated by facing, owner-colour remap,
  flat shading. Authored bounds and the first HVA pose now place each
  limb with fixed scale and a ground anchor (#73). Animated HVA poses and
  normal-table lighting remain open; voxel aircraft use the same path.

Building art resolution (found 2026-10-17 by matching SHP canvas
sizes and filename hashes across the retail archives): for
`NewTheater=yes` art, the filename's second character is the theater
letter, via `T` temperate, `A` snow, `U` urban, `D` desert, `L` lunar,
`N` new urban (e.g. civilian `CALAB` -> `CTLAB.SHP` in `ISOTEMP.MIX`,
`CULAB.SHP` in `ISOURB.MIX`). Player buildings often have no theater
variant; the generic `G` variant in `GENERIC.MIX` is the fallback
(`GACNST` -> `GGCNST.SHP`; snow uses the plain `A` name in `SNOW.MIX`).
The renderer tries theater letter, then `G`, then the plain name.

## Terrain classes for the sim

`yr_sim::terrain` maps the TMP `terrain_type` byte to ground classes.
Dominant value per tile-set filename prefix on the retail corpus
(temperate/urban/snow, measured 2026-10-17): 9 = water, 10 = beach,
11 = paved, 12 = road, 13 = clear, 14 = rough, 15 = cliff. Note the
iso tile coordinates run past the map's `Size=` rectangle, so a sim
grid must span the tile extents, not the rectangle.

## Hostile-input posture

Chunk headers, counts and sizes are never trusted: decoders bound output
by the declared per-chunk size, require exact size matches, and the map
parser rejects packs whose decompressed shape is not exactly the retail
shape (entry stream + zero trailer; exactly 262,144 overlay bytes).
Unit tests cover truncated chunks, lying chunk sizes, a corrupted iso
trailer, and a short overlay pack.

## Authored preview

`MapPreview::parse` reads the optional `[Preview] Size=x,y,width,height` and
numbered `[PreviewPack]`. The latter uses the same chunk container with LZO1X;
the expanded bytes are row-major RGB, exactly width*height*3. The retail EB1
thumbnail is 118x61 and expands to 21,594 bytes. The native front menu uses this
source image rather than substituting a newly rendered overview.

Dimensions are limited to 1024x1024, encoded input to 8 MiB, and every chunk's
input range and total declared output are checked before decompression. Duplicate
line numbers, invalid dimensions, truncated chunks and output-size mismatches
are rejected. No preview is different from a corrupt preview. The 234-map corpus
check now validates every present authored thumbnail as well as terrain data.

For EB1, 356 ore cells were projected into the authored thumbnail during the
channel-order probe. Of those, 251 samples carry gold-like RGB values and none
carry the byte-reversed gold pattern. Cells (44,31) and (45,31) both map to
(160,144,32). This supports retaining RGB for this retail preview rather than
assuming a byte order from a different game's preview description.

## Sparse simulation support

`TerrainGrid::from_tiles` distinguishes an absent coordinate from an authored
clear tile (#105, #125). The grid rectangle remains a storage/pathfinding bound;
only records resolved to valid TMP subtiles create usable cells. Ground and
flying movement, building footprints, cursor orders and session free-cell
searches reject the remaining holes. `TerrainGrid::new` still creates a fully
populated clear rectangle for synthetic scenarios. Resource/overlay edits
cannot turn absent slots into playable ground.

A map record with tile index -1 or 65535 is present clear ground. Simulation
terrain resolves it through tile set 0, as the renderer already does, retaining
the record's level and subtile. Missing/unresolved art remains unsupported
rather than becoming invented clear ground.

The retail EB1 regression enumerates all grid coordinates, checks every
traversable cell and setup entity against the authored records, checks
placement rejection at missing origins and round-trips the sparse terrain.
A separate retail-backed fixture compares a clear sentinel to explicit tile 0.
`LocalSize` is projected radar/crop metadata, not an axis-aligned rectangle in
these map-cell coordinates. This fix preserves authored tiles outside that
crop. Blocking those additional border tiles needs controlled original-game
playable-bounds observation and is not inferred from the missing-cell defect.
Sparse support is hashed and stored in simulation save version 3; see
`docs/formats/save.md` for compatibility with older match setups.

## Bounded theater tile indexes

`TileSetIndex::parse` returns a checked `Result` (#292). A supplied
`TilesInSet` must be an unsigned 32-bit integer; a missing count still means
zero. Cumulative expansion is limited to 65,536 filenames and a conservative
16 MiB budget for generated filename bytes plus `String` entries. Each set is
checked before reserving or generating its names, with checked addition and
multiplication. The filename estimate uses the set's longest numeric suffix
(minimum two digits), uppercased prefix/extension and the entry size. Allocator
bookkeeping and the already-parsed INI are outside that estimate.

These are native resource limits, not established original-engine limits.
Oversized indexes and supplied malformed counts now return errors instead of
unbounded expansion or silently treating an invalid count as zero. There is
no partial index or tile-number truncation. Ordinary cumulative numbering,
retired empty sets, uppercase filenames and first-missing-section behavior
remain unchanged. Existing metadata within these bounds keeps the same names.
Map rendering, simulation terrain setup and radar propagate contextual errors;
the radar retains its existing optional-layer recovery policy.

Asset-independent tests cover huge/malformed/aggregate counts, long-prefix and
aggregate storage rejection, the 65,536-name boundary and existing numbering.
Synthetic in-memory/file assets exercise the real terrain, radar and map-render
entrypoints. A receipt-verified `mix-dump render-map` run demonstrates the
recoverable error on synthetic metadata without a PNG. This is loader/parser
verification, not a retail archive scan, complete Session, physical Mac/Retina
or original-game comparison. Simulation and save formats are unchanged.
Missing/malformed terrain templates now return contextual loader errors
([TMP recovery policy](tmp.md), #293). Map-coordinate extent allocation remains
a separate defect (#294).
