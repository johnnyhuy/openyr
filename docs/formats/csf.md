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

`yr-formats::csf` exposes `CsfFile::parse(&[u8]) -> Vec<CsfEntry>` plus
lookup by label name (case-insensitive). The renderer/localisation layer
resolves labels at load time; the sim never reads CSF.

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

`yr-app::sidebar` draws both the native window and headless frames. Each
60x48 cameo has a name beside it, wrapped to three lines, followed by its
`Cost=` and duration. The duration is the session's integer `BuildSpeed=`
tick count at 15 ticks/s, rounded up to tenths. For example, the Grizzly
is $700 and 441 ticks (29.4s); the refinery is $2000 and 1260 ticks (84s).
These are full-power estimates; low power or credit starvation can delay
completion. This does not change production formulas or gradual payment.

The panel is 352 pixels wide with two columns, and all current slots fit
in the 800-pixel-high frame. Clicking a name or price uses the same slot
as its cameo. Unaffordable cards and cards without an owned factory or
prerequisites dim; a paid structure remains bright while awaiting placement,
even if the yard is lost. Progress wipes, counts and placement outlines
remain visible. Set `OPENYR_SIDEBAR_DUMP=<dir>` while running
`cargo test -p yr-app --test retail sidebar -- --nocapture` to export
controlled funded, unaffordable, factory-loss and ready-structure panels.

Text uses the public-domain bitmap glyphs from
[`font8x8`](https://docs.rs/font8x8/0.3.1/font8x8/). Latin, extended Latin,
Greek and Hiragana are supported; other glyphs display `?`. Very long
names are clipped after three lines with `..`. Full Unicode font shaping,
category tabs and alternate sidebar layouts remain presentation work.
