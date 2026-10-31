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
count), once. Positive `Rate` uses the existing provisional presentation
interpretation `floor(ticks * Rate / 225)`, default 200 at 15 simulation
ticks/second. Drawing never advances a clock. `Translucent=yes` defaults to
50 percent transparency, and a valid `Translucency` percentage overrides it.
Straight-alpha blending uses widened integer channels. These timings,
transparency rules, palette lighting and anchors need comparison with the
original game; source-art comparisons do not establish exact engine parity.

The cache holds at most 64 decoded effect frames and 128 clip definitions.
At most 256 effects remain active; the oldest is dropped when full. Expired
clips disappear on simulation updates even without redraws. Cache keys use
resolved source frames rather than elapsed ticks. Effects do not change sim
hashes, event schemas, snapshots or RNG. Loading resets transient effects,
so prior flashes/explosions are not replayed from a snapshot.

Warhead impact animations require accurate shot provenance and remain in
#102, along with projectile art/trails, smoke/fire, debris, environmental
animations, infantry death sequences and weapon-specific effects. Exact
destruction-list selection and original compositing remain there too.

## Verification

Synthetic tests reject oversized canvases/counts, invalid ranges/rates,
overflowing starts, invalid transparency and path-like image names. They
check one-shot expiry at extreme ticks, supported CLI scales and alpha
blending without treating translucent colored pixels as ground shadows.
Retail tests compare all 78 source frames to independently decoded indices
and `ANIM.PAL` bytes, exercise cache eviction, all GI directions, same-tick
fire/death retention, active-effect bounds and unchanged sim/save bytes.

The app proof test renders old/default and native views plus an old view
zoomed to match native world magnification. The latter exposes lost terrain
detail without changing unit size. A controlled EB1 target then receives an
actual attack order and a destruction command, exercising shared event
routing and compositing rather than fabricated effect instances.

```sh
cargo test -p yr-render -p yr-app effects -- --nocapture
OPENYR_VISUAL_DUMP=/tmp/openyr-native-effects cargo test -p yr-app --bin openyr retail_native_detail_and_event_effect_proof -- --nocapture
cargo run --release -- <gamedir> EB1.MMX --frames /tmp/openyr-native-skirmish 300 900 1500 2400 3000
```

All exports contain retail art and stay outside git. No original engine
source or instructions were used. These observations come from the user's
retail SHP, palette and INI data plus the native rendering path.
