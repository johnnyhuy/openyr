# Unicode bitmap font

`GAME.FNT` in the retail `RA2.MIX/LOCAL.MIX` is 1,584,195 bytes. It uses
`fonT`, which differs from the `FoNt` bitmap font in `MAP.FNT` and the
older indexed fonts in `6POINT.FNT`, `8POINT.FNT` and `GRAD6FNT.FNT`.
Only `fonT` is decoded by `yr-formats::UnicodeFont` in this slice.

The format below was checked against the installed bytes and independent
pixel decoding. The public
[Unicode BitFont description](https://moddingwiki.shikadi.net/wiki/Westwood_Unicode_BitFont_Format)
identifies the format and its one-bit images. No original engine source,
third-party font decoder source, or original binary instructions were used.

## Layout

All integer fields are little-endian.

| Offset | Type | Observed value | Meaning |
|---|---|---|---|
| 0 | 4 bytes | `fonT` | Magic |
| 4 | u32 | 20 | Ideographic-space width metadata |
| 8 | u32 | 3 | Bytes per bitmap row |
| 12 | u32 | 16 | Bitmap rows per glyph |
| 16 | u32 | 17 | Line height |
| 20 | u32 | 29655 | Physical glyph count |
| 24 | u32 | 49 | Bytes per glyph record |
| 28 | 65536 x u16 | variable | BMP character map |
| 131100 | 29655 x 49 bytes | variable | Glyph records |

Each map entry is a one-based glyph id. Zero is unmapped. Aliases may
share a glyph, including control-code replacement symbols. Every physical
glyph in this install is reachable through a Unicode scalar value. The
format has no supplementary-plane entries.

A record contains a one-byte width followed by `stride * lines` bitmap
bytes. The stored record size must equal `1 + stride * lines`. Rows are
byte-aligned and pixels run from the most significant bit to the least
significant bit. Only the first `width` pixels of each row are visible.
A space has width four and an empty bitmap. The digit `0` is five pixels
wide, `1` four, lowercase `i` two, uppercase `W` eleven and `界` fifteen.

The file size matches exactly:
`28 + 65536*2 + 29655*(1 + 3*16) = 1584195`.

## Validation

The parser checks the full map and record spans against input length using
checked arithmetic before allocating. It rejects out-of-range map ids,
widths beyond the row stride, inconsistent record sizes, zero counts,
truncated tables and oversized dimensions. The format parser caps bitmap
and line dimensions at 4096. It returns owned tables and borrowed glyph
views, with checked pixel access. It performs no filesystem or platform
operations.

Synthetic tests cover aliases, blank spaces, bit order, unmapped and
supplementary characters, hostile header counts, truncated spans and bad
map ids or glyph widths. `game_font_matches_every_retail_unicode_glyph`
checks all 29,655 physical glyphs pixel by pixel against independently
addressed raw records, and checks known glyph widths.
`mix-dump scan` now validates every `fonT` payload it recognizes and reports
the glyph count. Other FNT variants remain classified without decoding.

## Sidebar use

`SidebarArt` reads `GAME.FNT` through the normal VFS, so a loose or patched
font follows the same precedence as other game data. The optional UI loader
accepts up to 64 bitmap columns, 32 rows and 40-pixel line spacing. Missing,
malformed or oversized UI fonts retain the existing `font8x8` fallback.
Font bytes and text layout remain outside saves and simulation state.

The shared window/headless compositor draws glyphs at native size. Credits
are centered by measured width; queue counts and READY labels use the same
metrics. Hover tooltips wrap names at 224 pixels with word boundaries when
possible and a maximum of three name lines. Long words and wide Japanese
characters use glyph advances rather than a fixed character count. A
separate character cap bounds allocations even for zero-width font glyphs.
The tooltip height follows the font line height, and its position is
clamped to the frame. Unsupported characters use the retail question mark.

The current advance is `glyph width + 1`, with the final tracking pixel
excluded from measured width. Line spacing uses the stored value 17.
Credit text starts at canvas y=0, putting its digit ink at y=4 through 12.
Exact original tracking, language-specific space handling, text colors,
shadows and tooltip timing still require observation under #86. Matching
the retail glyph pixels does not establish exact original UI typography.

`retail_text_pixels_metrics_and_wide_tooltips_match_game_font` compares
painted ASCII, Latin and Japanese strings against raw font records,
including missing-character fallback. It checks proportional widths and
long Japanese names. Set `OPENYR_FONT_DUMP=<external-dir>` while running
`cargo test -p yr-app --lib sidebar -- --nocapture` to export the sample.
The existing UI dump also emits the ready tooltip with the native font.
All dumps contain retail art and stay outside git.
