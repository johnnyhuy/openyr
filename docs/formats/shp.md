# SHP sprites (TS/RA2 variant)

Palette-indexed sprite sheets: cameos, buildings, infantry, animations,
overlays, theater objects (trees, wakes). Index 0 is transparent. Parser:
`crates/yr-formats/src/shp.rs`.

## Layout

All integers little-endian.

```
file header (8 bytes)
  u16 zero          always 0 (distinguishes from TD/RA1 SHP)
  u16 width         canvas width shared by every frame
  u16 height        canvas height
  u16 frame_count

frame header (24 bytes) x frame_count
  u16 x, y          frame bounding box position inside the canvas
  u16 w, h          frame bounding box size
  u32 flags         low byte = compression (below)
  u8  radar[3], pad average colour for the radar/minimap
  u32 reserved
  u32 offset        absolute file offset of pixel data; 0 = empty frame
```

Compression (low byte of `flags`):

| Value | Encoding |
|---|---|
| 0, 1 | raw: `w * h` bytes |
| 2 | per row: `u16 len` (counts itself, always `w + 2`) then `w` raw bytes |
| 3 | per row: `u16 len` (counts itself) then zero-run RLE: `00 n` = `n` transparent pixels, any other byte = literal |

Type 3 rows must contain complete zero/count pairs and enough decoded pixels
to fill their frame bounding-box width. Retail encoding permits a final
transparent run to reach exactly one pixel beyond that width; that pixel is
discarded. A literal outside the width, a run exceeding the one-pixel allowance,
a missing count or an underfilled row is rejected as `BadFrame`. Row prefixes
bound each encoded slice, so a run cannot borrow a count from the next row.
Zero-length runs remain harmless no-ops.

The #375 audit independently walked all type 3 row bytes in the 16 retail MIX
archives, including nested archives. Of 5,778,273 rows, 624,940 ended exactly at
the width and 5,153,333 ended one transparent pixel beyond it. None ended short,
had an incomplete pair or placed a literal outside the width. For example,
`expandmd01.mix/0x51d8da20`, frame 0, row 0 has width 76 and bytes
`00 33 55 27 54 56 00 16`: 51 transparent pixels, four literals and 22
transparent pixels, totaling 77. Requiring exact width would reject valid
retail artwork. Authored tests retain both endings, long runs, zero-length
runs and canvas offsets while checking malformed frame/canvas and CLI failures.

## Empirical notes (retail corpus)

