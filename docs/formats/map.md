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

## `[IsoMapPack5]` — terrain cells

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
cliff faces aligning with their tops, and shorelines closing — confirming
the 15 px-per-level step and the extra placement. Not yet drawn: overlay
objects (ore/gems/walls), `[Terrain]` objects, and placed
structures/units (all SHP/VXL art).

## Hostile-input posture

Chunk headers, counts and sizes are never trusted: decoders bound output
by the declared per-chunk size, require exact size matches, and the map
parser rejects packs whose decompressed shape is not exactly the retail
shape (entry stream + zero trailer; exactly 262,144 overlay bytes).
Unit tests cover truncated chunks, lying chunk sizes, a corrupted iso
trailer, and a short overlay pack.
