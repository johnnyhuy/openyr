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
