# Native detail and combat effects

## Native world pixels

The app now defaults to map scale 1. TMP cells retain their authored 60x30
diamonds and live SHP/VXL art draws at one framebuffer pixel per source
pixel at zoom 1. Previously scale 2 sampled every second terrain row and
column and reduced live sprites to half size at the default zoom. Terrain
detail discarded during that map render could not be recovered by zooming.
Explicit scales 2 and 4 remain inspection modes; argument validation now
matches the renderer's supported set of 1, 2 and 4.

This restores source detail rather than inventing higher-resolution art.
Nearest-neighbor enlargement remains deliberate, and fractional zoom can
still alias. The window uses a 1280x800 framebuffer, which the existing
surface presentation scales for the display. Retina/display resizing,
optional filtering and indexed voxel/VPL lighting remain fidelity work.
The sidebar remains 168 framebuffer pixels wide independently of map scale.
Keeping the native terrain buffer costs about four times the old scale-2
buffer memory; EB1 uses about 30.5 MiB instead of 7.6 MiB.

## Retail sheets and routing

Local rules and art resolve the following authored effects:

| Trigger | Rules path | Sheet | Canvas | Frames |
|---|---|---|---|---|
| Tank firing | MTNK/HTNK Primary to weapon Anim | GUNFIRE.SHP | 28x26 | 4 |
| GI firing | E1 Primary=M60 to Anim | MGUN-N through MGUN-NW.SHP | 18x17 | 6 each |
| Vehicle destruction | First entry of type Explosion | TWLT070.SHP | 186x196 | 26 |
| AP shell impact | Fired 105mm/120mm to AP AnimList | S_CLSN16/S_CLSN22.SHP | 148x158 | 13 each |
| Small-arms impact | Fired M60 to SA AnimList | PIFFPIFF.SHP | 28x28 | 12 |
| Human infantry explosion | Lethal warhead InfDeath=3 to General InfantryExplode | S_BANG34.SHP | 148x158 | 13 |
| Human infantry burn collapse | Lethal warhead InfDeath=4 to General FlamingInfantry | FLAMEGUY.SHP | 52x102 | 60 collapse, 216 total |
| Human infantry electrocution | Lethal warhead InfDeath=5 | ELECTRO.SHP | 60x56 | 15 |
| Human infantry Virus death | Lethal warhead InfDeath=8 | VIRUSD.SHP | 186x190 | 35 |
| Human infantry Nuke Melt | Lethal warhead InfDeath=7 | NUKEDIE.SHP | 60x60 | 60 |
| Human infantry Brute smash | Lethal warhead InfDeath=10 to General InfantryBrute | BRUTDIE.SHP | 102x118 | 60 |

`105mm` and `120mm` both declare `Anim=GUNFIRE`. M60 declares eight
machine-gun names in screen compass order. Existing facing projection
selects the corresponding entry. The common vehicle `Explosion` list
starts with `TWLT070`; this first slice always selects that first entry.
It does not consume simulation randomness to choose among alternatives.

These effects use the retail `ANIM.PAL`. Source index zero is transparent.
Every frame belongs to the effect; there is no assumed second shadow half.
`Image` redirects are accepted only as bounded simple asset names. Canvas
dimensions must be at most 512x512 and frame counts at most 256 before any
canvas decoding. The SHP parser separately validates headers against actual
bytes. Invalid/missing effects are skipped without hiding world objects.

## Presentation policy

