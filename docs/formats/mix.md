# MIX archives (Westwood)

Evidence-based spec for the `.mix` container used by RA2/YR. Sources: XCC-format
documentation (Olaf van der Spek), EA's bundled XCC reader in
`CNC_TS_and_RA2_Mission_Editor`, and **our own hexdump/parse of the retail files**
at `~/Games/RA2YR/prefix/drive_c/Westwood/RA2` (2026-10-16). Where the two
disagreed, retail data won — see "Empirical notes" at the bottom.

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
[u8 sha1[20]]            only if flags & CHECKSUM — trailer after the body
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
3. `id = CRC-32(padded)` — standard ZIP CRC-32 (poly `0xEDB88320`, init
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
**rotate-and-add** hash of that name — the TD/RA1 scheme. XCC-written archives
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

Most nested archives in `ra2md.mix` remain unresolved (`mix-dump names` prints
`? (nested mix)`); extend `KNOWN_NAMES` in `tools/mix-dump` as names are
discovered, or read nested contents recursively to identify by shape.

## Open questions

- Format of the 7,033/7,631-byte binaries in `expandmd01.mix` (identical
  16-byte prefix `0a 05 01 08 00 00 00 00 b7 01 23 00 48 00 48 00`, likely
  sprites/voxels of the SHP family but the header matches no recalled spec —
  read ModEnc's SHP page and reverse the layout properly).
- Format of the 360 KB binary `0x65ac42bd` (starts `00 00 78 02 38 02 01 00`)
  and the 3× 12,946-byte nested MIXes in `ra2md.mix` (each holds 3 small
  binaries of 4,896 / 5,480 / 2,488 bytes — cameo-SHP-shaped).
- The 337 KB INI `0xb91d69a0` in `expandmd01.mix` has one section per map
  (`[CARUS08]`, `[CAWASH17]`, ...) with `Name=` display strings; likely the
  1.001 map-pack index. Needs its real filename.

## Related

- `langmd.mix` contains a nested MIX (`flags 0x00030000`) — nested MIXes must be
  recursed into (XCC `m_mix_expansion` behaviour).
- `.mmx` files (e.g. `EB1.mix` as `EB1.mmx`) are map archives in the same format.
