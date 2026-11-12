# MIX archives (Westwood)

Evidence-based spec for the `.mix` container used by RA2/YR. Sources: XCC-format
documentation (Olaf van der Spek), EA's bundled XCC reader in
`CNC_TS_and_RA2_Mission_Editor`, and **our own hexdump/parse of the retail files**
at `~/Games/RA2YR/prefix/drive_c/Westwood/RA2` (2026-10-16). Where the two
disagreed, retail data won, see "Empirical notes" at the bottom.

## Layout

A MIX is a flat archive: no filenames on disk, only 32-bit IDs. Files are stored
back-to-back, uncompressed.

```
[u32 flags]              present only if the first u16 == 0 (see "Format detection")
[u8 key_source[80]]      only if flags & ENCRYPTED
u16 file_count
u32 data_size            total size of the body
index: file_count * { u32 id, u32 offset, u32 size }   (offset relative to body start)
body: raw file data (data_size bytes)
[u8 sha1[20]]            only if flags & CHECKSUM, trailer after the body
```

Flag bits (in the 3rd byte of the flags word):

| Constant | Value | Meaning |
|---|---|---|
| `CHECKSUM` | `0x00010000` | 20-byte SHA1 trailer after the body (SHA-1 of the body only, retail-confirmed 2026-10-16) |
| `ENCRYPTED` | `0x00020000` | `key_source` present; header+index are Blowfish-encrypted |

