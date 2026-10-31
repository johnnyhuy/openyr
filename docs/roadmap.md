# Roadmap

Precedent says 2–3.5 years to a faithful playable port with a small team (re3 ~2y
with ~6 people; OpenRCT2 ~3.5y). We are one person plus agents, aiming for an
incremental, testable path where each milestone is independently useful.

The playable milestone includes original-style menus, contextual game cursors
and complete interaction feedback (#96, #98, #97), plus native-resolution
world rendering and original effects (#101, #102). Finish these M3/M5 items,
the sidebar work in #86 and repair/sell in #94 before addon/roguelite work or
declaring the continuation schedule complete. This scope was confirmed by
Johnny on 2026-10-31.

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
- [x] Exit cleanly when an inspection output pipe closes early (#79).
      All `mix-dump` commands propagate stdout failures; BrokenPipe exits
      successfully without a panic. Input errors and other output failures
      keep a failing exit code. Synthetic CLI tests run without retail assets.
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

- [x] Preserve native source detail by default (#101, #103): scale 1 retains
      60x30 TMP cells and full-size unit art at zoom 1. Explicit inspection
      scales are limited to 1/2/4. Matched-magnification retail comparisons
      expose the old half-resolution terrain loss (`docs/formats/effects.md`).
- [x] Retail muzzle and vehicle destruction effects (#102): actual firing
      and death events route type/weapon art through ANIM.PAL, bounded caches
      and one-shot presentation clocks. All 78 source frames, colored alpha,
      same-tick fire/death and unchanged sim/save data are checked.
- [ ] Complete original effects and graphics calibration (#102): warhead
      impacts, projectile art/trails, smoke/fire, debris, environmental and
      special-weapon effects, infantry deaths, muzzle offsets, depth, timing
      and transparency. Refine display scaling/filtering and indexed voxel
      lighting against original-game captures.
- [x] Map parsing: LZO/IsoMapPack5 + overlay packs, all 234 retail maps
      (`docs/formats/map.md`); tile-set index maps tile numbers to TMPs
- [x] TMP terrain rendering from real maps as PNG (`mix-dump render-map`;
      heights, cliff extras, eyeballed on urban + temperate retail maps)
- [x] Overlay objects (ore/gems/walls), `[Terrain]` objects and placed
      infantry/aircraft on the map (`mix-dump render-map`, eyeballed)
- [x] Structures on the map (NewTheater letter + generic-`G` art,
      foundation anchoring, shadow frames, owner-colour remap)
- [x] Voxel vehicles on the map (VXL body/turret/barrel projection).
      Authored limb bounds, first HVA poses, fixed camera scale and a
      shared ground anchor replace guessed attachment placement (#73).
      Normal-table lighting remains open
- [x] Geometry-derived voxel lighting (#82): cached local surface normals
      follow HVA poses and facings under a fixed camera light. Palette and
      house colours share bounded ambient/diffuse shading; ambiguous surfaces
      retain their unlit colours. Six retail models keep identical silhouettes
      at eight facings and share map/live shading. Retail indexed normals,
      VPL lookup and Wine calibration remain fidelity work.
- [x] Native window (`openyr`, winit+pixels/wgpu): rendered map with
      drag/WASD pan and wheel zoom; initial redraw after session loading (#50);
      screenshot-vs-PNG check still open
- [x] SHP + VXL drawing with palette + shadow pass for live sim objects
      (`yr-render::sprites`: building art on the footprint, infantry
      standing frames per octant, voxel tanks, house-colour remap,
      half-alpha shadows; window + `--frames` dumps). Static HVA poses
      apply to map and live voxels (#73)
- [x] Authored HVA frame playback for live voxel units (#78). Each body,
      turret and barrel advances within its own actual frame count on an
      absolute presentation clock. Black Hawk rotor poses match retail HVA
      transforms at all eight facings; one-frame vehicles remain static.
      Models are decoded once and unit sprite caching is capped at 512
      entries. The two-tick cadence remains provisional pending Wine.
- [x] Infantry walk, fire and idle sequences from retail `Sequence=`
      data, with all eight facings and matching shadow frames (#6).
      Presentation clocks observe movement and shots at sim ticks;
      malformed sequences fall back to standing art. Playback cadence
      is provisional pending the Wine comparison (`docs/formats/shp.md`).
- [x] Building active/production overlays and damaged variants from retail
      art, refinery/war-factory bibs below live objects, unioned ground
      shadows and foundation-based adjacent-structure ordering (#6).
      Production frames follow queue progress and stop while ready for
      placement (`docs/formats/shp.md`).
- [x] Refinery unloading art follows the actual harvester timer (#68, #6).
      Retail `SpecialAnim=GAREFNOR` plays once, stops at completion,
      and resumes its phase after save/load. Body pixels and damaged
      fallback are checked against all 19 retail frames.
- [x] Complete static war-factory shells (#71): lower shell, roof interior,
      raised side and closed roof panels from retail art. All three factions'
      normal/damaged body pixels and anchors match independent SHP canvases.
      Panels remain closed during rollout pending the Wine comparison (#6).
- [ ] Wine side-by-side infantry and Construction Yard crops; verify
      cadence (including HVA playback), overlay depth adjustments and
      factory doors (#6)
- [x] Sidebar labels from `ra2md.csf` (#47): all 22 Allied entries show
      CSF names, retail costs and full-power build times. Missing/invalid
      strings warn and fall back to rules names or type ids. Cards dim
      for missing prerequisites/factories or insufficient credits; paid
      structures stay bright until placed. Names and hit boxes share the
      window/headless panel (`docs/formats/csf.md`).
- [x] Sidebar category tabs (#65): structures, defences, infantry and vehicles
      filter all 22 entries without changing production ids. Tabs show
      a green marker for paid structures on hidden pages.
      Window and headless frames share card/tab hit boxes and drawing;
      switching tabs backs out of the ghost while keeping the paid structure.

- [x] Retail Allied sidebar chrome and production clock (#84): native
      168-pixel panel, authored category button states and 60x48 cameos at
      the observed screen coordinates. Names, costs and full-power build
      times move into hover tooltips. Clock coverage follows actual queue
      progress and freezes with credit stalls. Active paid work keeps its
      brightness when credits fall below full price (#85). All 54 unfinished masks
      and source panel/button pixels are checked against retail canvases
      (`docs/formats/sidebar.md`). Repair/sell,
      bottom command bar and original-game clock compositing remain open.
- [x] Retail Unicode text (#88): validated `GAME.FNT` parser and native
      sidebar glyphs for credits, queue counts, READY and hover tooltips.
      Text measures variable widths and wraps wide characters; all 29,655
      glyphs match raw retail records. Exact original spacing and colors
      remain in #86 (`docs/formats/font.md`).
- [x] Powered radar and live minimap (#90): actual owned `Radar=yes`
      providers and player power control the 33-frame Allied transition.
      TMP radar colors, live footprints/units/resources, a viewport outline
      and radar navigation/orders share the window/headless panel. Retail
      frame pixels, power loss/recovery, source-cell clicks and save/load
      reconstruction are checked (`docs/formats/radar.md`). Exact cadence,
      color/scale calibration and shroud remain in #86.
- [x] Native repair/sell controls (#94): authored Allied button states,
      K/L input modes, rules-gated owned structure actions, deterministic
      credit-funded repair jobs and sale refunds. Jobs survive save/load;
      sale frees footprints and uses its retail EVA cue. Source pixels and
      controlled retail damage/stall/repair/sale are checked
      (`docs/formats/structure-services.md`). Original rounding/timing,
      cursor/wrench art, deconstruction and survivors remain unverified.
- [x] Allied power meter (#92): retail red/yellow/green pips show current
      player output, consumption and spare power. A demand tick shows
      shortages even at zero output; hover gives exact totals. Placement,
      destruction, enemy isolation, compact windows and save/load are checked
      in the shared window/headless panel (`docs/formats/sidebar.md`).
      Display scale and surplus thresholds still need original-game calibration.
- [ ] Scale generator output with observed health fractions (#93). The
      current simulation keeps full output until destruction, including in
      radar, production and the meter.
- [ ] Complete original sidebar behavior and command bar (#86), including
      radar calibration/shroud, power-meter calibration, repair/sell fidelity, text calibration,
      button transitions,
      queue hold/scrolling and original-game compositing comparisons.
- [ ] Contextual animated game cursors (#98): observed retail frames,
      palettes and hotspots for selection, movement, attack, harvesting,
      placement and other supported commands. Match actual command validation,
      keep size independent of map zoom, handle Retina coordinates and restore
      the system pointer on leave/focus loss/native dialogs. Include cursor
      states in shared headless proof and compare original-game behavior.

## M4: Simulation v1

- [x] Deterministic core: yr-sim skeleton (fixed-point, RNG, hash);
      terrain grid with passability/speed per locomotor (retail-verified)
- [x] Unit movement on the grid (A*, occupancy)
- [ ] Reject absent map tiles inside the terrain bounding rectangle (#105).
      Map-built grids currently leave those holes as passable clear ground.
- [x] Building placement, harvesting loop (production queues in M5 sidebar)
- [x] Full-footprint refinery docking (#68, #69): harvesters deliver beside
      the retail 4x3 foundation and retain cargo if their refinery is lost.
      Two retail EB1 deliveries earn 1000 credits; a mid-unload snapshot
      follows the uninterrupted sim hash through the next delivery.
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
- [x] Diagnose and recover from live audio device interruptions (#60).
      Backend kind/message diagnostics distinguish active-stream notices
      from device loss. A separate audio worker retries transient failures
      with 0.5-8 second backoff, restarts music and preserves mute; expired
      effects/speech are discarded. Permission/configuration errors disable
      playback with a diagnostic. Device work never waits on sim ticks
      (`docs/formats/audio.md`).
- [x] Structure placement from the sidebar (#46): rules-sourced Allied
      structures gated by `Prerequisite=` (generic `POWER`/`PROC`/...
      groups) and country, built at the construction yard one at a time,
      placed through a green/red footprint ghost under the sim's own
      rules (flat clear land, no units, `Adjacent=` reach to a
      `BaseNormal` building); refineries bring their `FreeUnit=`
      harvester; finished structures cancel for a full refund
      (`docs/formats/rules.md`). Power is judged per owner (#52)
- [x] Save/load (#45): versioned integer-only sim snapshots, seed and
      map/rules compatibility checks, atomic disk replacement, F5/F9
      quicksave/load, live `--load` and headless `--save`/`--load` with
      absolute `--ticks`/`--frames`. Retail EB1 round-trips at five
      checkpoints and matches uninterrupted hashes through tick 3000
      to Won (`docs/formats/save.md`). Presentation is rebuilt on load.
- [x] Basic AI skirmish opponent (#21): computer production in `yr-sim`,
      separate owner credits/queues/power, retail costs and build times,
      a refinery-gated income stipend, an infantry scout and periodic
      attack groups. Rebuilds missing power/refinery/factory/barracks
      through normal structure queues and placement. Retail EB1 proof
      runs exercise both Won and Lost. Original AI logic remains in M6;
      see `docs/skirmish-ai.md` for the prototype policy.
- [x] Unambiguous workspace launch command (#54): `cargo run --release -- ...`
      defaults to `yr-app` for live sessions and headless proof captures;
      inspection tools remain available through `-p mix-dump` / `-p sim-bench`
- [x] Finder launch without terminal arguments (#75, #76): `scripts/package-macos.sh`
      builds a local native `OpenYR.app`. Default EB1 skirmish uses discovered
      retail data or a remembered folder chosen through macOS. Startup errors
      appear in a native alert; Finder logs live in `~/Library/Logs/OpenYR`.
      Explicit CLI and headless commands still work (`docs/macos-app.md`).
- [ ] Native menu and match flow (#96): original-style main menu, skirmish
      setup/map preview, saved matches, pause/resume, save/load, restart,
      options and quit. Add Won/Lost screens, loading/error feedback and
      confirmation before discarding a match. Pausing stops sim ticks without
      catch-up on resume; modal input never reaches map commands. Preserve
      explicit CLI/headless launches and prove each menu state on its PR.
- [ ] Complete world interaction and desktop UX (#97): consistent selection,
      supported control groups/hotkeys, hover/target/order feedback, bounded
      camera movement and configurable edge scrolling. Clear held input on
      focus/menu changes; verify resizing, Retina hit coordinates, tooltip
      clipping and visible save/load feedback. Sidebar queues and command-bar
      controls remain in #86; repair/sell remains in #94.

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
