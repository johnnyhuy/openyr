# CSF string tables

Evidence-based spec for `.csf` (compiled string files, e.g. `RA2MD.CSF` inside
`langmd.mix`). Sources: retail hexdump (2026-10-16) plus published XCC/ModEnc
documentation.

## Layout

```
char[4]  magic        " FSC"           (bytes 20 46 53 43)
u32      version      3
u32      num_labels
u32      num_strings
u32[2]   reserved     8 zero bytes (retail confirmed, 2026-10-16)
then num_labels labels:
  char[4] magic       " LBL"
  u32     num_strings (1 in practice)
  u32     name_len
  char    name[name_len]            ASCII label name, e.g. "CRD:CREDITS" (no NUL)
  then num_strings strings:
    char[4] magic     " RTS" = plain value
                      "WRTS" = value + trailing ASCII sound reference
                      "NRTS" = empty string
    if value:
      u32   value_len               in UTF-16 code units
      u16   value[value_len]        UTF-16LE, each code unit bitwise-NOT (XOR 0xFFFF)
    if "WRTS":
      u32   ref_len
      char  ref[ref_len]            ASCII, e.g. "ceva001e" (the EVA voice sample)
```

Decoded value: invert each `u16`, interpret UTF-16LE.

## Empirical notes

`langmd.mix` -> `RA2MD.CSF` (id `0xbd835079`, 573,269 bytes) begins
`" FSC" 03 00 00 00 | 5b 14 00 00 | 5b 14 00 00 | 00 00 00 00 00 00 00 00`
= version 3, 5,211 labels, 5,211 strings, 8 reserved zero bytes. Retail counts:
4,007 " RTS" + 1,204 "WRTS" + 0 "NRTS" = 5,211. The WRTS records are exactly the
`VOX:*` EVA labels (`VOX:ceva001` -> value "Warning: Nuclear Silos detected",
sound `ceva001e`), confirming the reference field is the voice sample stem.
Label strings use `{CRD:...}`-style markup as part of the text (e.g. credits
lines), i.e. braces are content, not syntax. First label: `THEME:Intro` with
value `Intro` (each UTF-16 unit stored inverted, `ff b6` = `!0x0049`).

## Usage in the engine

`yr-formats::csf` exposes `CsfFile::parse(&[u8]) -> Result<CsfFile, CsfError>` plus
lookup by label name (case-insensitive). The renderer/localisation layer
resolves labels at load time; the sim never reads CSF.

## Bounded decoded storage

The parser checks the table's minimum label/value bytes before reserving entry
storage: each label requires 12 bytes and every value requires at least its
four-byte tag. Label-local counts cannot exceed the remaining declared table
total or the available value tags. Name, UTF-16 and sound-reference ranges use
checked arithmetic before reading. Count mismatches are errors; empty tables
and zero-value labels remain supported.

A label with multiple values stores its decoded name once in `Arc<str>`.
`CsfEntry::label` exposes that shared immutable name instead of `String` (#401).
Previously, a 4,644-byte authored table with one 4,096-byte label and 128 null
records created 128 separate name buffers, totaling 512 KiB. Shared names keep
label storage proportional to input bytes, including when a parsed table is
cloned, without rejecting valid multi-value tables. Values and sound references
retain their original order; case-insensitive lookup still returns the first
value for the first matching label. This is parser storage policy, not an
original-engine allocation claim.

Authored tests check shared backing pointers, Unicode/sound/null records,
every truncated prefix, hostile counts/lengths, native localization fallback
and actual `mix-dump scan` behavior beside valid tables. The retail known-answer
check retains all 5,211 RA2MD.CSF entries, including its voice references.

## Production sidebar

`yr-app::localization` reads `RA2MD.CSF` through the normal asset lookup
order once per session setup and resolves each buildable's `UIName=` by
case-insensitive label lookup. Missing tables, parse failures, missing/null
values and empty strings log a warning and fall back to the rules `Name=`,
then the type id. Names stay in presentation data, excluded from the sim
state and the save's setup fingerprint.

The retail EB1 check in `crates/yr-app/tests/retail.rs` compares every one
of the 22 Allied slots with an independent CSF lookup. Labels checked
in this install on 2026-10-30 include `Name:MTNK`, `Name:GAREFN`, and
the `UIName=` references for `AMRADR` and `GAROBO`. The app's unit tests cover
case differences, Unicode values and all fallback cases without retail data.

`yr-app::sidebar` draws both the native window and headless frames. The
168-pixel Allied panel uses retail chrome, category buttons and two columns
of 60x48 cameos. Hovering a cameo shows its localized name, `Cost=` and
full-power duration in a bounded tooltip over the map. The duration is the
session's integer `BuildSpeed=` tick count at 15 ticks/s, rounded up to
tenths. For example, the Grizzly is $700 and 441 ticks (29.4s); the refinery
is $2000 and 1260 ticks (84s). Low power or credit starvation can delay
completion. This does not change production formulas or gradual payment.

All entries in each current category fit in the 800-pixel-high frame.
Unavailable cameos dim; paid structures remain bright while awaiting
placement, even if the yard is lost. An active queue uses the retail
angular clock mask instead of a vertical wipe. Unfinished paid work does
not dim a second time when credits fall below its original full price.
Counts and placement outlines remain visible. Set
`OPENYR_SIDEBAR_DUMP=<dir>` while running
`cargo test -p yr-app --test retail sidebar -- --nocapture` to export
controlled funded, unaffordable, factory-loss and ready-structure panels.
See [sidebar art and geometry](sidebar.md) for source pixels and remaining
original-game comparison work.

Text uses the installed `GAME.FNT` bitmaps with variable glyph widths,
falling back to public-domain `font8x8` glyphs when the font is absent or
invalid. Unsupported characters use the retail question mark. Names wrap
at 224 pixels and at most three lines, ending with `..` when truncated.
The line height comes from the font. Font bytes remain outside save
compatibility and simulation hashes. See [font format and checks](font.md)
for raw-glyph comparisons and remaining typography calibration.

## Sidebar categories

The tabs are Structures, Defences, Infantry and Vehicles. Retail
`RULESMD.INI` uses `BuildCat=Combat` for the defence page, including
non-firing structures such as the Gap Generator and SpySat Uplink.
`BuildCat=Power`, `Resource` and `Tech` use the structure
page. The app compares Combat without case sensitivity and puts absent
or unknown building categories on Structures; this fallback is a prototype
presentation policy, not a measured engine default. Infantry and vehicle
pages follow the registered techno kind.

The retail EB1 sidebar test on 2026-10-30 checks all 22 slots against raw
INI values. It finds nine Structures, seven Defences, two Infantry and
four Vehicles. The defence order is NASAM, GACSPH, GAWEAT, ATESLA,
GASPYSAT, GAGAP, GAPILL. This groups superweapon buildings without
implementing their effects.

Filtering keeps the original session slot as the production kind tag.
Drawing, clicks, queue counts, progress, cancellation and placement all
use that tag. Queue counts appear on unit cameos; a finished structure
marks its tab with a green underline, including on hidden pages. Changing
tabs backs out of the placement ghost but keeps the paid structure. Click its
card again to place it. Category metadata and the selected tab are
presentation only and do not affect saves or deterministic state hashes.

`--frames` writes each requested tick with all four tabs through the same
composition code as the live window. Structures retains `frame-<tick>.png`;
the other files append `-defences`, `-infantry` or `-vehicles`. The retail
sidebar tests also export each tab at funded, in-progress, ready and
defence-ready states through `OPENYR_SIDEBAR_DUMP`.
