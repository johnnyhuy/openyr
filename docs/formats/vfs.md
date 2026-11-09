# Asset VFS: engine lookup order

`crates/yr-assets` presents a YR install as one file namespace, the way
`gamemd.exe` does: a filename resolves to the **first** source that holds it.
`mix-dump vfs <gamedir> [--theater <name>] [file...]` prints the cascade and
where each file comes from.

## Order (YR 1.001)

1. **Loose files** in the game directory. These are checked first, so a
   loose `rulesmd.ini` overrides every archive (community-documented: "Lastly, it
   reads any files loose in the RA2 directory, again overriding").
2. `LANGMD.MIX`, `LANGUAGE.MIX`
3. `EXPANDMD99.MIX` … `EXPANDMD00.MIX`, higher numbers first
4. `RA2MD.MIX`, `RA2.MIX`
5. `CACHEMD`, `CACHE`, `LOCALMD`, `LOCAL`, `AUDIOMD`
6. `ECACHE*.MIX`, then `ELOCAL*.MIX` (alphabetical ascending, as listed by
   the file system)
7. `CONQMD`, `GENERMD`, `GENERIC`, `ISOGENMD`, `ISOGEN`, `CONQUER`,
   `CAMEOMD`, `CAMEO`
8. `MAPSMD03`, `MULTIMD`, `THEMEMD`, `MOVMD03`
9. Not in ModEnc's table but present in retail and needed for their contents:
   `AUDIO`, `NTRLMD`, `NEUTRAL`, `LOADMD`, `LOAD`
10. Theater archives (`set_theater`), in this order: YR tile set, RA2 tile set, YR art,
    RA2 art, per-theater extras. For snow that is `ISOSNOMD`, `ISOSNOW`, `SNOWMD`,
    `SNOW`, `SNO`. Absent candidates are skipped; retail has no
    `TEMPERATMD.MIX`.

Nested archives are themselves resolved through the cascade. `CACHEMD.MIX`
mounts from inside `RA2MD.MIX`, and `AUDIOMD.MIX` from inside `LANGMD.MIX`.

Source for steps 2 to 8: ModEnc "MIX" (loading hierarchy table and notes,
including ECACHE ordering in RA2/YR). Vanilla YR ranks `LANG*.MIX` above
`EXPANDMD##`; Ares reverses that, and this VFS follows vanilla.

## Retail verification

`crates/yr-assets/tests/retail.rs` against the local install:

| File | Resolves from | Note |
|---|---|---|
| `RULESMD.INI` | `EXPANDMD01.MIX` (743,215 B) | shadows the 1.000 copy in `RA2MD.MIX/LOCALMD.MIX` (742,958 B, 102 lines differ) |
| `SOUNDMD.INI` | `EXPANDMD01.MIX` | 1.001 patch |
| `RA2MD.CSF` | `LANGMD.MIX` | |
| `UNITTEM.PAL` | `RA2.MIX/CACHE.MIX` | |
| `CEVA060.WAV`, `AUDIO.BAG` | `LANGMD.MIX/AUDIOMD.MIX` | YR's 37.6 MB bank shadows RA2's 17.6 MB `AUDIO.BAG` |
| `CLEAR01.TEM` | `RA2.MIX/ISOTEMP.MIX` | only with the temperate theater set |
| `GACNST.SHP` | `RA2.MIX/SNOW.MIX` | only with the snow theater set |

Opening the full install (memory-mapped, 27 mounts) takes about 0.4 s.

## Known gaps

- **Side archives** (`SIDEC01.MIX`, `SIDEC02MD.MIX`, `SIDENC0x.MIX`) hold
  per-faction sidebar/UI art under identical names, so they must be selected
  by side and not mounted together. The Allied sidebar now reads
  `SIDEC01.MIX` in a separate UI namespace (`docs/formats/sidebar.md`).
  Allied and Soviet sidebar art loads by requested side. The Yuri `SIDEC02MD.MIX`
  entries are fully hash-named (`docs/formats/menus.md`), but Yuri
  gameplay and its sidebar remain unwired.
- **Per-file exceptions.** ModEnc notes the engine expects some files only
  from specific archives (e.g. `unit*.pal` from cached mixes, not
  `EXPANDMD##`). The VFS applies one global order; exceptions will be added
  when the renderer/loader hits them.
- RA2-only content (`MAPS01/02`, `MULTI`, `THEME`, `MOVIES01/02`) is not
  mounted; YR's table does not list it.
