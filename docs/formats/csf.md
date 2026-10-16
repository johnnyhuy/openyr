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
then num_labels labels:
  char[4] magic       " LBL"
  u32     num_strings (1 in practice)
  u32     name_len
  char    name[name_len]            ASCII label name, e.g. "CRD:CREDITS" (no NUL)
  then num_strings strings:
    char[4] magic     " RTS" = value present, "NRTS" = empty string
    if " RTS":
      u32   value_len               in UTF-16 code units
      u16   value[value_len]        UTF-16LE, each code unit bitwise-NOT (XOR 0xFFFF)
```

Decoded value: invert each `u16`, interpret UTF-16LE.

## Empirical notes

`langmd.mix` -> `RA2MD.CSF` (id `0xbd835079`, 573,269 bytes) begins
`" FSC" 03 00 00 00 | 5b 14 00 00 | 5b 14 00 00` = version 3, 5,211 labels,
5,211 strings. Label strings in the file use `{CRD:...}`-style markup as part of
the text (e.g. credits lines), i.e. braces are content, not syntax.

## Usage in the engine

`yr-formats::csf` exposes `CsfFile::parse(&[u8]) -> Vec<CsfEntry>` plus
lookup by label name (case-insensitive). The renderer/localisation layer
resolves labels at load time; the sim never reads CSF.