`mix-dump render` preflights the complete inspection sheet before decoding its
frames (#397). Checked dimensions and a 1 GiB RGBA budget bound composition;
fallible reservation reports failure before creating or replacing an output.
Empty frame slots need no canvas decoding and retain their transparent sheet
positions, including the legal 0x0 NULL.SHP case. Authored colored/empty-frame
sheets retain exact PNG bytes. This is an inspection budget, not a format limit
or native-rendering change; current retail verification needs the install.

`mix-dump scan` over all 16 retail archives decodes every frame of 5,450 SHP
files (83,626 non-empty frames) with zero errors:

| Compression | Frames |
|---|---|
| 0 | 2,452 |
| 1 | 5,296 |
| 2 | 252 |
| 3 | 75,626 |

- **Type 2 carries a length prefix on every row**, and every one equals
  `w + 2` (the decoder enforces this; 252 frames pass). OpenRA's loader reads a
  single prefix and then contiguous rows, which only works for one-row frames.
- **0x0 canvases are legal**: `ra2.mix/CONQUER.MIX/NULL.SHP` is 56 bytes, a
  0x0 canvas with two empty frames whose reserved fields hold `0xCC` (MSVC's
  uninitialised-stack fill), so the original tool wrote frame headers straight
  from uninitialised structs.
- Cameos are 60x48 single-frame raw SHPs (`language.mix/CAMEO.MIX/HTNKICON.SHP`).
  They are localised assets, so they live in `language.mix`, not `ra2.mix`.
- Odd frame dimensions occur; frames are not padded in storage.
- `expandmd01.mix/0x65ac42bd` (360 KB) is a single-frame 632x568 SHP.
- Building SHPs store body frames first, then the same number of shadow
  frames (`GACNST.SHP`: 3 body states, normal and two damage levels, then 3
  shadows). Shadow pixels are coverage masks: draw them as translucent black,
  not via the palette. Visual check: `mix-dump render` (the cameo, the snow
  Construction Yard and the urban road templates render correctly).
- Infantry SHPs start with the 8-frame standing ("Ready") sequence, one
  frame per screen octant, **counter-clockwise from screen up**: frame 0
  faces up (back to the camera), 1 up-left, 2 left, 3 down-left, 4 down
  (facing the camera), 5 down-right, 6 right, 7 up-right. The shadow
  frames follow the body frames as for buildings (`frames / 2` offset).
  Observed on `RA2.MIX/CONQUER.MIX/GI.SHP` (`mix-dump render` with
  `UNITTEM.PAL`); used by `yr-render::sprites`.

## Infantry action sequences

`artmd.ini` was extracted from `ra2md.mix/LOCALMD.MIX/ARTMD.INI`.
The rules `E1` image resolves to `GI`, whose art section names
`GISequence`. `GGI` names `GuardianGISequence`. The sequence values are
`start,count,facing_stride[,fixed_facing]`:

| Sequence | GI | Guardian GI |
|---|---|---|
| Ready | `0,1,1` | `0,1,1` |
| Walk | `8,6,6` | `8,6,6` |
| FireUp | `164,6,6` | `204,6,6` |
| Idle1 | `56,15,0,S` | `56,15,0,S` |
| Idle2 | `71,15,0,E` | `71,15,0,E` |

The body frame is `start + facing * facing_stride + phase`, with facing
counter-clockwise from screen up as above. A zero stride selects the
same frames for every facing; the idle art already depicts the fixed
direction. The shadow is at `body_frame + total_frames / 2`.
`infantry_animation_matches_retail_sequence_canvases` in
`crates/yr-render/tests/retail.rs` compares every body/shadow coverage
pixel and every non-remap body colour against the independent SHP
canvases for both types, all facings, and every walk/fire/idle phase.
`OPENYR_ANIMATION_DUMP=<external-dir>` exports those sprites as PNGs.

The live window and `--frames` share the following prototype policy:

- Two simulation ticks per animation frame at 15 ticks/s. This cadence
  is a presentation choice, not an oracle-verified timing rule.
- Actual position changes select looping Walk. A blocked or stopped
  unit returns to Ready even if it still has a movement target.
- A newly observed shot starts FireUp. It plays once, returns to Ready,
  and restarts on the next shot. Movement takes priority.
- A stationary unit occasionally plays Idle1 or Idle2, alternating in
  90-tick intervals, then returns to its current Ready facing.
- Clocks live in `yr-app::animation`, outside the simulation hash.
  Repeated redraws without another simulation tick do not advance them.
- Checked arithmetic validates the complete sequence, across all eight
  facings, against the actual SHP body-frame count. Invalid or absent
  action data falls back to Ready, then a safe standing frame. The
  decoded cache is keyed by the resulting frame, not an unbounded tick.

Prone, deployed and paradrop sequences are not wired to sim states yet.
Wine side-by-side validation remains under #6.

## Ordinary infantry deaths

Retail `RULESMD.INI` documents `InfDeath` as the warhead's infantry death
selector. SA and HollowPoint use 1; HE uses 2. GISequence and
GuardianGISequence both declare `Die1=134,15,0` and `Die2=149,15,0`.
These are non-directional body ranges with corresponding shadow frames
at the normal half-sheet offset. AP uses 3; its initial generic explosion is documented in
`docs/formats/effects.md`. Flying fragments remain separate work, and the
placeholder `Die3=0,1,1` is not a supported unit-sheet death clip.

Explicit selectors 1 and 2 use these unit-sheet clips. Selectors 3, 4, 5, 6, 7, 8
and 10 use separate generic death sheets, documented in `docs/formats/effects.md`.
The fatal positive health transition emits `FatalHit` with the actual shot origin and victim
type, position, facing and owner at that hit. The app associates it with
the later `Died` event, including victims spawned or reassigned during
the tick. A later arrival against the already dead
target cannot replace the lethal weapon. Saved in-flight shots keep their
source even after the shooter dies. Unknown/legacy origins, direct destroy
commands and unsupported selectors omit this optional animation.

The clip uses the theater unit palette and the victim's house remap,
with the matching half-alpha shadow in the shared ground coverage mask.
Bodies join live objects in foundation painter order, so dying infantry
do not paint over buildings in front of them. There is no selectable
entity, health bar, occupancy or simulation timer for a death clip.
The existing provisional two-tick cadence plays each frame once, then
removes the sprite at age 30 for the retail 15-frame clips. Drawing never
advances the clock. Each scene caps dying actors at 256; their decoded
frames share the existing 512-entry unit cache. Loading clears deaths
along with other past transients, while later saved-projectile arrivals
can create new ones.

Sequence arithmetic validates all eight facings before selection. Infantry
sequence loading rejects zero/oversized canvases (over 256x256), odd frame
counts and sheets above 4096 frames; parser table allocations remain
bounded by source bytes. Missing, malformed and expired death data draw
nothing, including no fallback standing unit or marker. Retail tests compare
all 60 GI/Guardian GI death body/shadow frames, all facings and two house
colors against independent canvases. App tests exercise actual fatal shots,
saved flights, source loss, expiry, repeated redraws and unchanged sim/save
bytes. `OPENYR_INFANTRY_DEATH_PROOF=<external-dir>` exports a controlled
EB1 GI/IFV sequence from the shared compositor.

This establishes retail frame layout and native routing, not original
playback parity. Original cadence, corpse persistence, death/corpse layering,
other special death selectors, death overrides and prone/deployed deaths remain
in #102. No original implementation code was consulted.

## Building layers

Retail `artmd.ini` names building attachments separately from the body.
The body and all probed attachments have a body half followed by an
equal-sized shadow half. The shadow half exists even for attachments
whose art section does not request `Shadow=yes`.

The following was checked against `ra2.mix/GENERIC.MIX` and
`ra2md.mix/LOCALMD.MIX/ARTMD.INI`, using independent body/shadow pixel
comparisons in `building_layers_match_retail_canvases`:

| Building attachment | Normal body range | Damaged body range | Shadow requested |
|---|---|---|---|
| `GACNST_A` | 0..3 | 3..6 | no |
| `GACNST_B` (production) | 0..20 | 20..40 | normal only |
| `GAPOWR_A` | 0..8 | 8..16 | no |
| `GAPILE_A` | 0..15 | 16..31 | yes |
| `GAWEAP_A` | 0..15 | 16..29 | no |
| `GAWEAP_B` | 0..5 | 0..5 | no |
| `GAREFNL1`, `L2`, `L3` | 0..3 | normal fallback | no |
| `GAREFNOR` (unloading) | 0..19 | normal fallback | no |

Ranges use an exclusive end, matching the retail `LoopStart`/`LoopEnd`
boundaries in the shared sheets. Some sheets leave an unused boundary
frame. Damaged sections redirect `Image=` back to the normal sheet.
`ActiveAnimFour=GAREFNL4` is present in the INI, but none of its theater,
generic or plain SHPs resolve in this install; it is safely skipped.

`GAREFN` names `SpecialAnim=GAREFNOR`. The generic `GGREFNOR.SHP`
resolves from `RA2.MIX/GENERIC.MIX`; its header is a 210x196 canvas
with 40 frames (20 body, 20 shadow). The retail art section specifies
`LoopStart=0`, `LoopEnd=19`, `LoopCount=1`, `Rate=200`, and no shadow.
The same independent canvas test checks all 19 selected unloading frames,
including normal-art fallback when the refinery is damaged.

`BibShape=GAREFNBB` and `GAWEAPBB` resolve to generic sheets with four
frames: normal/damaged bodies followed by their shadows. All the above
attachments share their parent's canvas and anchor. Their art centre is
offset from the top-left footprint cell by
`((foundation_w-foundation_h)*15, (foundation_w+foundation_h-2)*15/2)`.
Attachment `X`/`Y` offsets are added in native pixels. Filename lookup
uses the theater variant, generic `G`, then the plain image name.

The window and `--frames` now use the same building presentation policy:

- Body and bib frame 0 at normal health, frame 1 at half health or below.
  This threshold matches the retail `ConditionYellow=50%` value.
- Up to four active attachments and one production attachment, with
  damaged variants selected independently and normal art as fallback.
- `LoopStart` (or `Start` when absent) through exclusive `LoopEnd`.
  Missing ends use the actual body count. Invalid, empty, overflowing or
  out-of-body ranges are skipped, preserving the rest of the building.
- Prototype cadence is `floor(ticks*Rate/225)`, modulo the range length.
  `Rate` defaults to 200. This interpretation is provisional; the Wine
  timing comparison has not been done.
- Production uses whole ticks of queue progress at the sim's first
  player factory of that role. Credit stalls freeze the frame, and a
  finished structure awaiting placement stops the production overlay.
- Refinery `SpecialAnim` starts at elapsed tick zero of an actual
  harvester unloading operation. It plays once through the exclusive end,
  then disappears; it never loops while the unloading timer finishes.
  Cargo delivery, refinery loss or movement away stops the overlay.
  The first active harvester in id order supplies the presentation clock.
  This clock is derived from saved sim state, so loading mid-unload
  resumes the correct phase without a separate presentation snapshot.
- Explicit `ActiveAnim*Powered=yes` gates an attachment's clock on its
  owner's power balance. Missing flags leave the attachment running.
- Bibs draw on the ground before all live bodies. Body and attachment
  shadow masks are unioned on the ground, darkening each covered pixel
  once before any live body is painted. Shadows never paint over a
  neighbouring live building's body.
- Foundation bounds establish painter dependencies along both map
  axes. A large building stays behind objects beyond its east or south
  edge even when its front-cell diagonal exceeds theirs. Unconstrained
  objects, overlaps and cycles use stable diagonal order.
- Decoded layers are cached separately by file, frame, remap, shadow
  flag and anchor. Overlay combinations and elapsed ticks are not keys.

`OPENYR_BUILDING_DUMP=<external-dir>` exports normal/damaged composites
for five Allied base types at ticks 0 through 23 from the retail test.
These contain retail artwork and must stay outside git.

## Static factory shells

`UnderDoorAnim`, `UnderRoofDoorAnim`, `RoofDeployingAnim` and
`DeployingAnim` also supply pieces of the idle factory. Omitting these
sheets loses panel and damaged-state pixels. The base sheet already
contains most of the closed factory. Individual retail sheet renders show
a lower shell, roof interior, raised side and closed roof. The prototype draws them in that order after the base body
and before the active attachments. These panels remain closed during
vehicle rollout; their movement and depth still need the Wine comparison.

All twelve shell sheets below have four frames: normal body, damaged
body, then two shadow masks. Canvas sizes are 266x224 for Allied,
282x224 for Soviet and 272x242 for Yuri. The attachment sections
do not request shadows. This was checked by parsing the actual SHP headers
and comparing every selected body pixel and anchor in
`building_layers_match_retail_canvases`, at ticks 0, 17 and `u64::MAX`,
including low power and an active production queue.

| Factory | Lower shell | Roof interior | Raised side | Closed roof |
|---|---|---|---|---|
| Allied | `GGWEAP_1` | `GGWEAP_4` | `GGWEAP_3` | `GGWEAP_2` |
| Soviet | `NGWEAP_1` | `NGWEAP_3` | `NGWEAP_4` | `NGWEAP_2` |
| Yuri | `YGWEAP_1` | `YGWEAP_4` | `YGWEAP_3` | `YGWEAP_2` |

Allied and Soviet files resolve from `RA2.MIX/GENERIC.MIX`; Yuri files
resolve from `RA2MD.MIX/GENERMD.MIX`. These static states cannot use the
animation ranges blindly. In retail `artmd.ini`, `GAWEAP_3` and
`GAWEAP_4` claim `LoopEnd=9`, and their damaged sections claim
`LoopStart=10`, `LoopEnd=19`, despite having only two body frames.
Static body, bib and shell layers therefore use the actual body count
and health state. Animated attachments still require a valid bounded
loop range. `Image=` redirects and per-attachment offsets still apply.

Allied and Yuri omit explicit damaged attachment keys; their shell
sheets still contain the damaged body at frame 1. Soviet keys redirect
to the shared sheets explicitly. `OPENYR_BUILDING_DUMP` also writes
`GAWEAP-<health>-<tick>-before.png` by omitting just the four new layers
at the same tick, health and remap for comparison with the fixed output.

Factory door/deployment animations, per-overlay
`ZAdjust`/`YSort` depth, rubble, and the Wine side-by-side comparison
remain fidelity work under #6. Static map objects are still baked into
the terrain image; the new layers and ordering apply to live objects.

The retail EB1 economy test starts the existing harvester with the same
`Harvest` command as the native Ctrl+H shortcut (formerly H). Unloading starts at tick 623, pays 500
credits at tick 654, and pays another 500 at tick 1399. Setting
`OPENYR_REFINERY_SAVE=<external-file>` when running
`eb1_harvester_delivers_to_retail_refinery_with_working_animation` saves
the tick-623 state for `openyr --load <file> --frames <external-dir>
623 630 640 654 --passive`. All captures stay outside git.

## Sources

- XCC Utilities `shp_ts_file` (Olaf van der Spek) and ModEnc "SHP (TS)".
- OpenRA `ShpTSLoader.cs` (GPL-3.0), used as a cross-check only.
- Verified against retail data as above.

## Pre-placed building idle layers (#341)

The match background uses `render_map_background`, leaving reconstructable
SHP building structures for live composition. Unsupported static art remains
baked; the same body/layer loader controls exclusion and live capability. Ordinary `render_map` inspection and chooser
previews retain their static object image. Neutral map structures now use the
same body, bib, active layers, damage state, shadows and foundation ordering as
other live buildings. Active clips already read absolute simulation ticks while
production is idle; this fix removes the static-background exception. It does
not invent clips for a building without an authored active attachment.

The SpriteBank retains the original map house colour by building type and cell,
because Session intentionally collapses pre-placed map structures to a neutral
simulation owner. This lookup is presentation-only, reconstructed from the map;
no house metadata or animation state is added to simulation or save fields.
Neutral units remain static as before. Missing or malformed optional active
layers are skipped while valid body/attachments remain visible.

Authored tests compare match background and complete preview at identical map
bounds, original-house versus generic-neutral remap, and two actual shared
compositions at three layouts with no production. Idle pixels advance while
composition preserves complete Session bytes/hash, selection and pending orders.
Destroying the building leaves no stale baked body. Set
`OPENYR_BUILDING_IDLE_PROOF` with `cargo test -p yr-app --bin openyr
building_idle -- --nocapture` to export the authored frames and state receipts.
These fixtures are not retail/original captures. Original cadence, layer depth,
retail reconstruction and physical Mac/Retina checks remain open under #6.

## Static map unit standing frames (#350)

Static vehicle/infantry/aircraft SHP jobs now use the same projected heading and
standing/Ready selection as live SpriteBank images. Sequence loading and bounds
checks are shared; absent/invalid Ready uses the existing facing fallback and
an unavailable body frame falls back to zero. Its shadow follows that resolved
body frame, never the rejected index. Buildings and nonunit map layers retain
their paths. Authored directional frames, a nonzero Ready start and shadow
masks verify agreement. This is native consistency, not original direction or
cadence calibration. Supported map-unit bodies now use the live lifetime and
original-house path in #351, described in `map.md`; unsupported art stays static.

## Decoded building-layer storage (#411)

Building frames were previously cached indefinitely by filename, frame,
shadow flag, house remap and anchor offset. A 600-frame authored working
overlay exceeded 512 retained entries at tick 510; further phases and colors
continued growing the cache. This is a native storage defect, not evidence
about the original engine's cache implementation or animation timing.

The live building cache now retains at most 512 entries and 128 MiB of RGBA
vector capacity. Failed decodes count toward the entry limit. FIFO eviction
skips layers already resolved for the current building request, so loading a
new layer cannot evict that request's base or earlier layers. Requests resolve
the base first, followed by shells, active, production and unloading layers,
then the bib. The returned bib still draws below bodies as before; insertion
priority does not change draw order. If a modded building's simultaneous
working set exceeds the budget, later optional layers are omitted and retried
on future requests. Budget rejection is not cached as a decode failure.

The byte limit covers retained decoded building RGBA allocations, not total
process memory, parsed art, other caches or transient decoding. Existing SHP
canvas limits still apply before decoding. Eviction reconstructs frames from
the same source bytes, selected phase and remap; it adds no simulation fields,
RNG calls, save metadata or clocks.

Authored regressions exercise 1,200 phase/color requests, entry and byte
pressure, negative entries, protected working sets, optional-layer retry and
exact body/shadow/anchor reconstruction. A retail check churns 600 Construction
Yard colors and compares 24 complete factory/refinery/yard requests before and
after eviction, including damaged, powered, production and unloading states.
The existing independent retail SHP-canvas comparisons remain in place.

```sh
cargo test -p yr-render cache -- --nocapture
cargo test -p yr-render --test retail building_layers_match_retail_canvases
```

Set `OPENYR_BUILDING_CACHE_PROOF` for the retail cache test to export matching
cold/rebuilt factory layer PNGs outside git. These source-pixel comparisons do
not establish original animation cadence, per-overlay depth or physical Mac
display calibration, which remain in #6.
