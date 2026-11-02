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
| Human infantry electrocution | Lethal warhead InfDeath=5 | ELECTRO.SHP | 60x56 | 15 |

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
and per-effect depth still need original-game observation; the first pass
paints effects after world bodies and before health bars and UI.

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
depth against tall objects remain unimplemented or uncalibrated. Bodies draw
at the authoritative ground position with terrain lift, before transient
effects. No new flight clock or physics approximation is added by rendering.
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

All exports contain retail art and stay outside git. No original engine
source or instructions were used. These observations come from the user's
retail SHP, palette and INI data plus the native rendering path.