`Event::Fired` selects muzzle art from the shooter's actual type/Primary.
`Event::Died` selects destruction art from the destroyed type. Before each
tick the app remembers entity types and positions, preserving effects from
an entity that fires and dies during that same tick. Surviving entities use
their current poses. Effects are centered on the entity's ground position
with the map's actual terrain-height lift. Barrel/infantry muzzle offsets
and art-specific layer overrides still need original-game observation.
Ground-position effects and cannon bodies now join units, buildings and
infantry death clips in the footprint painter order (#204). A rear effect
therefore passes behind a foreground body instead of painting through it.
An effect in front retains straight-alpha blending. Effects on a known
building inherit its full foundation, while their draw anchor remains the
actual muzzle/impact/destruction position. This keeps impacts above the
struck building itself and preserves its depth against adjacent structures.
Remembered foundations survive the building's destruction; unknown targets
fall back to an authoritative ground point. Equal-position units or matching
foundations precede their combat art; equal-position cannon bodies precede
transient clips, and clips keep event insertion order. Bibs and unioned
ground shadows remain below bodies, with health bars and UI above all combat art.

This is the existing native ground-plane policy, not original-engine layer
parity. Effect `Layer`, `ZAdjust` and `YSort` overrides, airborne projectile
height, muzzle offsets and interleaving with objects already baked into the
terrain image remain unimplemented or uncalibrated. No new simulation
position, clock, RNG call or saved state is introduced.

Clips play from `Start` (default 0) to exclusive `End` (default actual frame
count). Finite `LoopCount` defaults to 1 and allows 1 through 1024 total
traversals of `[LoopStart, LoopEnd)`, defaulting to the full clip. Any prefix
plays once before the loop and any tail once afterwards. Every endpoint
must lie within the actual sheet and clip. Zero, negative/infinite, malformed
and oversized counts are unsupported optional art. No frame copies or timers
are allocated from loop counts. Positive `Rate` uses the existing provisional
presentation interpretation `floor(ticks * Rate / 225)`, default 200 at 15 simulation
ticks/second. Drawing never advances a clock. `Translucent=yes` defaults to
50 percent transparency, and a valid `Translucency` percentage overrides it.
Straight-alpha blending uses widened integer channels. These timings,
transparency rules, palette lighting and anchors need comparison with the
original game; source-art comparisons do not establish exact engine parity.

The cache holds at most 64 decoded effect frames and 128 clip definitions.
At most 256 effects remain active; the oldest is dropped when full. Expired
clips disappear on simulation updates even without redraws. Cache keys use
resolved source frames rather than elapsed ticks. Rendering effects does not
mutate sim hashes, snapshots or RNG. Loading resets transient effects,
so prior flashes/explosions are not replayed from a snapshot.

## Warhead impacts

Every new shot retains its original weapon identity and delivered damage,
before target armour. Instant hits and projectile arrival emit an `Impact`
with the target's actual position. Impact is distinct from positive health
damage: armour can reduce a hit to zero without erasing its visual arrival.
A shell remains in flight after its shooter dies (#119). Its origin is saved
with the projectile, so loading does not require a remembered shooter or
guess a weapon from the victim's type. Target loss still discards the homing
projectile under the current combat model.

Weapon identities use case-insensitive FNV-1a over the rules ID. Both creation
and lookup reject ambiguous IDs rather than assigning another weapon's art.
The identity is independent of registry insertion order. Unknown sources and
legacy in-flight shots without provenance retain gameplay but omit impact art.
Neither provenance nor output events affect state hashes or consume RNG.

The retail `AnimList` comment establishes increasing damage order. The
modding field documentation for [AnimList](https://modenc.renegadeprojects.com/AnimList)
specifies 25-point bands of delivered damage, clamped to the final entry;
target-dependent reductions do not select the band. OpenYR follows that
documented rule for ordinary target impacts. Thus 105mm's 65 damage selects
AP's last entry, S_CLSN22, while M60's 15 damage selects SA's PIFFPIFF.
Boundary tests cover 24/25 and 49/50, zero/negative damage and large values.
This is a data/documentation comparison, not a controlled original-game
timing observation. No implementation source was used.

The existing bounded effect decoder supplies ANIM.PAL pixels, transparency,
one-shot playback and terrain-height anchoring. The impact starts at age zero
on its arrival tick and pauses with the simulation. Loading clears past
transients but preserves new in-flight shot origins. The exact anchor, depth,
normal lighting and cadence remain provisional, as above. Random EMEffect
selection is omitted; water-cell detonation/splash, special warhead overrides,
impact sounds, craters and scorch are not implemented in this slice.

Ordinary infantry `Die1`/`Die2` sequences now use the lethal shot's explicit
warhead selector and retain victim art and owner outside the sim. Their
unit-palette bodies and shadows follow world depth ordering; see
`docs/formats/shp.md` for layout, limits and the provisional playback policy.
`FatalHit` records only the first positive-health transition to death;
later same-phase impacts cannot change its cause. Past death clips reset
on load, while newly arriving saved shots can start them.

Projectile trails and other projectile classes, smoke/fire, debris,
environmental animations, other special infantry deaths and special weapon
effects remain in #102. Exact destruction selection and original compositing
remain there too.

## Electrocution deaths

Retail `RULESMD.INI` comments define `InfDeath=5` as electro. Both
`ElectricBolt`/`Shock` and `TankBolt`/`Electric` declare that selector.
`ARTMD.INI` labels `[ELECTRO]` as the infantry Tesla zap animation, with
`Layer=ground`, `LoopEnd=3`, `LoopCount=5`, `Scorch=yes` and
`Report=InfantryZap`. It declares no palette override, shadow, transparency,
start, end or rate. `RA2.MIX/CONQUER.MIX/ELECTRO.SHP` has 15 frames on a
60x56 canvas; every frame decodes with `ANIM.PAL`. There is no shadow half.
These are local retail data observations, without engine source inspection.

The actual lethal weapon now selects this generic clip for registered
infantry without `NotHuman=yes`. Animals, vehicles, aircraft, buildings,
unknown weapon sources and unsupported selectors receive no guessed generic
death. The clip ignores victim facing and house remap. Missing or invalid
optional art suppresses the clip without restoring a standing victim.
Non-human special deaths and per-type death overrides remain unverified.

Electrocution uses the same retained fatal pose and world painter order as
ordinary infantry deaths, rather than the transient overlay pass above all
objects. It has no selectable entity, health bar, occupancy or sim timer.
The native loop policy repeats frames 0..3 five times, then plays 3..15.
At default rate 200, its 27 phases occupy ages 0 through 30 and expire at
age 31. Exclusive LoopEnd, total rather than additional loop count, default
rate, anchor and palette lighting are provisional native policies pending
an original-game observation. Source comparisons do not calibrate playback.
The declared scorch and animation report are not applied by this slice;
existing unit death audio remains unchanged.

The existing 256-actor death bound and 64-frame effect cache also bound
electrocution. Repeated phases share the 15 decoded source frames. Updates
expire clips without redraws; loading resets past deaths while retained
in-flight weapons can cause new ones after restore. Tests check all source
frames, loop boundaries, the tail, extreme ages/rates/counts, eligibility,
missing art, saved flights after shooter loss and unchanged sim/save bytes.
The controlled compositor proof adds a retail Tesla Trooper to the test's
type registry because the Allied sandbox does not offer that unit. It fires
its actual ElectricBolt at a GI; ordinary GI/IFV proofs remain alongside it.

## Nuke Melt deaths

Retail `RULESMD.INI` comments explicitly name `InfDeath=7` as Nuke Melt.
`DESO` declares `Primary=RadBeamWeapon`, whose `RadBeamWarhead` uses that
selector with `Radiation=yes`. `RadEruptionWarhead`, `RadSite`, `NUKE`,
`DiskWH` and `AntiPerson` also declare selector 7. The death selector, not
the radiation flag or the shooter's current type, determines the clip.
This slice renders fatal weapon hits; it adds no radiation-field damage.

`ARTMD.INI` has `[NUKEDIE]` with `Scorch=yes`, `Layer=ground` and
`Report=InfantryMelt`. There is no image, palette, transparency, rate, start,
end or loop override. `RA2.MIX/CONQUER.MIX/NUKEDIE.SHP` is 26,584 bytes,
60x60 pixels with 60 frames. Independently decoded `ANIM.PAL` canvases show
a green figure collapsing into a puddle, then disappearing. All 60 frames
are body art, without a shadow half. These are observations from the local
retail data, without original engine source inspection.

The shared special-death route accepts explicit lethal selector 7 for
registered infantry without `NotHuman=yes`. It retains the actual fatal
pose and joins the world painter order, sharing the existing actor/frame
bounds and ignoring house remap and facing. Animals, other techno classes,
unknown sources and missing or malformed optional art get no guessed melt.
The saved lethal weapon still resolves after its shooter disappears; the
victim remains absent from occupancy, selection and simulation state.

The existing default rate 200 plays each source frame once, occupying ages
0 through 67 and expiring at age 68. Repeated redraws do not advance it.
Original timing, palette lighting, scorch, animation reports, per-type
overrides and non-human exceptions remain unverified in #102. Loading
resets past death clips; later saved-flight arrivals can create new ones.
Source comparisons do not establish original-engine parity.

Tests compare all 60 source canvases and anchors, then check eligibility,
optional-art failure, expiry at normal/extreme ages and saved flights after
shooter loss. The controlled EB1 compositor fixture registers a retail
Desolator and fires its actual RadBeamWeapon at a GI. Per-frame exports
include simulation ticks and hashes, and composition preserves complete
save bytes. Export with
`OPENYR_INFANTRY_DEATH_PROOF=<external-dir> cargo test -p yr-app --bin openyr retail_infantry_death_composition_proof -- --nocapture`.

## Yuri head-pop deaths

Retail `RULESMD.INI` comments explicitly name `InfDeath=6` as Yuri head
explode. `[General] InfantryHeadPop=YURIDIE` selects the generic art.
`YURI` declares `Secondary=PsiWave`, which uses `PsiPulse` and selector 6.
`SuperPsiWave` and `CRSuperMindBlast` use `SuperPsiPulse`, also selector 6.
Both warheads declare `PsychicDamage=yes`; the death selector determines
the clip independently of that flag. The current native combat model
supports targeted lethal hits, not Yuri deployment or the declared area
damage. The controlled proof equips Yuri with his retail secondary and
orders a one-cell attack; it does not establish original deployment behavior.

`ARTMD.INI` declares `[YURIDIE]` with `Scorch=yes`, `Layer=ground` and
`Report=InfantryPsyCrush`. Its `AltPalette=yes` line is commented out.
There is no active image, palette, transparency, rate, start, end or loop
override. `RA2.MIX/CONQUER.MIX/YURIDIE.SHP` is 26,864 bytes, with 30 frames
on a 230x150 canvas. Independent SHP decoding and `ANIM.PAL` comparison
check every source frame; there is no shadow half. These are retail data
observations without original engine source inspection.

The shared special-death route now resolves the general setting for lethal
selector 6 on registered infantry without `NotHuman=yes`. It retains the
actual fatal pose and uses world painter order, ignoring facing and house
remap. Missing, invalid or malformed optional art suppresses the clip without
restoring the dead victim or hiding other objects. Other techno classes,
animals and unknown sources receive no guessed generic death.

The existing bounded effect cache and actor limit apply. At provisional
default rate 200, frames occupy ages 0 through 33 and expire at age 34.
Original timing, anchor, palette lighting, scorch, animation reports and
non-human/per-type exceptions remain unverified in #102. Loading clears
past transient deaths; saved-flight arrivals can create new ones even after
the shooter disappears. Tests verify general-setting overrides, all source
pixels, expiry, eligibility, optional-art failure, saved-flight provenance
and unchanged hashes/save bytes. The shared compositor proof exports every
age through expiry using the same command documented for Nuke Melt above.

## Virus deaths

Retail `RULESMD.INI` comments name `InfDeath=8` as Virus explosion.
`VIRUS` declares `Primary=Virusgun` and `ElitePrimary=VirusgunE`; both
weapons use the `Virus` warhead and selector 8. `[General] InfantryVirus=VIRUSD`
selects the generic clip. The general setting remains authoritative when
map rules override it.

`ARTMD.INI` declares `[VIRUSD]` with `Scorch=yes`, `Layer=ground`,
`Report=VirusVictimSwell`, `SpawnsParticle=VirusCloud1` and `NumParticles=3`.
There is no image, palette, transparency, rate, start, end or loop override.
`RA2MD.MIX/CONQMD.MIX/VIRUSD.SHP` is 60,728 bytes, with 35 frames
on a 186x190 canvas. All frames are body art, without a shadow half.
Source comparisons use
`ANIM.PAL`, expanded from six-bit channels with transparent index zero.
These are observations from retail data, without original code inspection.

Explicit lethal selector 8 now uses the shared bounded special-death path
for registered infantry without `NotHuman=yes`. It retains the actual fatal
pose, joins world painter order and ignores house remap and facing.
Missing, unsafe or malformed optional art suppresses the clip without
restoring the dead victim. Other techno classes, animals and unknown
sources receive no guessed Virus death.

The existing provisional rate 200 plays all 35 frames over ages 0 through
39 and expires at age 40. Drawing never advances the clock. Tests compare
all source canvases, check general-setting overrides and optional-art failure,
and retain the lethal weapon through saved flights after shooter loss.
The controlled EB1 fixture registers the retail Virus and fires its actual
Virusgun at a GI. Composition preserves simulation hashes and complete
save bytes; the shared proof command above exports every age through expiry.
Original timing, anchor, lighting, gas particles, scorch, animation reports
and non-human/per-type exceptions remain in #102. Source agreement does
not establish original-engine playback parity.

## Brute smash deaths

Retail `RULESMD.INI` names `InfDeath=10` as getting smashed into pieces by
a Brute. `[General] InfantryBrute=BRUTDIE` selects the generic death art.
`BRUTE` declares `Primary=Punch`, which uses `Battering` and selector 10.
`PunchE`, `Smash`, `SmashE`, `TRexBite` and `TRexInfBite` also resolve to
selector 10. The route follows the actual lethal weapon, independently of
its shooter's type or the victim's current owner. Mutation uses selector 9
and remains separate simulation and rendering work.

`ARTMD.INI` declares `[BRUTDIE]` with `Scorch=yes`, `Layer=ground` and
`Report=InfantryPsyCrush`. Its `AltPalette=yes` line is commented out. There
is no active image, palette, transparency, rate, start, end or loop override.
`RA2MD.MIX/CONQMD.MIX/BRUTDIE.SHP` is 13,336 bytes, with 60 frames on a
102x118 canvas. Independent SHP decoding compares every source canvas
against `ANIM.PAL`; there is no assumed shadow half. These are retail data
observations without original engine source inspection.

Registered infantry without `NotHuman=yes` receive the configured clip
at their actual fatal-hit position and facing in world painter order.
The generic art ignores facing and house remap. Missing general settings,
invalid names, malformed optional art, unknown sources and non-human victims
receive no guessed replacement. The existing actor and cache limits apply.
Rendering never restores occupancy or selection for a dead victim.

At the provisional default rate 200, the clip occupies ages 0 through 67
and expires at age 68. Redraws do not advance it. Original timing, anchor,
palette lighting, scorch, animation reports and non-human/per-type exceptions
remain unverified in #102. Past transients reset on load; saved-flight arrivals
can still create deaths after the shooter disappears.

Tests cover every source frame and anchor, eligibility, general-setting
replacement/removal, optional-art failure, expiry at normal/extreme ages,
saved lethal provenance after shooter loss and unchanged hashes/save bytes.
The shared controlled compositor fixture registers a retail Brute and fires
its actual Punch at a GI one cell away. Per-age exports include ticks and
state hashes, using `OPENYR_INFANTRY_DEATH_PROOF` as documented above.

## Infantry explosion deaths

Retail `RULESMD.INI` names `InfDeath=3` as flying death and declares
`[General] InfantryExplode=S_BANG34`, described as the animation when infantry
explodes. Tank weapons `105mm` and `120mm` use `AP`, which explicitly selects
3; `120mmx` uses `ApocAP` with the same selector. `SonicZap` also selects 3.
These observations come from installed rules, including the EXPANDMD01
rules override, without original engine source inspection.

`ARTMD.INI` declares `[S_BANG34]` with `Normalized=yes`, `Translucent=yes`,
`Report=Explosion10`, `UseNormalLight=yes`, `Crater=yes` and `Scorch=yes`.
There is no active image, rate, start, end or loop override.
`RA2.MIX/CONQUER.MIX/S_BANG34.SHP` is 7,008 bytes, with 13 frames on a
148x158 canvas. Every independently decoded source canvas is compared with
`ANIM.PAL`, transparent index zero and the existing 50-percent translucency
policy (alpha 127). The canvas anchor is (74,79); no shadow half is assumed.

Generic death sheets use straight-alpha effect blending at their world
painter position and bypass the unit shadow union (#192). This preserves
translucent color instead of discarding it in the opaque unit body pass.
Ordinary Die1/Die2 retain their body and shadow passes.

The initial native presentation resolves the general setting for explicit
lethal selector 3 on registered human infantry. It uses the actual fatal-hit
position in world painter order and ignores facing and house remap. The
route does not infer eligibility from the shooter's type. Invalid or missing
settings, unknown sources, non-human victims and malformed optional art
receive no guessed clip. Existing bounded actor and frame caches apply.

At the provisional default rate 200, ages 0 through 14 cover all 13 frames;
the clip expires at age 15. This is the configured explosion only. Original
flying fragments, movement, cadence, anchors, palette lighting, normalization,
crater/scorch and animation reports remain unverified in #102. Presentation
never changes simulation state or restores a dead actor. Past transients reset
on load; later saved-flight arrivals retain the lethal weapon and can create
new clips after shooter loss.

Tests check all source pixels and alpha, human eligibility, general-setting
replacement/removal, optional-art failure, expiry including extreme ages,
saved-flight provenance and unchanged simulation hashes/complete save bytes.
The shared compositor fixture fires a real retail MTNK `105mm` shell at a GI
two cells away and exports per-age ticks/hashes with
`OPENYR_INFANTRY_DEATH_PROOF`, as documented above. The ordinary impact effect
remains independent; removing the death layer provides a same-tick comparison.

## Initial burn collapse

Retail `RULESMD.INI` names `InfDeath=4` as burn death and selects
`[General] FlamingInfantry=FLAMEGUY`. Initiate `PsychicJab` and its elite
variant use `SAFlame`, which selects 4. Mirage `MirageGun` and its elite
variant use `MirageWH`, also selecting 4. The route follows the lethal
weapon, independently of the current shooter type and victim owner.

`ARTMD.INI` declares `[FLAMEGUY]` with `IsFlamingGuy=true`,
`RunningFrames=6`, `LoopCount=1` and `Rate=500`. The installed
`RA2.MIX/CONQUER.MIX/FLAMEGUY.SHP` is 63,112 bytes, with 216 frames on a
52x102 canvas. Contact-sheet inspection shows eight six-frame running
groups at 0-47, a collapse at 48-107 and an empty trailing half at 108-215.
Every trailing frame has an empty SHP header. These observations come from
retail INI/SHP data, without original engine source inspection.

This initial stationary presentation plays only the collapse at the actual
fatal-hit position. For `IsFlamingGuy=true`, the bounded effect loader
requires a positive `RunningFrames`, an even sheet count and a nonempty
collapse after eight running groups. Explicit start/end and loop ranges
must stay within that collapse. Missing, invalid, overflowing or out-of-range
layout values suppress the optional clip before canvas decoding. The same
limits on canvases, actors and caches as other generic deaths apply.

Human infantry with an explicit lethal selector 4 resolve the configured
general setting. Other classes, non-human infantry, unknown provenance and
missing or malformed optional art receive no guessed burn clip. The art
uses the existing `ANIM.PAL` and effect blending policy without house remap,
in world painter order. It creates no simulation entity or occupancy.

At the existing provisional rate conversion, the native 15 Hz clock uses
source frame `48 + floor(age * 500 / 225)`, expiring at age 27. It skips
some source frames at that cadence. Tests slow only their comparison clock
to visit all 60 independently decoded collapse canvases and anchors, then
check the actual retail-rate frame choices and expiry. Redraws preserve
the simulation hash and complete save bytes. Past transients reset on load;
saved flights can still create burns after shooter loss. The controlled EB1
proof registers a retail Initiate and fires its actual PsychicJab at a GI,
using `OPENYR_INFANTRY_DEATH_PROOF` as documented above.

Directional running, motion and the run-to-collapse transition are not yet
implemented. Original cadence, facing, palette lighting, shadow treatment,
reports and corpse persistence remain unverified in #102. Source-pixel
agreement does not establish original-engine parity.

## Cannon bodies and travel

Retail `RULESMD.INI` resolves MTNK's `105mm` and HTNK's `120mm` to `Cannon`,
with `Speed=40`, `Image=120MM` and `Arcing=true`. The corresponding art section
is empty. `RA2.MIX/CONQUER.MIX/120MM.SHP` is 48 bytes, one 24x24 canvas with
12 opaque pixels. Static projectile bodies now decode every pixel with the
current theater's unit palette, transparent index zero and no shadow half.
The retained weapon identity selects the projectile image independently of
the current shooter or target. The renderer accepts one-frame SHP bodies,
caps canvases at 128x128 and caches at most 128 weapon sources. Missing or
ambiguous sources, invalid names/data, invisible types, directional/animated
sheets and voxel projectiles have no guessed body.

The previous rules conversion used `Fix::from_int(Speed * 4)` and multiplied
velocity by an extra 256 (#122). The current native policy uses raw leptons:
Speed 40 becomes 160 leptons/tick. A stationary four-cell test therefore
fires on tick 1 and impacts on tick 7, with a saved flight continuing exactly.
Negative/overflowing physical velocities and values outside the save domain
are rejected. `Inviso=yes` uses an instant hit regardless of its unused Speed;
retail M60/InvisibleLow declares Speed 100 but fires and impacts on tick 1.
The existing no-projectile and zero-speed paths remain instant. A named but
undefined projectile is incomplete weapon data and is rejected.

This corrects an internal unit mismatch; the factor of four is still a native
policy pending controlled original observation in #19. Flight positions keep
the existing deterministic axis-clamped homing model. Original arcing height,
barrel launch offsets, collision, trajectories, trails, palette lighting and
art-specific layers against tall objects remain unimplemented or uncalibrated.
Bodies draw
at the authoritative ground position with terrain lift, in world footprint
order alongside bodies and transients. Equal-position transients follow cannon
bodies. No new flight clock or physics approximation is added by rendering.
Save/load immediately reconstructs the body from saved position/provenance;
old flights without provenance omit optional art. Rendering never changes
the sim, save bytes or RNG.

## Verification

Synthetic tests reject oversized canvases/counts, invalid ranges/rates,
overflowing starts, invalid transparency and path-like image names. They
check one-shot expiry at extreme ticks, supported CLI scales and alpha
blending without treating translucent colored pixels as ground shadows.
Retail tests compare all 116 source frames to independently decoded indices
and `ANIM.PAL` bytes, exercise cache eviction, all GI directions, same-tick
fire/death retention, active-effect bounds and unchanged sim/save bytes.
Impact regressions use the same MTNK shooter/victim type with two different
fired weapons. After killing the shooter and restoring a saved flight, each
arrival selects its own weapon's art exactly once. A moving-target sim check
also verifies original delivered damage versus reduced health loss.

The app proof test renders old/default and native views plus an old view
zoomed to match native world magnification. The latter exposes lost terrain
detail without changing unit size. A controlled EB1 target then receives an
actual attack order and a destruction command, exercising shared event
routing and compositing rather than fabricated effect instances.
The AP proof actor is the American MTNK from the corrected setup (#124).
Selecting only by the shared GUNFIRE
muzzle sheet had picked HARV instead; its real impact is PIFFPIFF (#123).
The close-up uses 4x zoom, records launch and arrival separately, compares
visible/unknown-source flights at the same gameplay hash, and verifies
identical pixels after saving the flight and rebuilding the sprite bank.
Camera centering now uses the shared terrain-aware ground projection (#129),
so the cannon fixture no longer needs a private height compensation.

```sh
cargo test -p yr-render -p yr-app effects -- --nocapture
OPENYR_VISUAL_DUMP=/tmp/openyr-native-effects cargo test -p yr-app --bin openyr retail_native_detail_and_event_effect_proof -- --nocapture
cargo run --release -- <gamedir> EB1.MMX --frames /tmp/openyr-native-skirmish 300 900 1500 2400 3000
```

The combat-depth regression fires an actual retail MTNK `105mm` at a
one-health vehicle beside a Construction Yard, once behind it and once in
front. Sampled flight/clip ticks compare opaque foreground-building pixels
to both a combat-hidden baseline and the previous overlay composition. Live unit
coverage is excluded from this building-specific mask because those units
have their own world depth. The flight, muzzle, impact and destruction all
come from simulation events. A saved flight reconstructs with a fresh sprite
bank; repeated compositions preserve the hash and complete save bytes.
Building impacts also retain their own body coverage, and event-routing
checks preserve a foundation through destruction. Missing optional art never
produces a fallback diamond.

```sh
OPENYR_COMBAT_DEPTH_PROOF=/tmp/openyr-combat-depth cargo test -p yr-app --bin openyr retail_combat_art_respects -- --nocapture
```

The exports pair the fixed compositor with a reconstruction of the old overlay
at the same tick and hash, plus JSON receipts. They demonstrate native
occlusion; they do not establish original-game layer or timing parity.

All exports contain retail art and stay outside git. No original engine
source or instructions were used. These observations come from the user's
retail SHP, palette and INI data plus the native rendering path.
