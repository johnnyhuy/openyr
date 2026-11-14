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

## M0: Research and repo (baseline done)

- [x] Restore retail-backed presentation and setup regressions (#369, #370, #371).
      Exclude fractional live-unit coverage from the building-only pixel mask;
      retain real rear/front overlap, saved-flight and trail checks. Pin the
      bounded infantry death house ramp while checking source bodies/shadows.
      Explain
      the corrected EB1 initialization hash with a historical-state comparison
      and retain complete legacy-profile save bytes through native loading
      (`docs/formats/effects.md`, `docs/formats/skirmish-setup.md`).

- Precedent study: `docs/research/`
- Repo, licence, method: `docs/legal.md`
- Wine testbed retained as reference oracle at `~/Games/RA2YR`
- [x] Keep the required macOS CI job on an explicit standard Intel runner
      (#184), retaining all checks after hosted arm64 runner acquisition
      failed before any steps ran. Local retail/window verification also
      exercises Apple Silicon.

- [x] Retain retail-art assertions under stable Clippy 1.100 (#374). Bounded
      iterator loops preserve both rotor matrices, all cursor/control/clock/power
      frame counts, literal values and source-pixel checks. Runtime rendering and
      the required warning policy stay unchanged.

- [x] Guard runtime proof against shared-target executable reuse (#238).
      A bounded Cargo/worktree fixture reproduces stale output. Isolated builds
      record source tree, Cargo command and executable size/SHA256; verified
      runs reject changed source or executables (`docs/runtime-proof.md`).

- [x] Use mise for project Rust/Python tools, local development tasks and CI;
      retain `rust-toolchain.toml` as the Rust toolchain source of truth.
      Showcase in-game features with Markdown tables, screenshots and looping
      GIFs (#251), hosted separately from source, and explain engine/content
      licensing and the clean-room method.

- [x] Build a classic command-console GitHub Pages showcase (#261) with existing
      game captures, source downloads, macOS build orders and legal context.
      Site chrome is original CSS; media remains outside main (`docs/website.md`).

- [x] Put battlefield gameplay before menus in README tables (#339), with
      verified combat, infantry movement and base-operation GIFs from earlier
      native captures. Pin media outside main and record source versions,
      conversion hashes and sampled-tick limits (`docs/demos.md`).
- [x] Publish a fresh retail-backed gameplay recording and README showcase
      (#339, #372). The isolated native EB1 export retains every tick from
      2-3000 with synchronized audio, scripted production/placement/attack orders,
      full MP4 and short gameplay GIFs. Final-build sample exports match exactly;
      README links pin verified media outside main. This is shared-compositor
      headless playback, not interactive desktop capture, original timing or
      physical input evidence (`docs/demos.md`).

## M1: Asset layer (done 2026-10-17; open items carried forward)

- [x] MIX reader: classic + flagged headers, SHA1 trailer, encrypted (RSA/Blowfish) headers
- [x] Reject truncated unencrypted flagged MIX headers before reading fields
      (#365). All short prefixes of authored zero/checksum-flag archives return
      errors without panic; complete payloads remain intact (`docs/formats/mix.md`).
- [x] Filename hash (TS/CRC32 variant) with known-answer tests
- [x] CSF reader (labels, UTF-16 inverted strings)
- [x] Bound multi-value CSF label storage (#401). Shared immutable names prevent
      multiplicative allocation; checked table/local counts and byte ranges
      reject hostile declarations. Authored parser, CLI and native localization
      checks preserve Unicode, null/sound records and first-value lookup
      (`docs/formats/csf.md`). Retail known-answer checks retain 5,211 entries.
- [x] INI reader (sections, keys, comments, order-preserving)
- [x] `mix-dump` CLI (list/extract/info/names)
- [x] Bound composed SHP/TMP inspection rasters (#397). Wide TMP coordinates
      preserve small extreme translations; checked PNG dimensions, a 1 GiB
      budget and fallible reservation prevent unbounded sheet allocations.
      Actual CLI errors preserve existing/absent destinations, while authored
      translated/extra/empty-sheet controls retain their pixels. Native
      rendering, format acceptance, simulation and saves are unchanged
      (`docs/formats/tmp.md`, `docs/formats/shp.md`). Retail checks need data.
- [x] Exit cleanly when an inspection output pipe closes early (#79).
      All `mix-dump` commands propagate stdout failures; BrokenPipe exits
      successfully without a panic. Input errors and other output failures
      keep a failing exit code. Synthetic CLI tests run without retail assets.
- [x] PAL / colour tables (6-bit RGB palettes)
- [x] SHP reader (cameos, buildings, infantry sprites): every retail frame decodes
- [x] Reject malformed SHP zero-run rows (#375), including missing counts,
      underfilled rows, nontransparent overflow and runs beyond retail padding.
      Preserve the observed one-pixel transparent row overrun; authored
      frame/canvas and CLI checks retain valid neighbors (`docs/formats/shp.md`).
- [x] Reject WSA packets overlapping metadata and empty explicit loop packets
      (#394). Authored initial/chained, palette and loop fixtures retain valid
      ranges and reject every truncated prefix (`docs/formats/wsa.md`). Retail
      WSA playback remains unverified because the install contains no WSA files.
- [x] VXL + HVA readers (vehicles)
- [x] Reject VXL columns outside their declared byte/height bounds (#377).
      Validate complete offset tables and constrain every column read before
      decoding; preserve exact-height empty runs and valid sparse/full columns.
      Authored parser/CLI regressions and the 16-archive retail census pass with
      zero errors (`docs/formats/vxl.md`). Voxel lighting and original animation
      cadence remain separate fidelity work.
- [x] TMP terrain tiles
- [x] Reject incompatible TMP diamond geometry (#386): four-pixel row
      validation prevents accepted 2x1 templates from panicking in rendering.
      Public row generation rejects invalid dimensions before allocation;
      valid TS/RA2 geometry, CLI output preservation and terrain-loader errors
      are checked (`docs/formats/tmp.md`). Simulation/save formats stay intact.
- [x] `mix-dump scan`: recursive census + corpus-wide parser check
- [x] Fail the corpus check when the census contains parser errors (#111,
      #113), including malformed CSF files. Print the census and first 40
      diagnostics before failing; intentionally closed output pipes still
      exit successfully. Synthetic CLI tests cover malformed archives,
      nested assets and the uncapped error total.
- [x] Use an actual closed pipe for CLI output tests (#202), avoiding an
      intermittent macOS socket fixture failure in the required workspace suite.
- [x] PCX reader (8-bit paletted + 24-bit planar; all 271 retail files)
- [x] Reject PCX RLE runs exceeding the declared complete image (#367),
      retaining padding and cross-row/plane runs. Authored boundary and native
      loading-flag rejection/recovery checks preserve valid neighbor pixels;
      retail corpus revalidation remains unavailable without an install
      (`docs/formats/pcx.md`).
- [x] AUD / AUDIO.BAG+IDX / IMA WAV decoders, bit-exact vs ffmpeg
- [x] Reject unrepresentable canonical WAV exports (#390). Fallible encoding
      checks complete mono/stereo frames and RIFF/rate/platform limits before
      allocation. CLI rejection preserves existing output; ordinary PCM/IMA
      and native mixer capture bytes remain unchanged (`docs/formats/audio.md`).
- [x] Isolate map-extent test fixtures when wall-clock names collide (#392).
      Atomically claim bounded scratch candidates and acquire cleanup ownership
      only after creation, preserving preexisting and concurrent test files.
- [x] Reject malformed WAV container and PCM geometry (#387): bound chunks
      by RIFF size, require complete headers/padding and validate mono/stereo
      frame alignment and rates before decoding. Authored parser/CLI/native
      lookup checks preserve existing exports, valid neighbors, bank fallback
      and IMA fact trimming. All 1,223 retail WAVs still pass the corpus scan
      (`docs/formats/audio.md`).
- [x] Bink 1 container metadata (#110): all 146 retail Bink entries validate
      with bounded frame/track tables and packet ranges (`docs/formats/bink.md`).
- [x] WSA metadata: bounded header, offset-table and frame-range parser
      (`yr-formats::WsaFile`), routed in `mix-dump scan` through recovered
      .WSA names. The retail census holds no WSA assets, so synthetic
      known-answer and hostile-header fixtures carry the checks
      (`docs/formats/wsa.md`). LCW/XOR-delta decoding remains future work.
- [x] `yr-assets` VFS: engine-order MIX cascade, nested mounts, theaters, loose overrides
- [x] Reject ambiguous physical loose filenames (#360) instead of letting
      directory traversal order choose rules/art. Actual scratch installs check
      both case-variant creation orders, unchanged source bytes, unique lookup
      and existing empty/non-regular filtering. Explicit ordered in-memory
      fixtures, simulation and save formats retain their policies
      (`docs/formats/mix.md`).
- [x] Name the seven reported Yuri side-archive entries and three Allied menu
      background plates (#311, #310). Literal entry-ID checks and authored MIX
      CLI proof verify resolution; supplied retail PNG sizes/hashes are checked
      separately. The optional retail frame/palette test needs a local install.
      Side mounting, Yuri gameplay and original presentation remain open
      (`docs/formats/menus.md`, `docs/formats/mix.md`).
- [x] Resolve individual loose sidebar files before the selected Allied/Soviet
      archive (#86), keeping shared archive namespaces isolated. Authored native
      source-color crops, malformed/hostile optional-art recovery and exact
      Session-byte checks cover both side geometries (`docs/formats/sidebar.md`).
      Global fonts and ordinary VFS lookup are unchanged.
- [x] Retain selected-side tab geometry when optional art fails (#358).
      Drawing, native fallback and hit boxes share fixed Allied/Soviet dimensions;
      malformed first-tab and palette fixtures preserve valid neighbor pixels
      and all four tab boundaries (`docs/formats/sidebar.md`).
- [ ] Side-specific archives (SIDEC##) and remaining original per-file loader exceptions
- [x] MIX name database: every nested archive named; ~88% of all entries named
      via INI-derived candidates with content-kind collision rejection

## M2: Typed game data

- [x] Reject overflowing exact decimal fractions (#406). Signed checked
      accumulation retains six-digit truncation, percentage scaling and both
      i64 boundaries. Wide lepton conversion keeps valid extremes printable;
      authored parser/CLI checks cover debug and release without retail data
      (`docs/formats/rules.md`).
- [ ] Bound public Decimal integer construction and multiplication (#408),
      including representable products with overflowing intermediates and an
      explicit policy for unrepresentable results.

- [x] Parse `rulesmd.ini`/`artmd.ini` into typed technos/weapons/warheads/projectiles (`yr-rules`)
- [x] `mix-dump rules <gamedir> <ID>`: resolved unit, weapon chain, damage table, art provenance
- [x] INI fidelity: header text after `]` (MirageWH), first-char booleans, map merge
- [x] `[General]`/`[CombatDamage]` globals, houses/countries, skirmish defaults
- [x] Superweapons, `soundmd.ini` (typed data): 12 retail superweapons
      register from `[SuperWeaponTypes]` plus building `SuperWeapon=` grants,
      and 819 sound events plus `[Defaults]` load through `SoundData`.
      Charging, firing, effects and playback wiring remain future work
      (`docs/formats/rules.md`).
- [ ] Engine defaults per key, verified against the Wine oracle
- [ ] Name database: close the remaining ~12% (RA2-only VXLs, PCX, audio)

## M3: Renderer v1

- [x] Keep supported map-unit art synchronized with live state (#351): shared
      bounded art preflight omits reconstructable bodies from match backgrounds
      while previews and unsupported fallback stay static. Transient unit-id/
      kind identity retains map-house colors through movement and save/load;
      native SHP/VXL body/coverage, unioned shadows, ownership and destruction
      checks preserve encoded matches and setup keys. Original/retail depth,
      model orientation, animation cadence and physical checks remain open
      (`docs/formats/map.md`, `docs/formats/shp.md`).

- [x] Preserve native source detail by default (#101, #103): scale 1 retains
      60x30 TMP cells and full-size unit art at zoom 1. Explicit inspection
      scales are limited to 1/2/4. Matched-magnification retail comparisons
      expose the old half-resolution terrain loss (`docs/formats/effects.md`).
- [x] Retail muzzle and vehicle destruction effects (#102): actual firing
      and death events route type/weapon art through ANIM.PAL, bounded caches
      and one-shot presentation clocks. All 78 source frames, colored alpha,
      same-tick fire/death and unchanged sim/save data are checked.
- [x] Initial damaged-vehicle smoke (#405, #102): explicit retail damage
      systems resolve bounded particle frames and translucency. Exact health
      gates, fixed-world puffs, emission/expiry caps, native composition and
      resumed future emission after load preserve complete Session bytes.
      All 20 source canvases match; original timing, drift, emitter offsets,
      sparks, buildings and aircraft remain open (`docs/formats/effects.md`).
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
- [x] Reduce unit color contrast and coarse voxel sampling (#342): bounded
      house ramps avoid clipped highlights, softer provisional lighting keeps
      dark faces readable, and subpixel voxel coverage preserves fractional
      edges without whole-frame blur. Authored house/facing/depth/anchor checks
      cover map/live agreement and shadow isolation; original/retail calibration
      remains open (`docs/formats/vxl.md`).
- [x] Animate pre-placed neutral building idle layers (#341, #6): omit
      reconstructable SHP structures from match backgrounds and reuse live
      body/bib/active layers, original map-house colors and existing foundation/
      shadow ordering. Static previews and unsupported art stay baked. Authored
      background/idle/destruction checks preserve complete Session bytes,
      selected units and pending orders; original/retail calibration remains open.
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
- [x] Add classic Q/W/E/R sidebar category shortcuts (#323, #86, #97):
      deliberate match input shares mouse-tab category/row state, preserves
      selection, READY work, pending orders, camera and Session bytes, and
      cancels stale gestures and placement/service modes. Authored full-frame
      mouse/key comparisons, guard cases and compact scroll restoration pass.
      The manual establishes the mappings; original transitions and physical
      Mac/Retina checks remain open (`docs/formats/sidebar.md`).
- [x] Draw authored sidebar category press states (#86): held mouse input
      selects the pre-press inactive/selected sheet variant, tracks origin-tab
      hover and clears on release or pointer/focus/modal cancellation. Native
      Allied/Soviet fixture crops and fallback recovery preserve encoded
      Sessions, selection, camera and queued orders. Original transition
      calibration and physical/retail checks remain open
      (`docs/formats/sidebar.md`).
- [x] Keep held sidebar tabs outside world pointer actions (#356, #86,
      #97, #98): origin-tab ownership suppresses world action/hover previews,
      edge scrolling and wheel actions until release or cancellation. Native
      authored regression checks reproduce the old feedback/edge leakage,
      preserve Session bytes and queued intents, and verify fresh click/pan/
      zoom recovery (`docs/window-input.md`). Original and physical calibration
      remain open.
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
- [x] Harvest cursor feedback (#98): retail gold chevron frames 90 to 98
      map resource cells under selected harvesters to a harvest state, and
      the shared resolver issues the harvest order on click while other
      selected units keep their move order. Source pixels, resolver
      filtering and the named click acknowledgement are checked
      (`docs/formats/cursors.md`). The frame assignment is provisional
      pending a controlled original-game capture.
- [x] Align Shift-only cursor feedback with additive selection (#290, #98,
      #97). Shared resolution retains Select/Pointer without tactical orders,
      including when A is armed; normal and Ctrl+Shift orders remain intact.
      Synthetic Sim checks cover 70 target/selection/mode cases and unchanged
      save bytes/hash (`docs/formats/cursors.md`). Retail/physical Mac checks
      and original modifier calibration remain open.
- [ ] Contextual animated game cursor parity (#98): observed retail frames,
      palettes and hotspots for selection, movement, attack, harvesting,
      placement and other supported commands. Match actual command validation,
      keep size independent of map zoom, handle Retina coordinates and restore
      the system pointer on leave/focus loss/native dialogs. Include cursor
      states in shared headless proof and compare original-game behavior.

## M4: Simulation v1

- [x] Preserve authored initial map-unit facing (#350): shared bounded byte
      normalization and a scenario heading command cover vehicle, infantry and
      aircraft setup without changing ordinary spawns/production. Static/live
      SHP Ready/shadow and VXL direction/anchor checks use authored art. Old
      unaffected keys survive; affected old matches reject without rewriting
      snapshots (`docs/formats/map.md`, `docs/formats/save.md`). Native direction
      consistency is verified; original/retail compass calibration remains open.

- [x] Apply initial map-unit health fractions (#349) to vehicles, infantry
      and aircraft without reducing their type maximum. Shared bounded integer
      initialization, all three parsed sections, native pip/Attack feedback,
      damage lethality and exact save/load are checked with authored fixtures.
      Normal starting forces/production keep their path; unaffected setup keys
      remain compatible while affected old matches reject without rewriting
      (`docs/formats/map.md`, `docs/formats/save.md`). Retail/original and
      physical calibration remain open.

- [x] Preserve full maximum health for pre-placed damaged map structures
      (#346): bounded map fractions affect current HP only. Authored setup,
      damaged-art predicate, native health pips, generator/repair and save/load
      checks cover full/half/low/zero/overfull health. Unaffected old matches
      retain their setup key; affected old matches reject without rewriting
      saved HP (`docs/formats/map.md`, `docs/formats/save.md`). Retail and
      original malformed-input calibration remain unverified.

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
- [x] Select EVA speech from the local player's country (#398): supported
      Allied/Soviet matches use the retail `Allied`/`Russian` keys for fresh
      and disk-restored sessions. Authored PCM and actual retail infantry
      production captures retain effects, CSF fallback and exact Session/save
      bytes. Physical output and original timing remain uncalibrated
      (`docs/formats/audio.md`).
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
- [x] Preserve occupied files when remembering the game-data folder (#362,
      #96): share the interface preference writer's bounded exclusive creation,
      synced replacement and owned cleanup. Actual scratch files verify occupied
      files, Unix symlink targets, failed publication and retry without changing
      discovery order or JSON. Native macOS chooser delivery remains unverified
      (`docs/macos-app.md`).
- [x] Validate newly chosen game-data folders through the actual VFS (#364,
      #96), sharing validation between initial discovery and the menu action.
      Required regular loose archives must be nonempty and parse; ambiguous
      loose names and symlink-only archives reject before remembering. Scratch
      files verify unchanged preferences/source bytes and valid mixed-case retry.
      Native chooser delivery and complete retail content remain unverified
      (`docs/macos-app.md`).
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
- [x] Make original-game reference startup repeatable (#148). Two
      consecutive GUI-attached launches each verified the gamemd.exe Main
      Menu with matching launcher parent PIDs, and the Skirmish preview
      control re-measured 145x113 at `(644,37)`. Shells without GUI access
      or wineserver socket permission prove nothing about the prefix. Keep
      the successful pixel evidence and the failing startup receipts
      separate.
- [x] Reject Alt/Option-modified menu Enter (#384, #96, #97): Enter and
      keypad Enter cannot activate focused confirmations or other controls.
      Protected overwrite/delete files, pending loads and repeat recovery pass
      through native handlers; ordinary Enter and Option text remain usable
      (`docs/window-input.md`, `docs/formats/menus.md`). Physical/original
      modifier calibration remains open.
- [ ] Native menu and match flow (#96): original-style main menu, skirmish
      setup/map preview, saved matches, pause/resume, save/load, restart,
      options and quit. Add Won/Lost screens, loading/error feedback and
      confirmation before discarding a match. Pausing stops sim ticks without
      catch-up on resume; modal input never reaches map commands. Preserve
      explicit CLI/headless launches and prove each menu state on its PR.
- [x] Bound theater tile-index expansion before loading (#292, #96).
      Checked count/storage budgets and fallible parsing reject oversized or
      malformed metadata without truncating tile numbering. Synthetic parser,
      terrain/radar/map-loader and CLI proof cover recovery
      (`docs/formats/map.md`); retail scans remain unavailable.
- [x] Replace missing/malformed TMP panics in terrain setup with contextual
      loader errors (#293), preserving ordinary sparse-grid behavior. In-memory
      fixtures verify valid metadata, clear-sentinel/cache reuse and complete
      synthetic Sim save-byte/hash equivalence (`docs/formats/tmp.md`). Retail
      scans and physical Mac checks remain unavailable.
- [x] Bound map-coordinate-derived terrain/raster extents before allocation
      (#294). Declared tile rectangles use a native 262,144-cell budget; full
      rasters use checked wide bounds and a 1 GiB pre-downscale budget. Parser,
      programmatic terrain/radar and actual tiny raster tests preserve sparse
      behavior and an independently specified sparse Sim save
      (`docs/formats/map.md`). Large-map compatibility remains unverified;
      retail/physical Mac checks remain open.

- [x] Cancel pending loads on asynchronous menu movie failure (#288, #96).
      Normal modal entry clears input, cancels work and drops queued retries;
      late success/error/progress cannot replace recovery. Actual synthetic
      decoder failures, load channels and fallback frames verify three sizes
      (`docs/formats/bink.md`); retail/physical Mac checks remain open.
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
      `docs/order-feedback.md`. Original appearance/timing and physical
      desktop input remain open.
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
- [x] Remove the white selected-unit outline (#343): selected units draw no
      white corner overlay; health, hover/target previews, drag selection and
      command semantics stay unchanged.
- [x] Revise native unit status toward classic UI (#312, #97): selected units
      share corner-bracket geometry with hover/attack previews; bordered health
      pips replace the flat stripe, clip to the world viewport and remain empty
      at zero health. Three-layout/three-zoom compositor and cursor/cancel checks
      preserve encoded Session bytes. Exact original geometry/colors/visibility,
      retail sprite overlap and physical Mac/Retina checks remain open
      (`docs/order-feedback.md`, #97/#98). The selected white outline was
      subsequently removed in #343 at the user's request.
- [x] Preview selectable and attackable world targets (#97): green/red native
      corner brackets use the existing cursor/click resolver and current entity
      ground anchors. Focus/menu/gesture/command-mode guards, Shift routing,
      compact layouts, unavailable attackers and complete save-byte invariance
      are checked. Original appearance and physical Retina input remain open (`docs/order-feedback.md`).
- [x] Acknowledge powered-radar orders (#97): move/attack-move requests reuse
      native world-order text and snapped destination markers without changing
      commands, selection, zoom or order-time camera position. Navigation and
      cancellation clear the notice. Retail pointer routes, three compact/default
      layouts, three zooms, disabled/modal/focus guards and complete snapshot
      equivalence are checked (`docs/order-feedback.md`). Original calibration
      and physical Mac/Retina input remain open.
- [x] Acknowledge repair/sell requests (#97): the existing service resolver
      drives repair, active-job cancellation, sale and unavailable-target
      notices. World/sidebar mode cancellation clears feedback while retaining
      queued commands and selection. Retail pointer routes at three layouts
      and zooms preserve command/save behavior; equivalent stepped snapshots
      match. Appearance/timing and physical Mac/Retina input remain unverified
      (`docs/order-feedback.md`, `docs/formats/structure-services.md`).
- [x] Acknowledge structure-placement requests (#97): queued requests show
      native text and footprint-center markers; invalid clicks retain READY
      state and placement mode. Cancellation, stale modes and pointer guards
      preserve queued work. Retail cameo/world routes at three layouts and
      zooms retain command/save behavior and equivalent stepped snapshots.
      Original appearance/timing and physical Mac/Retina input remain open
      (`docs/order-feedback.md`, `docs/formats/rules.md`).
- [x] Acknowledge sidebar production requests (#97): queued builds, holds,
      resumes, cancellations and occupied-queue clicks show native text through
      the shared two-second notice, and READY pickup announces placement.
      Retail cameo routes preserve queued commands and complete pre-step save
      bytes. Original appearance and timing remain open (`docs/order-feedback.md`).
- [x] Show selected units' current attack targets (#97): deduplicated native
      brackets follow living hostile targets from existing sim state, independent
      of pointer hover or brief notices. Selection/context/removal/restoration,
      three layouts/zooms, position updates and complete snapshot invariance are
      checked. No commands, combat or save fields change. Original appearance
      and physical Mac/Retina input remain open (`docs/order-feedback.md`).
- [x] Preview repair and sell targets (#97): green or red corner brackets
      follow the shared service resolver in repair/sell modes, covering
      full-health, damaged and enemy buildings at three layouts and zooms.
      Pointer-only checks keep complete save bytes unchanged, via
      [docs/order-feedback.md](order-feedback.md). Original appearance and
      physical Mac/Retina input remain open.
- [x] Keep pause-menu recovery diagnostics readable (#234, #96, #97): measured
      word wrapping preserves paragraphs and long tokens without the prior
      400-character cutoff. Shared input/draw geometry supports keyboard and
      wheel scrolling at compact/default sizes while Back/Escape, frozen ticks
      and complete saved-match bytes remain intact (`docs/formats/menus.md`).
      Original layout calibration and physical Mac/Retina checks remain open.
- [x] Clear command modes when F5 opens a save result (#237, #96, #97):
      success and failure share ordinary menu input cleanup, cancelling modes,
      modifiers, gestures and transient recall feedback. Selection, assigned
      teams, queued orders and full saved-match bytes remain intact; recovery
      and stale releases are checked (`docs/formats/menus.md`).
- [x] Confirm successful saved-match menu loads (#97, #96): the world feedback
      strip shows the restored tick and bounded save name after commit. Failures,
      cancellation, restart and fresh setup do not announce a saved load. Retail
      async handler/compositor checks preserve complete save bytes at three sizes
      (`docs/formats/menus.md`); original text/timing and physical input remain open.
- [x] Keep long Save name edits visible (#243, #96, #97): measured suffix
      fitting retains the caret and final typed characters at compact widths,
      with a hidden-prefix indicator. Unicode boundaries, input limits and full
      output filenames remain intact; retail/fallback pixels and handler save
      bytes are checked (`docs/formats/menus.md`).
- [x] Handle Save/player-name shortcuts without inserting shortcut text
      (#248, #96, #97): Ctrl/Command+A selection, replacement, selected Backspace,
      visible Save selection and modifier/focus cleanup. Actual handlers,
      Unicode/input bounds, retail/fallback editor pixels and complete saved
      match bytes are checked. Clipboard and physical Mac input remain outside
      this native editing slice (`docs/formats/menus.md`).
- [x] Identify target files in Save/Load/Delete confirmations (#253, #96, #97).
      Quoted basenames retain long suffixes, escaped controls and spaces within
      measured, scrollable text regions in both skins. Button defaults and
      navigation, cancellation, complete retail match/save bytes and existing
      backend validation remain intact (`docs/formats/menus.md`). Original dialog
      calibration and physical Mac/Retina input remain open.
- [x] Ignore focus-synchronization key presses (#255, #97, #96): synthesized
      and unfocused presses cannot activate menus or world commands. Suppress
      follow-on repeats until release or a fresh focused press; releases still
      clear held input through modal/focus changes. Asset-independent native
      dispatch tests verify temporary-save preservation, deliberate deletion,
      name/navigation guards and normal shortcuts (`docs/window-input.md`).
      Physical Mac/Retina delivery remains unverified.
- [x] Save filesystem-valid long external filenames (#257, #96, #97): bounded
      exclusive sibling names no longer append to or lossily expand the target.
      Actual-file regressions cover ASCII/Unicode/Linux byte names, occupied
      candidates, symlinks and failed-rename cleanup without changing encoded
      save bytes or simulation. Retail reconstruction and physical Mac checks
      remain outside this storage slice (`docs/formats/save.md`).
- [x] Recognize right drags made of small pointer steps (#259, #97): activate
      from cumulative press displacement and include the threshold-crossing
      motion once, retaining incremental panning, bounds and click jitter.
      Handler regressions cover 27 size/zoom/direction cases, out-and-back
      motion and stale releases. Synthetic shared-compositor frames show the
      camera shift without retail data (`docs/camera-input.md`); physical input
      and original threshold calibration remain open.
- [x] Cancel unfinished mouse gestures on right press (#263, #97): clear
      stale left/radar gestures before match routes and redraw the rubber band.
      An active world right-click/pan rejects conflicting left presses; queued
      commands and existing right-click policies remain intact. Handler checks
      cover layouts, routes and release order; synthetic compositor frames
      verify removal without retail data (`docs/window-input.md`). Original
      button-chord calibration and physical Mac/Retina input remain open.
- [x] Align world cursor/hover feedback with gesture ownership (#298, #98,
      #97): nonzero net held arrows and tracked right presses suppress action
      previews before redraw, including service and placement modes. Native
      synthetic Sessions verify 108 arrow/layout/action combinations, real
      queued commands, ready structures, opposing arrows, key-up recovery and
      unchanged encoded saves. Retail, original and physical Mac checks remain.
- [x] Reject new world gestures during held-key camera panning (#286, #97).
      Shared net direction drives redraw and input ownership; opposing keys,
      key-up recovery, sidebar and menu routes retain their behavior. Native
      handlers/compositor verify 60 arrow/layout/mode cases without retail data
      (`docs/camera-input.md`); physical Mac and original parity remain open.
- [x] Cancel pointer gestures at camera key-down before redraw (#284, #97).
      Native dispatch/compositor regressions cover 48 arrow/gesture/layout
      cases, stale releases and fresh post-key-up boxes. Selection, queued
      intents, command modes and camera timing remain intact
      (`docs/camera-input.md`); retail/physical Mac checks remain open.
- [x] Clear complete pointer tracking on repair/sell mode changes (#282, #97),
      including stale pan flags and edge arming. Native handlers and fallback
      composer frames verify 72 mode/gesture/layout combinations, inert old
      releases and visible fresh box drags (`docs/window-input.md`). Selection
      and queued intents remain; retail/physical Mac checks are still open.
- [x] Pick units at their exact drawn subcell anchors (#280, #97), replacing
      cell-center snapping while keeping height projection and viewport/owner
      guards. Synthetic Sim checks cover 216 layout/scale/zoom/height/position
      cases and eight movement ticks with unchanged post-tick snapshot bytes
      and hashes (`docs/window-input.md`). Retail/physical Mac checks remain open.
- [x] Bound box selection to the visible world viewport (#278, #97): share
      clipped picking/painting geometry, exclude sidebar-hidden anchors and
      keep ordinary owner/projection behavior. Native compositor pixels and
      synthetic Sim picks at three sizes/zooms preserve full snapshot bytes
      and hashes (`docs/window-input.md`). Retail/physical Mac checks remain open.
- [x] Make native attack-move key activation exclusive and repeat-safe (#276,
      #97): cancel unfinished pointer and placement/service modes before arming;
      OS repeats cannot re-arm after an order or cancellation. Asset-independent
      handlers/compositor checks preserve selection and queued intents, with
      fresh actions and focus/menu guards intact (`docs/order-feedback.md`).
      Retail simulation and physical Mac checks remain open.
- [x] Replace an armed attack-move action when picking up a ready structure
      (#302, #97, #86). Actual sidebar and world handlers verify 27 layout/zoom/
      outcome cases, placement/cancellation recovery, fresh A activation and
      ordinary production, preserving full encoded saves and prior intents.
      Synthetic compositor proof is asset-independent; original mode parity,
      retail reconstruction and physical Mac/Retina input remain unverified.
- [x] Cancel menu pointer tracking on deliberate keyboard input (#274, #96,
      #97): discard held buttons, Skirmish slider and map-thumb drags before
      normal key routing. Actual temporary-file/handler checks prevent stale
      confirmation deletion and preserve fresh actions, popups and focus guards
      at three sizes (`docs/window-input.md`). Physical Mac and retail checks
      remain open.
- [x] Page through native Load/Delete save lists with Page Up/Page Down and
      Home/End (#270, #96, #97). Keep selection visible, bound endpoint arithmetic
      and preserve exact confirmation paths and file bytes on cancellation.
      Native dispatch/fallback frames cover compact/default shells; retail
      reconstruction and physical Mac input remain open (`docs/formats/menus.md`).
- [x] Accumulate high-resolution menu wheel input (#268, #96, #97): retain
      fractional pixel/line motion and multi-row magnitude, bound large events
      and discard partial gestures across page/input/focus changes. Dispatch
      and fallback compositor checks preserve selection and selected Save paths
      (`docs/window-input.md`). Retail and physical Mac checks remain open.
- [x] Preserve the selected Save file's actual identity (#265, #96): unchanged
      listed paths survive truncated/lossy labels and overwrite decline. Effective
      edits create a new-name request; stale, unlisted or out-of-folder selected
      targets recover without falling back to a different sibling. Actual-file
      handler fixtures check exact confirmation/Commit intents and protected
      sibling bytes without a Session (`docs/formats/menus.md`). Retail overwrite
      and physical Mac/Retina checks remain unverified; save encoding is unchanged.
- [x] Preserve occupied files and symlink targets when saving interface preferences
      (#299, #96): bounded exclusive sibling creation, synced replacement and
      owned cleanup retain the existing JSON schema. Actual native handler and
      scratch-file checks cover publication errors, Controls recovery and retry;
      physical Mac input remains unverified (`docs/formats/menus.md`).
- [x] Apply normal modal input cleanup to live preference-write recovery (#303,
      #96, #97). Actual M failures and recovery at three sizes cancel old modes
      and gestures while preserving queued orders, selected units, assigned
      teams and encoded Sessions. Successful mute remains nonmodal; authored
      compositor proof is asset-free (`docs/formats/menus.md`).
- [x] Protect an earlier named save when a fresh front-menu match commits
      (#306, #96). The new match targets quicksave in the current save folder;
      actual F5/F9 dispatch preserves old named bytes and writes exact fresh
      Session bytes. Controlled result channels cover load/restart/preview/
      failure/cancel policies at supported layouts (`docs/formats/menus.md`).
      Retail reconstruction and physical Mac/Retina checks remain open.
- [x] Keep saved matches beyond the first 64 reachable (#308, #96). Retain all
      regular saves found in the existing bounded directory scan; native
      Save/Load/Delete handlers verify a newly written 65th slot, exact paths,
      both skins and three layouts while preserving file and Session bytes.
      The 4,096-entry scan and no-follow filters remain; synthetic proof does
      not establish retail reconstruction (`docs/formats/menus.md`).
- [x] Suppress held-key repeats across modal boundaries (#314, #96, #97):
      track physical presses separately from world keys; menu/popup transitions,
      focus loss, native folder selection and prepared loads require release or
      a fresh press before an old repeat can edit, pan or zoom. Native Save,
      dropdown and 24 resume cases at three layouts preserve existing selection,
      command and Session/save policies. Same-editor repeats remain available;
      retail reconstruction and physical Mac/Retina checks remain unverified
      (`docs/formats/menus.md`).
- [x] Add visible-unit type expansion on T (#316, #97): use the shared drawn
      ground bounds to add living owned matching kinds while retaining owned
      seeds, excluding sidebar-hidden candidates and pruning invalid IDs.
      Native count feedback, mode/gesture cleanup, input guards and unchanged
      camera, queued orders and Session bytes are checked across layouts/zooms.
      The manual establishes T's mapping; detailed type-selection scope and
      repeated-press behavior remain native policies pending original comparison
      (`docs/control-groups.md`). Retail/physical checks remain unverified.
- [x] Add battlefield-wide type selection on a second T press (#97): the
      manual's double-press expands the represented living owned types beyond
      the viewport. The native 400 ms window requires the same types and
      selection as the first press; selection changes, focus loss, menu input
      and match replacement interrupt it. Count feedback, camera, queued
      orders and complete Session bytes stay unchanged at two layouts.
      Original timing and physical Mac/Retina checks remain open
      (`docs/control-groups.md`).
- [x] Align H with the documented base-navigation shortcut (#318, #97):
      center the oldest owned living construction yard using drawn ground
      projection and bounded world dimensions. Keep selection, queued orders,
      zoom and armed command modes; cancel stale pointer tracking. Preserve
      Harvest commands on Ctrl+H and document the intentional binding change.
      Native elevated/bounds/layout, mode, guard and command tests preserve
      Session bytes. Yard choice, fallback/cycling and notice calibration remain
      native policies; retail/physical checks remain open (`docs/camera-input.md`).
- [x] Require consecutive deliberate T input for global expansion (#321,
      #97): fresh other keys, accepted Left/Right presses, routed wheels and
      rejected tracked T chords interrupt the prior window. Guarded repeats,
      rejected input and pointer motion preserve normal double-T behavior.
      Native Session regressions reproduce the old offscreen addition and
      verify viewport-only restart, valid global pairs and unchanged camera,
      orders and save bytes (`docs/control-groups.md`). Original timing and
      physical/retail calibration remain open.
- [x] Add battlefield-wide P selection (#325, #97): select living owned
      units of every type without a seed, including offscreen harvesters, and
      prune ineligible IDs. Native handlers cancel old gestures/modes and
      transient double-press state while retaining teams, camera, READY work,
      queued orders and complete Session bytes. Authored compositor, input
      guards and count/empty feedback checks pass. The manual establishes
      mapping/scope; original, retail and physical calibration remain open
      (`docs/control-groups.md`).
- [x] Reject Command-modified team numbers (#327, #97) before recall,
      assignment or camera/mode mutations. Fresh rejected chords end old
      recall timing; assigned membership, selection, pointer gestures, camera,
      queued orders and Session bytes remain intact. Authored baseline/fixed
      regressions cover all nine slots, modifier combinations, ordinary
      shortcuts and shared composition. Original and physical/retail
      calibration remain open (`docs/control-groups.md`).
- [x] Use configured frame dimensions for team centering (#328, #97):
      share `camera_size()` so no-surface handlers honor compact frames while
      retaining the live-texture lookup and default-size result. Authored
      projected-midpoint checks cover all nine slots, three sizes/zooms and
      single/mixed teams; resize/compositor proof preserves selection, teams,
      zoom, orders and Session bytes. Existing redraw bounds and original/
      physical calibration remain unchanged (`docs/control-groups.md`).
- [x] Isolate Command-modified keys from world actions (#331, #97): menu
      editing routes first; rejected keys cannot submit orders, replace modes,
      write save/preferences or change camera input. Suppression prevents old
      repeats after modifier release and ends stale type/team timing on fresh
      chords. Authored dispatch/compositor and actual protected-file checks
      preserve orders, gestures, selection and Session bytes; documented plain/
      Ctrl shortcuts remain. Physical/original calibration is still open
      (`docs/window-input.md`, `docs/order-feedback.md`).
- [x] Add N creation-order unit navigation (#333, #97): cycle living
      owned units by stable ID, including offscreen harvesters, with wrap and
      bounded ground-projected camera centering. Native ordinal/empty feedback,
      input guards and gesture/mode cleanup preserve assigned teams, READY work,
      zoom, pending orders and Session bytes. Authored layout/zoom sequences
      cover filters, seeds, singleton/empty worlds and stale releases; original
      and physical/retail calibration remain open (`docs/control-groups.md`).
- [x] Show health in resolved hover previews (#335, #97/#98): reuse the
      existing pip raster for healthy selection/attack/service targets without
      a separate hit test or selection change. Shared compositor tests cover
      45 layout/zoom/target cases, thirteen ownership guards and unchanged
      selected/damaged visibility, camera, orders and Session bytes. Original
      visibility policy and physical/retail calibration remain open
      (`docs/order-feedback.md`).
- [x] Follow a selected owned unit with F (#337, #97): toggle transient
      tracking from authoritative subcell ground positions through redraw,
      preserving zoom and existing raster bounds. Manual gestures/camera input,
      context changes, invalid targets and READY pickup stop follow; ordinary
      sidebar production remains usable. Authored moving/layout/zoom, guards,
      cancellation, READY/team and Session-byte checks pass. Original/retail and
      physical calibration remain open (`docs/camera-input.md`).
- [x] Frame installed matches on the owned base (#379, #97): direct window
      startup and completed fresh/restart/saved menu loads share terrain-aware,
      bounded construction-yard framing, with owned-building/unit and empty-map
      fallbacks. Zoom and exact Session/save bytes remain unchanged; original
      framing and physical input calibration remain open (`docs/camera-input.md`).
- [x] Isolate Alt/Option-modified world keys (#381, #97): retain native Alt
      state, preserve menu text routing and reject unsupported tactical, storage,
      camera and selection chords through the existing desktop-key guard.
      Repeat/focus recovery and protected-file/Session checks preserve ordinary
      bindings (`docs/window-input.md`). Physical Mac/Option and original
      modifier calibration remain open.
- [ ] Complete world interaction and desktop UX (#97): consistent selection,
      supported control groups/hotkeys, hover/target/order feedback, bounded
      camera movement and configurable edge scrolling. Clear held input on
      focus/menu changes; verify resizing, Retina hit coordinates, tooltip
      clipping and visible save/load feedback. Sidebar queues and command-bar
      controls remain in #86; repair/sell remains in #94.
- [x] Keep Command/Super+Enter outside menu confirmations (#403), alongside
      Alt/Option+Enter. Protected overwrite/delete/load targets, menu focus and
      repeat suppression survive desktop chords; plain Enter, Command+A name
      replacement and Option text retain their routes. Authored native-handler
      and shared-compositor checks preserve complete Session bytes and queued
      orders (`docs/window-input.md`). Physical Mac delivery remains open.
- [x] Stop held camera keys when Alt/Option or Command/Super arrives later
      (#393). Clear active keyboard ownership immediately, retain suppression
      after modifier release, and require key-up or fresh input. Ordinary
      Control/Shift panning, menu editing and complete saved match bytes stay
      intact (`docs/window-input.md`, `docs/camera-input.md`).

- [x] Animate accepted menu clicks and route their supplied sound event (#344,
      #96): bounded SDBTNANM press/persistent-control phases and MenuClick audio
      preserve immediate navigation and disabled/cancelled/repeated/muted guards.
      Authored frames and mixed PCM verify routing without a physical device;
      original transition order/cadence and retail playback remain uncalibrated
      (`docs/formats/front-menu.md`).

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