Size identity (XCC's validity check, byte-exact):

- classic: `6 + count*12 + data_size == file_size`
- flagged: `4 + (80 if encrypted) + align8(6 + count*12) + data_size + (20 if checksum) == file_size`

Classic (Tiberian Dawn) archives have no flags word at all: they start with
`u16 count` which is never 0, while flagged archives start with `u16 0`.

Unencrypted flagged headers require all ten bytes of flags, count and data size
before reading fields (#365). Inputs shorter than that return `TooSmall`, including
six- to nine-byte prefixes previously capable of panicking. Authored tests reject
every truncated prefix of zero-flag and checksum-flag archives and retain each
complete payload. This guard does not change complete classic/encrypted parsing
or the strict size identity.

### Encrypted headers

The 80-byte `key_source` yields a 56-byte Blowfish key (XCC `get_blowfish_key`):

1. Take the base64 Westwood public key
   `"AihRvNoIbTn85FZRYNZRcT+i6KpU+maCsEqr3Q5q+LDB5tH7Tz2qQ38V"`, strip the 2-byte
   DER integer header (`02 28`), giving a 40-byte modulus `n`. Public exponent
   `e = 0x10001`.
2. Split `key_source` into 40-byte chunks; each chunk is a little-endian integer.
3. `chunk^e mod n` per chunk; take the low 39 bytes of each result; concatenate.
4. The first 56 bytes are the Blowfish key.

Then Blowfish-ECB-decrypt `align8(6 + count*12)` bytes starting at offset 84
(after flags + key_source). The first decrypted 8 bytes contain
`u16 count, u32 data_size` (6 bytes) plus the first 2 bytes of the index.

## File IDs (the hash)

Names are hashed to `u32` IDs. For RA2/YR (the TS scheme):

1. Uppercase the name; `/` becomes `\`.
2. Pad to a multiple of 4 bytes: if `l & 3 != 0`, append one byte with value
   `l & 3`, then append `name[l & ~3]` (the first byte of the tail) `3 - (l & 3)`
   times. (e.g. `RA2MD.CSF` (9) -> `RA2MD.CSF \x01 F F`.)
3. `id = CRC-32(padded)`, standard ZIP CRC-32 (poly `0xEDB88320`, init
   `0xFFFFFFFF`, final XOR), e.g. `zlib.crc32`.

This is **not** the older rotate-and-add TD/RA1 hash
(`id = id.rotate_left(1) + le_u32(group)`), which only applies to TD/RA1 archives.

Index entries are sorted by `id` as **signed** `i32` on disk.

## Empirical notes (retail corpus)

Observed with `mix-dump`-equivalent parsing on 2026-10-16:

| Archive | flags | Variant | Notes |
|---|---|---|---|
| `theme.mix` | (classic) | plain | 16 RIFF/WAVE themes; `6 + 16*12 + data_size` exact |
| `langmd.mix` | `0x00010000` | checksum trailer | 10 files incl. `RA2MD.CSF` (magic ` FSC`) and nested MIX |
| `ra2.mix` | `0x00030000` | encrypted + checksum | 281 MB |
| `ra2md.mix` | `0x00030000` | encrypted + checksum | 204 MB |
| `language.mix` | `0x00030000` | encrypted + checksum | |
| `maps01.mix` | `0x00020000` | encrypted | |
| `expandmd01.mix` | `0x00030000` | encrypted + checksum | YR 1.001 patch archive |
| `multi.mix` | `0x00020000` | encrypted | |

Known-answer test that settles the hash scheme: `langmd.mix` index contains
`id = 0xbd835079`, and `CRC32(pad_ts("RA2MD.CSF")) == 0xbd835079` while the
rotate-and-add hash gives `0xdb4f6217` (absent). The entry's payload begins with
the CSF magic, confirming the mapping.

The documented constant `0x54C2D545` ("local mix database.dat") is the
**rotate-and-add** hash of that name, the TD/RA1 scheme. XCC-written archives
carry that entry; the hash scheme of the *host* archive still applies to all
normal entries.

### The keypair marker file (id `0x763c81dd`)

`ra2md.mix` and `expandmd01.mix` each contain a 151-byte file at id
`0x763c81dd` (the ID XCC uses for TS/RA2 game detection). Its content is a
Westwood-format INI:

```
[PublicKey]
1=AihRvNoIbTn85FZRYNZRcT+i6KpU+maCsEqr3Q5q+LDB5tH7Tz2qQ38V

[PrivateKey]
1=AigKVje8mROcR8QixnxUEF5b29Curkq01DNDWCdOG99XBqH79OaCiTCB
```

This is primary retail evidence for the public-key constant used in MIX header
decryption (it ships inside the game's own data, matching the value documented
by XCC and other community tools). Reading a user's own archives with it is
what every existing tool does.

### Resolved names (`mix-dump names`)

| Archive | ID | Name |
|---|---|---|
| `ra2md.mix` | `0xa01a9a03` | `URBANN.MIX` (terrain tiles) |
| `ra2md.mix` | `0xcd50b6e6` | `DESERT.MIX` |
| `ra2md.mix` | `0x78c2bb3f` | `LUNAR.MIX` |
| `ra2md.mix` | `0xeb108109` | `SNOWMD.MIX` |
| `expandmd01.mix` | `0x8218f9f4` | `RULESMD.INI` (743,215 bytes) |
| `expandmd01.mix` | `0x7a742519` | `SOUNDMD.INI` (99,392 bytes) |

More nested names confirmed by brute force over candidate stems:
`NTRLMD.MIX` (`0xc93b27a0`), `LOAD.MIX` (`0xbc80c10f`), `LOADMD.MIX`
(`0xdf966243`). Every nested archive in the retail install now has a name.

The side-archive name slice (#311, #310) adds literal known-ID checks for
`BKGDSM.SHP`, `BKGDMD.SHP`, `BKGDLG.SHP` and the seven reported
`SIDEC02MD.MIX` entries: `BKGDSMY.SHP`, `BKGDMDY.SHP`, `BKGDLGY.SHP`,
`RADARY.SHP`, `UIBKGDY.PAL`, `RADARYURI.PAL` and `KEY.INI`. These names
remain scoped by the archive containing each entry; naming them does not mount
multiple side namespaces or enable Yuri gameplay. Asset observations and the
optional retail decode test are described in [menu notes](menus.md).

Authored classic-MIX fixtures using the independently recorded literal IDs
verify CLI name resolution without retail data. They do not establish retail
corpus completeness or reproduce supplied retail artwork. The contributed
retail captures are separately checked by byte size and SHA256; a missing
install causes the retail test to skip, not pass a new retail scan.

### Name recovery from INIs (`mix-dump scan`)

The game names its own assets: `artmd.ini` (`Image=`, `Cameo=`), rules sections,
sound/EVA INIs and theater INIs (`FileName=` + `TilesInSet=`) all hold the
stems. `names::NameDb::learn_from_inis` tokenises every INI found in the
scanned archives and expands each token with extensions, art suffixes (`TUR`,
`BARL` for voxels; `ICON`, `UICO` for cameos; `MK` for build-ups) and theater
letters. It keeps only names whose hash hits a real entry ID.

Across all 16 retail archives (13,061 unique IDs) this learns **11,318 names
in about 3 s**, naming about 88% of entries: SHP 4,798/5,450, TMP 5,023/5,536,
WAV 1,223/1,223, VXL/HVA 139/221 and 139/220.

**Collisions are real.** About 10M candidates against 13k IDs predicts tens of
CRC32 false matches. Early runs produced names like `0CBDACBCBARL.HVA` on TMP
content (from 8-hex-digit digest values in map INIs; those tokens are now
dropped). `scan` therefore keeps a recovered name only when its extension
agrees with the detected content kind, and reports rejections.

**Building art naming** (retail evidence, `ra2.mix`/`ra2md.mix`):
`<side><theater><name>`, e.g. `GACNST.SHP`, `GGCNST.SHP`, `NACNST.SHP`,
`YGCNST.SHP`, `GTCNSTMK.SHP`. The side letter comes first: `G` Allied (a
TS-era GDI holdover), `N` Soviet, `Y` Yuri, `C` civilian. The theater letter
is the **second** character: `A` snow, `G` generic, `T` temperate, `U` urban,
`D` desert, `L` lunar, `N` new urban. Main sprites ship only in snow (`A`)
and generic (`G`) variants. Build-ups (`MK`) ship for every theater.

### Corpus scan exit status

`mix-dump scan <archive.mix>...` prints the census across all supplied archives
and reports parser failures, including recognized malformed CSF assets and
invalid archive entry ranges. A nonempty error list gives exit code 1 after
the census and diagnostics are printed. The total includes every collected
error even when only the first 40 diagnostics are displayed. A clean scan
exits with code 0. Unrecognized binary entries are still counted as binary;
successful scanning does not validate formats without a parser.

An intentionally closed stdout pipe exits successfully, as with the other
inspection commands (#79). Use an output file or let the command finish
printing when checking the corpus; an early consumer such as `head` can stop
the report before its validation result is returned. Other output failures
still fail the command. Synthetic CLI tests in `tools/mix-dump/tests/cli.rs`
exercise malformed archives, nested WAV/CSF failures, continued census
collection, the diagnostic limit and closed-pipe behavior (#111, #113).

## Resolved questions

- The 7,033/7,631-byte binaries in `expandmd01.mix` with prefix
  `0a 05 01 08 ...` are **PCX** images (manufacturer `0x0A`, version 5, RLE,
  8 bpp). The retail install holds 271 PCX files.
- `0x65ac42bd` (360 KB, `00 00 78 02 38 02 01 00`) is a single-frame 632x568
  **SHP** (`docs/formats/shp.md`).
- The 3x 12,946-byte nested MIXes are `DES.MIX`, `LUN.MIX`, `UBN.MIX`: tiny
  per-theater archives holding `AMMO01.SHP`, `WAKE2.SHP` and one more SHP.
- The 337 KB INI `0xb91d69a0` is `C1A02MD.MAP`, a campaign map. Its
  `[CARUS08]`-style sections are per-map rule overrides.

## Open questions

- Unclassified ("binary") entries are down to a handful: `VOXELS.VPL` (voxel
  lighting palette), `CREDITS*.TXT` and a few unnamed blobs. `AUDIO.IDX`/`AUDIO.BAG` are
  the sound bank (`docs/formats/audio.md`). `6POINT`/`8POINT`/`GRAD6FNT.FNT` use the
  classic Westwood font format (u16 file size, u16 2, block offsets).
- About 80 VXL/HVA names are unresolved. They are likely RA2-only units whose stems are
  absent from YR's INIs; try `rules.ini`/`art.ini` from `ra2.mix` explicitly.

## Related

- `langmd.mix` contains a nested MIX (`flags 0x00030000`), nested MIXes must be
  recursed into (XCC `m_mix_expansion` behaviour).
- `.mmx` files (e.g. `EB1.mix` as `EB1.mmx`) are map archives in the same format.

## Native isolated side lookup

The ordinary AssetFs cascade is unchanged. `read_loose` exposes only a named
root game-directory file, with the same case-insensitive mapping used by
`read`; it does not inspect mounted archives. The sidebar loader uses it before
selected `SIDEC01.MIX`/`SIDEC02.MIX` entry lookup, avoiding conflicting shared
filenames while permitting individual loose UI overrides (#86). Invalid
optional art keeps existing bounded/fallback behavior. This is the native
loose-first policy, not a claim that all original per-file exceptions have
been observed. See `sidebar.md` for authored verification and remaining limits.

## Ambiguous loose filenames

Physical game-directory loading rejects multiple nonempty regular files that
normalize to the same case-insensitive lookup name (#360). A case-sensitive
filesystem can hold both `RULESMD.INI` and `rulesmd.ini`; earlier loading silently
kept whichever directory entry arrived last. `AssetFs::open` now returns
`InvalidData` naming the normalized key, before mounting the cascade, instead
of allowing traversal order to choose rules or artwork. Filename debug quoting
keeps control characters escaped in the diagnostic. The check also uses the
existing lossy filename normalization; normalization collisions are ambiguous
regardless of their raw names.

No file is renamed, deleted or rewritten. Empty files and non-regular entries
remain ignored before collision checking; one nonempty file plus an empty case
variant therefore retains the earlier policy. Unique files still resolve
case-insensitively through `read` and `read_loose`. The explicitly ordered
in-memory `from_files` helper retains last-entry-wins behavior for tools and
fixtures; it has no directory-order uncertainty.

This intentionally rejects previously accepted ambiguous installs, which must
resolve their conflicting filenames before loading. It does not change MIX
parsing, simulation or save encoding, or rewrite existing saves. Actual
Linux scratch-directory tests check both creation orders, repeated rejection,
source-byte preservation and ordinary lookup/filtering. Collision fixtures
explicitly skip when the host filesystem cannot create distinct case variants.
Retail scans, original-game behavior and physical Mac checks remain separate.
