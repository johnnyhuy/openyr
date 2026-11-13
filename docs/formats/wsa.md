# WSA metadata

`yr-formats::WsaFile` parses Westwood WSA container metadata and frame
ranges without I/O or codec work. WSA is the XOR-delta animation format
from the Dune II and Command and Conquer era. Frame payloads stay
compressed: each stored frame is LCW data that expands to XOR-delta
commands against the previous image. Decoding either layer is outside
this parser's scope.

## Retail observations

The local Yuri's Revenge install ships no WSA assets. The 2026-11-08
`mix-dump scan` census classifies every entry in the 16 retail archives
(13,061 unique IDs across aud, binary, bink, csf, fnt, hva, ini, mix,
pal, pcx, shp, tmp, vxl and wav) with zero errors and no WSA kind, and
the install directory holds no loose .wsa files. The parser therefore
completes the Westwood format coverage for the earlier games and for
addon content; its checks are synthetic known-answer fixtures plus
hostile-header cases, not retail byte comparisons.

## Container layout

The field layout follows the public format description at
[ModdingWiki: Westwood WSA Format](https://moddingwiki.shikadi.net/wiki/Westwood_WSA_Format),
cross-checked against the Dune II notes at
[savoch.net](https://www.savoch.net/ben/dune2/formats/wsa/). No original
game code was consulted or copied.

The Command and Conquer era header is 14 little-endian bytes: frame
count, X and Y position, width, height, delta buffer size, and flags.
Bit 1 of the flags marks an embedded 768-byte 6-bit RGB palette stored
after the offset table. The delta field is the unpack buffer size the
encoder recorded; published notes report it runs 37 bytes below the
actual requirement, so the parser exposes it without enforcing it.

The offset table holds frame count plus two u32 entries, relative to
the file start with any palette bytes ignored. The first entry is zero
for chained clips that build on a previous file's final frame. The
final entry is either zero, leaving the previous entry as the data
end, or the data end itself, in which case the previous entry starts
an extra loop frame that maps the final image back to the first.

## Parser policy

Counts and sizes are checked with checked arithmetic against the input
length before either table is allocated. Dimensions must be nonzero
and at most 8192 per axis. Offsets must start at the table end (or be
the zero chain marker), strictly increase, and end exactly at EOF
after palette adjustment. Frame ranges reuse the borrowed input; no
payload is copied. The Dune II header variant without position and
flag fields, and the later variant with a u32 delta field, are outside
this parser's scope.

`mix-dump scan` routes entries to this parser only through a recovered
.WSA name, because the format carries no magic bytes. A malformed
named entry reports a scan error like every other strict parser.

## Chained packet bounds (#394)

Authored two-frame clips exposed two gaps in the documented range policy.
`[0, 1, 38, 0]` previously exposed metadata as frame one; palette adjustment
could move that read into the embedded palette. `[30, 34, 38, 38]` exposed an
empty loop packet. Both now return `WsaError::Index`.

The first stored offset of a chained clip must be at least the table end in
palette-free coordinates. A gap after the table remains allowed, including
the existing `[0, 34, 38, 0]` fixture. A nonzero terminal entry must strictly
exceed the preceding endpoint. Zero still means no loop packet. Tests cover
both palette modes, initial/chained clips, loop/no-loop clips, exact-boundary
and gapped starts, metadata overlaps and every truncated valid fixture prefix.
No codec or playback behavior is inferred from these containers. The local
retail install has no WSA files; retail census checks establish only that the
other supported formats remain compatible.
