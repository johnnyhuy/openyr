# Roadmap

Precedent says 2–3.5 years to a faithful playable port with a small team (re3 ~2y
with ~6 people; OpenRCT2 ~3.5y). We are one person plus agents, aiming for an
incremental, testable path where each milestone is independently useful.

## M0 — Research and repo (done)

- Precedent study: `docs/research/`
- Repo, licence, method: `docs/legal.md`
- Wine testbed retained as reference oracle at `~/Games/RA2YR`

## M1 — Asset layer (current)

- [x] MIX reader: classic + flagged headers, SHA1 trailer, encrypted (RSA/Blowfish) headers
- [x] Filename hash (TS/CRC32 variant) with known-answer tests
- [x] CSF reader (labels, UTF-16 inverted strings)
- [x] INI reader (sections, keys, comments, order-preserving)
- [x] `mix-dump` CLI (list/extract/info/names)
- [x] PAL / colour tables (6-bit RGB palettes)
- [x] SHP reader (cameos, buildings, infantry sprites): every retail frame decodes
- [x] VXL + HVA readers (vehicles)
- [x] TMP terrain tiles
- [x] `mix-dump scan`: recursive census + corpus-wide parser check
- [ ] PCX reader (loading screens, 271 retail files)
- [ ] AUD/IMA-ADPCM audio, WSA/BIK metadata
- [ ] `yr-assets` VFS with MIX cascade (ra2md.mix over ra2.mix, expandmd override order)
- [x] MIX name database: every nested archive named; ~88% of all entries named
      via INI-derived candidates with content-kind collision rejection

## M2 — Typed game data

- [ ] Parse `rulesmd.ini`/`artmd.ini`/`soundmd.ini` into typed TechTypes/Weapons/Warheads
- [ ] `mix-dump inspect <unit>`: show a resolved unit's full property inheritance
- [ ] Name database: close the remaining ~12% (RA2-only VXLs, PCX, audio)

## M3 — Renderer v1

- [ ] wgpu/Metal window, integer-scaled 640x480 virtual viewport
- [ ] TMP terrain rendering from a real map (`mapsmd03.mix`)
- [ ] SHP + VXL drawing with palette + shadow pass
- [ ] Sidebar/cameo UI from `ra2md.csf` strings

## M4 — Simulation v1

- [ ] Deterministic core: map grid, terrain passability, unit movement (fixed-point)
- [ ] Building placement, harvesting loop, production queues
- [ ] Weapons/warheads/damage with the original formulas
- [ ] `sim-bench` headless benchmark; state hashing

## M5 — Playable sandbox

- [ ] Skirmish loop on one map, no AI (scripted spawns), full input
- [ ] Audio hookup (themes, EVA, unit voices)
- [ ] Save/load (deterministic state dump)

## M6 — Fidelity pass

- [ ] `parity-digest`: state-hash comparison vs Wine oracle runs
- [ ] AI (original team/attack logic), superweapons, chronosphere-style teleports
- [ ] Replays: read `.rmx` command streams and re-simulate

## M7 — Modding surface

- [ ] Data-driven unit definitions (INI overrides + new asset slots)
- [ ] QoL patch layer (widescreen UI, hotkeys, queue controls, resolution)
- [ ] Docs for adding units

## M8 — RA3-style units

- [ ] Reference-image-based new sprites/voxels and sounds (original creations,
      inspired-by only — see `docs/legal.md`)
- [ ] Balance pass, skirmish validation

## Deferred / non-goals

- Network multiplayer (until M6+; needs lockstep or rollback decision)
- Perfect parity with 1.001 bugs (parity mode vs fix mode toggle, OpenRCT2-style
  "(original bug)" annotations)
- Windows/Linux ports (free once wgpu + cpal are in; not the focus)
