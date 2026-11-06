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
- [x] Keep the required macOS CI job on an explicit standard Intel runner
      (#184), retaining all checks after hosted arm64 runner acquisition
      failed before any steps ran. Local retail/window verification also
      exercises Apple Silicon.

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
- [x] Fail the corpus check when the census contains parser errors (#111,
      #113), including malformed CSF files. Print the census and first 40
      diagnostics before failing; intentionally closed output pipes still
      exit successfully. Synthetic CLI tests cover malformed archives,
      nested assets and the uncapped error total.
- [x] Use an actual closed pipe for CLI output tests (#202), avoiding an
      intermittent macOS socket fixture failure in the required workspace suite.
- [x] PCX reader (8-bit paletted + 24-bit planar; all 271 retail files)
- [x] AUD / AUDIO.BAG+IDX / IMA WAV decoders, bit-exact vs ffmpeg
- [x] Bink 1 container metadata (#110): all 146 retail Bink entries validate
      with bounded frame/track tables and packet ranges (`docs/formats/bink.md`).
- [ ] WSA metadata
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
- [x] Put ground-position combat art into world footprint order (#204, #102).
      Muzzle, impact/destruction clips and cannon bodies can pass behind
      foreground units/buildings while front effects retain alpha blending.
      Actual retail fatal shots check rear/front overlap, saved-flight cache
      reconstruction and unchanged sim/save bytes. Original layer overrides,
      muzzle offsets, projectile height and static baked objects remain open.
- [ ] Complete original effects and graphics calibration (#102): special
      warhead impacts, other projectile classes/trails, smoke/fire, debris, environmental and
      special-weapon effects, special infantry deaths, muzzle offsets, depth, timing
      and transparency. Refine display scaling/filtering and indexed voxel
      lighting against original-game captures.
- [x] Ordinary infantry death sequences (#102, #137): explicit lethal-weapon
      `InfDeath=1/2` selects retail `Die1/Die2`, retaining victim house art,
      body/shadow frames and world depth outside simulation state. Saved
      flights work after shooter loss; fatal metadata handles same-tick victim
      spawn or reassignment. One-shot expiry, source pixels and redraw/save
      invariance are checked. Original cadence, corpses and
      special death selectors remain in #102 (`docs/formats/shp.md`).
- [x] Human infantry electrocution (#102, #180): lethal `InfDeath=5` routes
      actual fatal-hit provenance to retail ELECTRO art, including its bounded
      finite loop and tail in world painter order. All 15 source frames,
      eligibility, expiry, saved flights after shooter loss and unchanged
      sim/save bytes are checked. Original cadence/loop semantics, scorch,
      animation reports and other special deaths remain unverified
      (`docs/formats/effects.md`).
- [x] Human infantry Nuke Melt deaths (#102, #182): lethal `InfDeath=7`
      selects the 60-frame retail NUKEDIE clip in world painter order.
      Every source frame, eligibility, expiry, saved flights after shooter
      loss and unchanged sim/save bytes are checked. Original timing,
      lighting, scorch, animation reports and non-human exceptions remain
      unverified (`docs/formats/effects.md`).
- [x] Human infantry Yuri head-pop deaths (#102, #185): lethal `InfDeath=6`
      resolves `[General] InfantryHeadPop` to the 30-frame retail YURIDIE
      clip in world painter order. All source frames, eligibility, expiry,
      general-setting overrides, saved flights after shooter loss and unchanged
      sim/save bytes are checked. Original timing, lighting, scorch and reports
      remain unverified; Yuri deployment and area damage are separate simulation
      work (`docs/formats/effects.md`).
- [x] Human infantry Virus deaths (#102, #187): lethal `InfDeath=8`
      resolves `[General] InfantryVirus` to the 35-frame retail VIRUSD clip
      in world painter order. All source frames, eligibility, expiry,
      general-setting overrides, saved flights after shooter loss and unchanged
      sim/save bytes are checked. Original timing, lighting, gas particles,
      scorch and reports remain unverified (`docs/formats/effects.md`).
- [x] Human infantry Brute smash deaths (#102, #188): lethal `InfDeath=10`
      resolves `[General] InfantryBrute` to the 60-frame retail BRUTDIE
      clip in world painter order. Every source frame, eligibility, expiry,
      general-setting overrides, saved flights after shooter loss and unchanged
      sim/save bytes are checked. Original timing, lighting, scorch and reports
      remain unverified (`docs/formats/effects.md`).
- [x] Human infantry Mutate deaths (#102, #218): lethal `InfDeath=9`
      resolves `[General] InfantryMutate` to the 60-frame retail GENDEATH
      clip in world painter order. Every source frame, eligibility, expiry
      and general-setting overrides are checked. No retail weapon carries
      the Mutate warheads, so selector resolution is exercised directly;
      Genetic Mutator simulation and the mutation transform remain separate
      work. Original timing, lighting, scorch and reports remain unverified
      (`docs/formats/effects.md`).
- [x] Initial human infantry explosion deaths (#102, #191, #192): lethal
      `InfDeath=3` resolves `[General] InfantryExplode` to retail S_BANG34
      with its declared translucency in world painter order. Generic deaths
      use effect blending and bypass unit shadows. All 13 source
      frames, eligibility, expiry, overrides, saved flights after shooter loss
      and unchanged sim/save bytes are checked. Flying fragments, original
      timing/lighting, crater/scorch and reports remain unverified
      (`docs/formats/effects.md`).
- [x] Initial stationary human infantry burn collapse (#102, #194): lethal
      `InfDeath=4` resolves `[General] FlamingInfantry` to retail FLAMEGUY.
      Bounded layout validation separates eight running groups and the
      trailing half from the 60-frame collapse. Source canvases, eligibility,
      expiry, overrides, saved lethal provenance and unchanged sim/save bytes
      are checked. Running motion, original timing, palette lighting, shadows
      and reports remain unverified (`docs/formats/effects.md`).
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
- [x] Warhead target impacts from retained shot provenance (#102, #123), including
      instant shots, projectile arrival and in-flight save/load. Ordinary
      AnimList selection uses delivered damage; AP and SA source pixels and
      shared composition are checked (`docs/formats/effects.md`). Exact
      original cadence, depth, lighting and special overrides remain open.
- [x] Draw single-frame retail cannon bodies at authoritative flight positions
      (#102, #122), including exact source pixels, invisible-weapon exclusion,
      bounded optional decoding and immediate in-flight save/load reconstruction.
      Original arcs, launch offsets, trajectories, lighting and depth remain open.
- [x] Directional retail missile bodies (#206, #102): bounded 32-facing SHP
      sheets (including an empty trailing half) follow the current axis-clamped
      homing direction in world depth. All DRAGON/MEDUSA source frames, eight
      firing directions, moving targets, shooter loss and fresh-cache saved
      flights are checked without changing hashes or save bytes. Original
      orientation calibration, original trail behavior, height, offsets and lighting
      remain open.
- [x] Initial configured missile line trails (#208, #102): actual movement
      segments, including arrival, use DRAGON/MEDUSA color and decrement
      settings in bounded transient world-depth history. Turning, shooter
      loss, source settings, expiry, save continuation and unchanged sim/save
      bytes are checked. Original fade cadence, raster/blend policy, height,
      offsets and precise layer behavior remain uncalibrated.
- [x] Initial explicit projectile trailer animations (#210, #102): actual
      flight movement resolves smoke/bubble clips through retained weapon
      identity, with bounded history and source alpha. Retail canvases,
      turns, shooter loss, arrival, expiry and two fresh-cache save
      continuations are checked. Original emission timing, motion, height
      and surface-layer behavior remain uncalibrated; legacy smoke weapons
      use temporary map references in the proof (`docs/formats/effects.md`).
- [x] Give the controlled cannon proof shooter player ownership (#131), so its
      real tank body is drawn alongside the muzzle, projectile and enemy target.
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
      (`docs/formats/sidebar.md`). Original repair/sell animation,
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
- [x] Scale generator output with health fractions (#93): produced power
      uses `power * hp / max_hp` with truncating integer division in the
      shared owner totals, so radar, production and the meter all see the
      reduced output. Drain stays unscaled. Damage, destruction, ownership
      isolation and save/load are checked in `yr-sim`. The manual confirms
      health-dependent output; the exact original rounding still needs
      Wine observation.
- [x] Hold/resume sidebar production (#86, #197): right-click suspends the
      unfinished factory head, left-click resumes without queuing a copy,
      and another right-click cancels with an exact refund. Progress/payment
      freeze while held, other queues continue, and held state survives saves.
      Native hold text and original multi-copy input details still need
      calibration (`docs/formats/sidebar.md`).
- [x] Scroll overflowing sidebar categories (#201, #86): per-category row
      positions, wheel/trackpad accumulation and bounded bottom controls share
      cameo geometry with drawing, hover and production clicks. Compact retail
      compositions reach every original slot and preserve saved match bytes;
      held and READY queues retain their actions. Wheel input over the panel
      leaves map zoom unchanged. Original arrow styling and timing remain open
      (`docs/formats/sidebar.md`).
- [ ] Complete original sidebar behavior and command bar (#86), including
      radar calibration/shroud, power-meter calibration, repair/sell fidelity, text calibration,
      button transitions,
      scrolling calibration and original-game compositing comparisons.
- [x] Native contextual cursor rendering: YR's 517-frame sheet and its
      palette, shared select/move/attack/service resolution, authored blocked
      states and fixed-size UI composition. Left clicks select/order and
      right clicks cancel (#100); Ctrl+Shift attack-move and Shift toggling
      share input validation. Focus/leave restore the system pointer and clear
      stale gestures. Elevated ground picking and box anchors match drawing
      (#104). Source canvases, route eligibility and native composition are
      checked (`docs/formats/cursors.md`). Hotspots and cadence are provisional.
- [x] Bounds-aware edge-scroll cursors (#98): retail green/slashed arrows
      use the same guarded direction as camera motion. Partially blocked
      corners show the remaining axis; fully blocked edges show a slash.
      Edge clicks cannot issue world/placement/service commands. Source
      pixels, inward native hotspots, all directions, zoom/bounds and complete
      match save invariance are checked. Original hotspots/behavior remain
      unverified (`docs/formats/cursors.md`).
- [ ] Contextual animated game cursor parity (#98): observed retail frames,
      palettes and hotspots for selection, movement, attack, harvesting,
      placement and other supported commands. Match actual command validation,
      keep size independent of map zoom, handle Retina coordinates and restore
      the system pointer on leave/focus loss/native dialogs. Include cursor
      states in shared headless proof and compare original-game behavior.

## M4: Simulation v1

- [x] Use raw leptons for physical projectile velocities and preserve
      explicit invisible instant hits (#122). Bound unsafe rules speeds and
      verify launch versus arrival, shooter death and in-flight save/load.
      Correct the retail Rhino duel to use its actual 120mm primary (#128).
      The factor of four and original trajectories still need #19 observation.
- [x] Deterministic core: yr-sim skeleton (fixed-point, RNG, hash);
      terrain grid with passability/speed per locomotor (retail-verified)
- [x] Unit movement on the grid (A*, occupancy)
- [x] Reject absent map tiles inside the terrain bounding rectangle (#105).
      Missing coordinates block movement, placement, setup searches and world
      orders; sparse support survives version-3 saves and changes setup keys.
      Authored border tiles outside LocalSize remain available pending original
      playable-bounds observation. Synthetic rectangles remain fully populated.
- [x] Preserve authored clear-sentinel tiles in simulation terrain (#125).
      Resolve them through tile set 0, matching the map renderer, rather than
      treating their records as missing ground.
- [x] Building placement, harvesting loop (production queues in M5 sidebar)
- [x] Full-footprint refinery docking (#68, #69): harvesters deliver beside
      the retail 4x3 foundation and retain cargo if their refinery is lost.
      Two retail EB1 deliveries earn 1000 credits; a mid-unload snapshot
      follows the uninterrupted sim hash through the next delivery.
- [x] Weapons/warheads/damage with the original formulas
- [x] Keep fired shells after shooter death (#119), with original damage and
      weapon provenance through arrival and save/load. Target loss still
      discards homing shots; a moving-target regression covers continuation.
- [x] Preserve legacy setup fingerprints when adding shot metadata (#121),
      including AI build profiles and FreeUnit weapons. Version-1 match saves
      remain loadable; version 2 retains full new shot provenance.
- [x] `sim-bench` headless benchmark; state hashing

## M5: Playable sandbox (the playable baseline)

Decided 2026-10-17: baseline first (sandbox, scripted spawns, no AI),
then basic-AI skirmish, then data-driven addons, then roguelite mode.

- [x] Match the declared American setup with Allied generated bases and
      AMCV/CMIN/MTNK/E1 forces (#124). Validate required types, country
      eligibility and body art, honor the configured force count, and
      return setup errors instead of substituting Soviet types. Authored
      map objects keep their types. Deployment, Chrono Miner teleportation
      and original starting-force calibration remain future work.

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
- [x] Restore saved installed maps from native front/pause menus (#159).
      The version-2 wrapper carries a bounded map filename; successful loads
      commit the matching terrain, sprites and session together. Legacy
      matching-map saves and explicit CLI compatibility checks remain supported.
- [x] Delete saved matches from front/pause menus (#171, #96): selected-slot
      confirmation defaults to No; stale files and failed deletions refresh
      the list with visible recovery. The last deletion disables the action.
      Temporary-file handler checks and retail compositions preserve complete
      live save bytes and the quicksave target (`docs/formats/menus.md`).
- [x] Wrap front recovery messages by measured font width and word boundaries
      (#161). Explicit paragraph breaks survive; long tokens split at Unicode
      character boundaries. Scrollable overflow retains the full diagnostic
      and keeps Back/Escape available (`docs/formats/front-menu.md`).
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
- [x] In-match menu flow (#108): retail Allied pause plates and controls,
      named save/load with confirmation, restart/quit, actual mute/zoom settings,
      recoverable loading/errors and Won/Lost actions. Menus stop sim ticks
      without catch-up and isolate map input. Shared-compositor proof and
      source-pixel checks are described in `docs/formats/menus.md`. Exact
      oracle calibration, non-Allied skins and delete remain in #96.
- [x] Animated front menu and match entry (#110, #112): native retail Bink
      playback, shell/warning art, single-player navigation, authored map preview,
      supported EB1 setup, saved matches, actual options and data-folder action.
      Abort/results return to Main Menu. All 431 movie frames match independent
      YUV decoding; limited-range RGB conversion preserves contrast. Explicit
      CLI/headless launches bypass menus (`docs/formats/front-menu.md`).
- [x] Reload pause-menu art with the chosen game-data folder (#118), including
      font, labels and backgrounds. A retail render comparison covers the cache.
- [x] Calibrate static main/single-player shell output against local original
      renderer captures (#133): RGB565 presentation, button/text alignment,
      blank-row palette and bottom-cap placement. Thirteen captured regions
      and five complete enabled buttons match. Timing and broader menus remain
      in #96; captures and limits are in `docs/formats/front-menu.md`.
- [x] Match captured front animation pixels (#135, #136): warning anchor and
      26 warning phases, plus exact visible movie output at 32 original phases.
      Independent full-clip decoding remains checked. Fresh paired captures
      expose a clock-calibration gap (#174) and delayed-export defect (#176);
      transitions and movie/shell synchronization still need calibration (#96).
- [x] Retain renderer exports through helper exit and bounded settling (#176).
      Late/duplicate/partial files and live producers are explicit; nineteen
      synthetic regressions and an original holdout reconcile every source
      and copied hash. Request ownership remains unresolved.
- [x] Remove host scheduling races from export-observer tests (#178). Ordered
      synthetic events cover quiet/deadline and partial-write boundaries;
      real subprocess checks retain after-exit output and use release gates.
- [ ] Calibrate the front-menu warning clock (#174). Historical first-image
      fits have invalid temporal associations. The repaired observer retains
      multiplicity and uncertainty; presentation/export stalls, initial epoch
      and the scheduling rule still need calibration
      (`docs/research/front-warning-clock.md`).
- [x] Keep retail menu-load test waits consistent (#138): the in-match test
      uses the front-menu test's bounded 180-second budget for reconstruction
      while retaining all save/load, cancellation and state assertions.
- [x] Match captured front menu headings (#142, #143): final glyph color,
      authored Skirmish Game caption and compact-header baseline. Three native
      heading regions match original exports (`docs/formats/front-headings.md`).
- [x] Calibrate the Skirmish preview client and raster (#144, #151): measured
      145x113 black control, landscape/portrait image bounds and discrete
      nearest-neighbor sampling. Original crops check unoccluded pixels;
      numbered markers and modal heading state remain separate work.
- [x] Draw original numbered start-position markers in authored Skirmish
      previews (#152), using actual waypoints and retail sprite/font art.
      Seven original regions check two through eight starts, portrait and
      landscape rasters, letterbox composition and marker rounding. Generated
      terrain and chooser projection remain part of the full menu work (#96).
- [x] Match the original setup/chooser backdrops (#160): separate retail
      mountain and portrait canvases with their own palettes. Actual native
      composition matches 106,176 lower Skirmish pixels and 351,566 chooser
      body/control pixels outside the two unavailable game-mode rows
      (`docs/formats/front-backdrop.md`).
- [x] Native Battle map chooser (#96): the 91 active packet entries retain
      authored order and localized titles. Mouse/keyboard selection, scrolling,
      preview loading/errors, cancel, accept and selected-map match entry use
      actual app handlers. Five complete original preview regions match;
      Killer Instinct's second marker retains a one-pixel gap (#164).
- [x] Credits and starting-unit controls in the native Skirmish form (#96):
      bounded rule-derived limits, mouse dragging and keyboard navigation feed
      both participants' match setup (#166). Five complete original control
      regions match, including the numeric wells (#168). Save/load and
      restart retain choices, with explicit legacy setup reconstruction.
- [x] Use real computer harvester income when one is alive (#167), retaining
      the economy-lite fallback without a harvester. Ownership, reload and
      fallback are checked without a simulation schema change.
- [x] Calibrate numeric slider value-well backgrounds (#168) using the classic
      three-part retail skin. All 63720 RGBA samples match across initial,
      endpoint and midpoint controls without exclusions.
- [x] Supported two-participant setup controls within #96: player names,
      nine Allied/Soviet countries, colors, authored starts, opposing teams,
      Short Game and native speed presets. Choices feed side-specific forces,
      production/AI, sidebar art, radar/remaps, country loading and bounded
      version-4 save metadata. Default/legacy save fingerprints stay compatible.
- [ ] Complete broader Skirmish modes, Yuri, country special abilities and
      remaining game rules. Original starting-force composition, speed and
      timing calibration remain part of #96.
- [ ] Calibrate the chooser start projection without map-specific offsets
      (#164), retaining all exact Skirmish comparisons. Free For All, Team
      Alliance, random generation and the complete Skirmish form remain in #96.
- [x] Calibrate heading foreground after dialog refresh (#153). Focus loss,
      chooser selection and modal return match the original refreshed state;
      re-entry restores initial styling. Twelve complete header comparisons
      retain the separately verified Main/Single Player defaults.
- [x] Preserve guarded public Win32 window/control observations and actual
      child exit status in `tools/wine-oracle`, with synthetic geometry,
      private Edit exclusion, silent-exit and absent-target checks. Publish
      the complete fixture before it becomes visible (#150).
- [ ] Make original-game reference startup repeatable (#148). Installed
      RA2MD.exe reached a verified gamemd.exe menu and supplied fresh preview
      observations. The observer supports that bootstrap with a synthetic
      lineage check and preserves processes at its deadline. A stopped-state
      recheck still found no windows, with a Wine window-driver error. Keep
      the successful pixel evidence and the failing startup receipts.
- [ ] Native menu and match flow (#96): original-style main menu, skirmish
      setup/map preview, saved matches, pause/resume, save/load, restart,
      options and quit. Add Won/Lost screens, loading/error feedback and
      confirmation before discarding a match. Pausing stops sim ticks without
      catch-up on resume; modal input never reaches map commands. Preserve
      explicit CLI/headless launches and prove each menu state on its PR.
- [x] Improve native start-flow loading within #96: prepare the sprite bank
      on the worker, show map/save names and actual loading stages, and queue
      an immediate retry behind cancelled work. Cancel removes queued retries;
      stale updates cannot replace the active page. First Tab on Main Menu
      reaches Single Player. Retail handler captures and controlled worker
      regressions cover these native behaviors; physical desktop checks remain.
- [x] Use retail American country loading artwork and the selected map preview
      for initial match and saved-match loading (#199). Saved-map preview updates
      follow the active worker and cannot reuse a different map's thumbnail.
      Country-map source pixels, scaled cancel controls, missing-art recovery
      and stage captures are checked on Linux and macOS. Retail EB1 reaches
      the same tick-6000 hash on both platforms. macOS display capture remains
      permission-blocked. Exact original text, bar geometry and timing remain
      uncalibrated within #96.
- [x] Calibrate shared loading text and badge placement against a public original
      capture (#213): five complete glyph masks, 16638 samples and 3569
      foreground pixels match; 716 source-badge pixels match at `(117,323)`.
      The specialty title/full briefing, country tag and single local progress
      row replace guessed text and independent AI bars. Map sampling, backdrop
      details, progress fill and runtime timing remain unverified within #96.
- [x] Preserve exact HSV color components in loading/menu swatches and house
      remaps (#214), avoiding float cancellation across an RGB565 quantum.
      Keep the supported Skirmish countries in retail registry order (#215)
      without changing saved country enums or the simulation schema.
- [x] Account for terrain lift when centering world/radar camera targets (#129).
      Shared ground projection keeps targets at the world viewport center
      across supported scales/zooms and three layouts; click/box selection,
      sim/save invariance and retail before/after framing are checked.
- [x] Unit team shortcuts (#146, #97): Ctrl+1-9 assignment, surviving owned
      unit recall, Shift additive selection, double-press camera centering and
      visible feedback. Repeat/modal/focus guards and match replacement keep
      input isolated; grouping preserves match save bytes. Native policies and
      physical-input limits are in `docs/control-groups.md`.
- [x] Bound camera movement and add optional edge scrolling (#155, #97).
      Arrow/edge panning uses redraw time with equal diagonal speed; raster
      bounds account for zoom and sidebar width and center oversized views.
      Front/in-match Controls persist the toggle, defaulting off. Focus,
      modal and gesture guards prevent stale orders; complete match saves
      remain unchanged. Native policies and physical-input limits are in
      `docs/camera-input.md`; original bounds/cursor calibration remains open.
- [x] Native tactical order acknowledgements (#97): shared-resolver move,
      attack-move and attack destination markers; Stop/Harvest key feedback
      with eligible unit counts, unavailable-action messages, fixed UI-pixel
      geometry, target tracking and two-second expiry. Focus/modal/selection
      cleanup and complete save-byte/sim-hash invariance are checked in
      `docs/order-feedback.md`. Original appearance/timing, service
      acknowledgements and physical desktop input remain open.
- [x] Reject pointer input in physical window margins (#223, #97): failed
      framebuffer mappings, focus loss, resize/DPI changes and enter/leave
      cancel stale gestures. Buttons and wheels require fresh valid motion;
      menu-to-match panning retains a local delta. Linux live before/after
      checks at 1600x1000 and 1920x1200 verify ignored margin wheels and working
      in-frame wheels; retail selection and complete save bytes are checked.
      Physical Mac Retina/input checks remain open (`docs/window-input.md`).
- [x] Keep compact clients usable (#224): bounded logical framebuffers shrink
      with the window down to the requested 640x480 logical minimum. Drawing,
      menu/slider hits, sidebar and world camera use the same dimensions.
      Retail start/loading, selection, pause/resume, save-byte invariance and
      Linux live before/after captures cover 640x480 and 1000x700; map zoom is
      preserved. Physical Retina delivery remains open under #97.
- [x] Preview selectable and attackable world targets (#97): green/red native
      corner brackets use the existing cursor/click resolver and current entity
      ground anchors. Focus/menu/gesture/command-mode guards, Shift routing,
      compact layouts, unavailable attackers and complete save-byte invariance
      are checked. Original appearance, persistent ordered-target highlights
      and physical Retina input remain open (`docs/order-feedback.md`).
- [x] Acknowledge powered-radar orders (#97): move/attack-move requests reuse
      native world-order text and snapped destination markers without changing
      commands, selection, zoom or order-time camera position. Navigation and
      cancellation clear the notice. Retail pointer routes, three compact/default
      layouts, three zooms, disabled/modal/focus guards and complete snapshot
      equivalence are checked (`docs/order-feedback.md`). Original calibration
      and physical Mac/Retina input remain open.
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
