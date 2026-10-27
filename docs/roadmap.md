# Roadmap

Precedent says 2–3.5 years to a faithful playable port with a small team (re3 ~2y
with ~6 people; OpenRCT2 ~3.5y). We are one person plus agents, aiming for an
incremental, testable path where each milestone is independently useful.

## M0: Research and repo (done)

- Precedent study: `docs/research/`
- Repo, licence, method: `docs/legal.md`
- Wine testbed retained as reference oracle at `~/Games/RA2YR`

## M1: Asset layer (done 2026-10-17; open items carried forward)

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
- [x] PCX reader (8-bit paletted + 24-bit planar; all 271 retail files)
- [x] AUD / AUDIO.BAG+IDX / IMA WAV decoders, bit-exact vs ffmpeg
- [ ] WSA/BIK metadata
- [x] `yr-assets` VFS: engine-order MIX cascade, nested mounts, theaters, loose overrides
- [ ] Side-specific archives (SIDEC##) and per-file loader exceptions
- [x] MIX name database: every nested archive named; ~88% of all entries named
      via INI-derived candidates with content-kind collision rejection

## M2: Typed game data

- [x] Parse `rulesmd.ini`/`artmd.ini` into typed technos/weapons/warheads/projectiles (`yr-rules`)
- [x] `mix-dump rules <gamedir> <ID>`: resolved unit, weapon chain, damage table, art provenance
- [x] INI fidelity: header text after `]` (MirageWH), first-char booleans, map merge
- [x] `[General]`/`[CombatDamage]` globals, houses/countries, skirmish defaults
- [ ] Superweapons, `soundmd.ini`
- [ ] Engine defaults per key, verified against the Wine oracle
- [ ] Name database: close the remaining ~12% (RA2-only VXLs, PCX, audio)

## M3: Renderer v1

- [x] Map parsing: LZO/IsoMapPack5 + overlay packs, all 234 retail maps
      (`docs/formats/map.md`); tile-set index maps tile numbers to TMPs
- [x] TMP terrain rendering from real maps as PNG (`mix-dump render-map`;
      heights, cliff extras, eyeballed on urban + temperate retail maps)
- [x] Overlay objects (ore/gems/walls), `[Terrain]` objects and placed
      infantry/aircraft on the map (`mix-dump render-map`, eyeballed)
- [x] Structures on the map (NewTheater letter + generic-`G` art,
      foundation anchoring, shadow frames, owner-colour remap)
- [x] Voxel vehicles on the map (VXL body+turret projection; HVA
      transforms and normal lighting still open)
- [x] Native window (`openyr`, winit+pixels/wgpu): rendered map with
      drag/WASD pan and wheel zoom; initial redraw after session loading (#50);
      screenshot-vs-PNG check still open
- [x] SHP + VXL drawing with palette + shadow pass for live sim objects
      (`yr-render::sprites`: building art on the footprint, infantry
      standing frames per octant, voxel tanks, house-colour remap,
      half-alpha shadows; window + `--frames` dumps). Building active
      animations and HVA transforms still open (#6)
- [x] Infantry walk, fire and idle sequences from retail `Sequence=`
      data, with all eight facings and matching shadow frames (#6).
      Presentation clocks observe movement and shots at sim ticks;
      malformed sequences fall back to standing art. Playback cadence
      is provisional pending the Wine comparison (`docs/formats/shp.md`).
- [ ] Building active/production overlays, bibs and adjacent-structure
      sorting; Wine side-by-side infantry and Construction Yard crops (#6)
- [ ] Sidebar/cameo UI from `ra2md.csf` strings (cameo build sidebar
      done in M5, structure placement done (#46); CSF names and tabs
      still open (#47))

## M4: Simulation v1

- [x] Deterministic core: yr-sim skeleton (fixed-point, RNG, hash);
      terrain grid with passability/speed per locomotor (retail-verified)
- [x] Unit movement on the grid (A*, occupancy)
- [x] Building placement, harvesting loop (production queues in M5 sidebar)
- [x] Weapons/warheads/damage with the original formulas
- [x] `sim-bench` headless benchmark; state hashing

## M5: Playable sandbox (the playable baseline)

Decided 2026-10-17: baseline first (sandbox, scripted spawns, no AI),
then basic-AI skirmish, then data-driven addons, then roguelite mode.

- [x] Sandbox loop on one map, no AI (scripted spawns), full input
      (session setup + in-window selection/orders + scripted waves:
      5 escalating attack-move waves from the enemy base, stopping once
      its buildings fall; idle wave units hunt the nearest player target,
      base guards join once their base falls; idle units self-defend.
      Production sidebar (#16): retail cameos, `Cost=`, `BuildSpeed`
      build times, gradual payment, stall on no credits, cancel refund,
      low-power slowdown, roll-out from `Factory=` buildings. Headless
      EB1: no production -> Lost, produced army -> Won. Proof frames via
      `openyr <gamedir> EB1.MMX --frames <dir> <tick>...`)
- [x] Playable prototype verified 2026-10-19: EB1.MMX loads and
      renders, the window session goes live with orders and the
      production sidebar, and the scripted skirmish plays to `Won`
      headlessly (`openyr <gamedir> EB1.MMX --frames <dir> 900 1500 1800`).
      Proof frames are kept outside the repo (they contain retail art).
- [x] Audio hookup (#17): retail theme loop, selection/order responses,
      weapon reports, death sounds and EVA notifications through CPAL/CoreAudio;
      M toggles mute. `--frames` exports the shared mix and a cue log alongside
      proof frames. Spatial audio, pitch variation and playlist controls remain
      fidelity work (`docs/formats/audio.md`).
- [x] Structure placement from the sidebar (#46): rules-sourced Allied
      structures gated by `Prerequisite=` (generic `POWER`/`PROC`/...
      groups) and country, built at the construction yard one at a time,
      placed through a green/red footprint ghost under the sim's own
      rules (flat clear land, no units, `Adjacent=` reach to a
      `BaseNormal` building); refineries bring their `FreeUnit=`
      harvester; finished structures cancel for a full refund
      (`docs/formats/rules.md`). Power is judged per owner (#52)
- [ ] Save/load (deterministic state dump)
- [ ] Basic AI skirmish opponent (stage 2)
- [x] Unambiguous workspace launch command (#54): `cargo run --release -- ...`
      defaults to `yr-app` for live sessions and headless proof captures;
      inspection tools remain available through `-p mix-dump` / `-p sim-bench`

## M6: Fidelity pass

- [ ] `parity-digest`: state-hash comparison vs Wine oracle runs
- [ ] AI (original team/attack logic), superweapons, chronosphere-style teleports
- [ ] Replays: read `.rmx` command streams and re-simulate

## M7: Modding surface (pulled forward, right after the baseline)

- [ ] Addon packs: data-driven unit definitions (INI overrides + new asset slots)
- [ ] QoL patch layer (widescreen UI, hotkeys, queue controls, resolution)
- [ ] Docs for adding units

## M8: RA3-style units

- [ ] Reference-image-based new sprites/voxels and sounds (original creations,
      inspired-by only, see `docs/legal.md`)
- [ ] Balance pass, skirmish validation

## R: Roguelite mode (after the baseline)

- [ ] Run structure: chained short battles, escalating scripted forces,
      survivors/credits carry over, seeded determinism
- [ ] Between-battle upgrade choices (1 of 3) and persistent local unlocks

## Deferred / non-goals

- Network multiplayer (until M6+; needs lockstep or rollback decision)
- Perfect parity with 1.001 bugs (parity mode vs fix mode toggle, OpenRCT2-style
  "(original bug)" annotations)
- Windows/Linux ports (free once wgpu + cpal are in; not the focus)
