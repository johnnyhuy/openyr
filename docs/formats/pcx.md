# PCX images

ZSoft PCX v5 with RLE, used for loading screens, menu backgrounds, briefing
art and faction emblems. Parser: `crates/yr-formats/src/pcx.rs`.

## Layout

```
header (128 bytes)
  u8  manufacturer = 0x0A, u8 version = 5, u8 encoding = 1 (RLE), u8 bits = 8
  u16 xmin, ymin, xmax, ymax      width = xmax - xmin + 1
  ...                              DPI, 16-colour palette (unused here)
  u8  planes (offset 65)           1 = paletted, 3 = RGB
  u16 bytes_per_line (offset 66)   per plane, >= width (even-padded)
RLE body                           bytes_per_line * planes * height decoded bytes
[8-bit only] u8 0x0C + 768 bytes of 8-bit RGB palette (last 769 bytes)
```

RLE: a byte `>= 0xC0` is a run of `b & 0x3F` copies of the next byte, anything
else is a literal. Runs may cross scanline and plane boundaries, so the
decoder expands the whole stream before splitting rows. 24-bit scanlines store
the R plane, then G, then B.

The complete decoded stream is a hard boundary (#367). A run longer than the
remaining declared bytes returns `PcxError::InvalidRun`; it is never clipped to
make malformed artwork appear valid. Missing literal/run bytes still return
`Truncated`. Padding and runs crossing rows or planes retain their existing
behavior. Trailing encoded bytes after a complete image retain the existing
policy and are not part of this check.

Authored tests cover all 3,969 combinations of final run length and remaining
capacity from 1 through 63, exact indexed/RGB image lengths, cross-row/plane
padding and truncated pairs. Loading-screen tests omit an overflowing optional
flag, retain its valid neighbor and recover exact source pixels after replacing
the malformed file. Three fallback compositions remain opaque. These fixtures
do not establish compatibility with the retail corpus or original presentation;
the current environment has no retail install.

## Empirical notes (retail corpus)

- 271 PCX files across the 16 archives, all decoding: 233 8-bit paletted and
  38 24-bit 3-plane. No other layouts occur.
- Visual check (`mix-dump render <archive>:<path> out.png`): the 640x400 menu
  radar background (`ra2.mix/LOCAL.MIX/0x58da0eb1`) and the 472x448 Soviet
  emblem (`ra2.mix/NEUTRAL.MIX/0xf1f8cf8a`) render correctly.
- Only 13 PCX names are recovered so far: their stems are not in the INIs.

## Sources

- ZSoft "Technical Reference Manual" (PCX v5), via the widely mirrored spec.
- Verified against retail data as above.
