# Allied radar presentation

The sidebar now uses `Radar=yes` from the merged rules to find an owned
radar provider. It requires the player's produced power to cover drained
power. Enemy structures and enemy power do not affect this check. Losing
power or the last provider immediately disables minimap input and begins
the closing animation.

The original [Red Alert 2 manual, radar screen section](https://static.wikia.nocookie.net/cnc_gamepedia_en/images/b/b5/Red_Alert_2_-_English.pdf/revision/latest?format=original)
describes the powered-radar requirement and click behavior. With no units
selected, a left click moves the camera; with selected units, it issues a
move order. The native sidebar follows this behavior and supports camera
dragging while no units are selected. Right click clears selection and
placement. The sandbox's existing attack-move modifier also works on radar
orders. Original YR input details still need a live comparison.

## Retail data

The installed `RULESMD.INI` declares `Radar=yes` on `GAAIRC`, `AMRADR`,
`NARADR`, `NAPSIS`, `GASPYSAT`, `CACHIG05` and `CANEWY05`. The first four
combat radar providers drain 50 power; `GASPYSAT` drains 100. The sidebar
checks the actual owned building kind, including captured foreign or tech
providers, rather than a hardcoded Allied building id. Explicit
`RadarInvisible=yes` excludes a kind from the minimap. Unspecified flags
currently mean false; stealth and detection behavior are not implemented.

`SIDEC01.MIX/RADAR.SHP` has 33 complete 168x110 canvases. Frame 0 contains
the Allied emblem. Subsequent frames flash and dissolve it; frame 32 has
an empty display. The source sheet draws at panel-relative (0,48).
The open display interior is (14,50,140,104), with a one-pixel border,
based on the [original-game screenshot](https://www.mobygames.com/game/5120/command-conquer-yuris-revenge/screenshots/windows/55949/)
used for the sidebar geometry.

All frames are decoded once through `SIDEBAR.PAL`. The presentation clock
advances one frame per two simulation ticks, reverses from its current
frame on availability changes and stops at either end. Repeated drawing
or observing the same tick does not advance it. This cadence is provisional,
not measured original timing. Fresh sessions or restored saves with active
radar start at the open endpoint. Transition phase is not saved.

## Terrain and live objects

The minimap is built from cell metadata, without shrinking the world image
or baking unit/building art into its background. The map's theater tileset
resolves each TMP and subtile. Its two RGB radar fields supply a mean
terrain color. The exact original use of the pair, elevation and lighting
remain unverified; the current mean is a presentation approximation.

Coordinates use the map's isometric cell projection. For map width `W`,
the base radar pixel for cell `(x,y)` is:

```text
rx = x - y + W - 1
ry = floor((x + y - W - 1) / 2)
```

The image crops to `[Map] LocalSize` with horizontal coordinates doubled.
EB1 has Size 60x62 and LocalSize (2,4,56,52), producing a 112x52 terrain
image. Its embedded preview is 118x61, consistent with a two-horizontal,
one-vertical projection scale for the full map. The map fits inside the
140x104 interior while preserving aspect ratio and centering letterboxing.
This establishes an internally consistent projection, not exact original
radar scaling or clipping across all maps.

The painter samples cached terrain colors, then current resource cells,
current building footprint cells and current unit positions. Owner colors
use the same rules color conversion as world sprites. Ore/gem marker colors
are provisional. Resource depletion and unit/building removal do not leave
stale art. The current sandbox has no shroud, so the minimap shows its full
playable terrain and visible sim objects. Shroud, fog, radar jamming and
SpySat reveal behavior remain future work.

A white rectangle shows the current world viewport. It uses the same
world/cell projection as camera navigation and clips to the active map
image. Input samples the same source cell as the painted destination pixel,
so letterboxing, border pixels, empty cells and transitions accept no
orders or navigation. Selected units receive normal `Command::Move` or
`Command::AttackMove` orders; UI input never edits sim positions.

## Validation and proof

Synthetic tests check presentation-clock timing, repeated observations,
mid-transition reversals, very large tick jumps, letterboxing and missing
cells. The retail test builds and places a real `AMRADR` through production,
checks all 33 source canvases pixel by pixel, and verifies ownership, enemy
power isolation, player power loss/recovery and provider destruction.
It checks every active minimap click against the actual sampled cell,
viewport clipping, live unit removal and identical active pixels after a
save/load round trip. Separate app tests check camera centering at three
zooms and that radar orders enter the command queue while preserving unit
selection. Drawing leaves sim hashes and encoded saves unchanged.

```sh
OPENYR_RADAR_DUMP=/tmp/openyr-radar-proof cargo test -p yr-app radar -- --nocapture
cargo run --release -- --load /tmp/openyr-radar-proof/active.oyrsave --frames /tmp/openyr-radar-active 697 730 --passive
```

The retail test exports opening/closing frames and saves for powered,
unpowered, recovered and provider-lost states. These are external proof
artifacts and must never enter git. Source assets remain on the user's
retail install. No original engine source or original binary instructions
were used. Original transition cadence, exact minimap colors/scale,
visibility and input behavior still require oracle observations under #86.
