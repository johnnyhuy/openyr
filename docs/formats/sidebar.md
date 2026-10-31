# Allied sidebar art

The sandbox uses the Americans. Its UI reads `SIDEC01.MIX` through the
normal VFS, then resolves the following names inside that archive. It does
not mount every faction archive into the world namespace. In this install
`SIDEC01.MIX` comes from `RA2.MIX`; its palette is `SIDEBAR.PAL`.

## Retail observations

The local SHP headers and decoded canvases contain:

| Sheet | Canvas | Frames | Use |
|---|---|---|---|
| `CREDITS.SHP` | 168x16 | 1 | Credit strip |
| `TOP.SHP` | 168x32 | 1 | Top surround |
| `RADAR.SHP` | 168x110 | 33 | Allied emblem and radar transition |
| `SIDE1.SHP` | 168x69 | 1 | Repair/sell and category surround |
| `SIDE2.SHP` | 168x50 | 1 | Two empty cameo pockets |
| `SIDE3.SHP` | 168x26 | 1 | Bottom cap |
| `TAB00.SHP` through `TAB03.SHP` | 28x27 | 5 each | Structure, defence, infantry, vehicle buttons |
| `GCLOCK2.SHP` | 60x48 | 55 | Production-clock coverage |

Index zero is transparent. The first clock frame is empty. Frame 1 covers
2,868 pixels, frame 27 covers 1,444, and frame 54 covers 60. Coverage
strictly decreases from frame 1 through 54. Thus frame 0 is the clear
completion state; treating all 55 frames as a single increasing coverage
loop reverses construction progress. The clock colour in `SIDEBAR.PAL`
is blue; it identifies coverage, rather than a colour to paint onto the
cameo. The current compositor darkens covered pixels to one third of the
original RGB. The exact original translucent lookup remains unverified.

The visible layout was observed in this
[original-game screenshot](https://www.mobygames.com/game/5120/command-conquer-yuris-revenge/screenshots/windows/55949/).
It is a 1024x768 Allied session with a 168-pixel right panel. Coordinates
below are relative to the panel's left edge and the top of the screen:

| Element | Position |
|---|---|
| Credit strip | (0, 0) |
| Top surround | (0, 16) |
| Radar | (0, 48) |
| Category surround | (0, 158) |
| Four category buttons | (26 + 28*n, 197) |
| Cameo columns | x = 23, 85 |
| Cameo rows | y = 228 + 50*n |

The two-pixel gutters remain outside the cameo hit boxes. `SIDE2.SHP`
repeats behind the grid. `SIDE3.SHP` caps the screen bottom. The layout
uses native UI pixels independently of map zoom. The screenshot supplies
layout evidence, not animation timing or an exact framebuffer oracle.
No original source code was consulted or copied.

## Presentation and verification

The window and `--frames` use the same panel, coordinates and queue state.
The clock selects frame `1 + floor(done * 54 / total)` while unfinished,
and frame 0 when complete. Whole queue fractions drive it, so credit
stalls and low-power progress use the actual production state. Drawing
never advances the clock or modifies the sim. A paid queue head keeps its
normal brightness beneath the clock even when the balance is below its
full cost. Unavailable unqueued entries dim as before.

Selected tabs use sheet frame 1 and other tabs frame 0. A green underline
keeps a paid structure visible on a hidden category. The `READY` label,
placement outline and other status presentation remain provisional.
Credits, queue counts, READY and hover tooltips now use the retail
[Unicode bitmap font](font.md) with measured widths. Localized names,
costs and duration estimates remain available by hovering a cameo.

`retail_sidebar_matches_source_pixels_and_all_clock_phases` independently
decodes the source SHPs. It compares nontransparent top, radar, credit,
button, empty-row and bottom-cap pixels at their actual drawn coordinates.
It checks every coverage pixel of all 55 masks, then every cameo pixel at
all 54 unfinished phases of a real EB1 production queue. It also checks
credit-stall progress, clipped hit boxes and presentation-only state.
Existing sidebar tests exercise all 22 production ids, category changes,
affordability, factory loss, cancellation and paid placement.

```sh
OPENYR_RETAIL_UI_DUMP=/tmp/openyr-sidebar-clock cargo test -p yr-app --lib sidebar -- --nocapture
OPENYR_SIDEBAR_DUMP=/tmp/openyr-sidebar-panels cargo test -p yr-app --test retail sidebar -- --nocapture
cargo run --release -- --frames /tmp/openyr-sidebar-session 1 11 100 300 900 1500 2400
```

Dumps contain retail artwork and stay outside git. The optional loader
requires the observed dimensions and frame counts before decoding, keeping
UI memory bounded. Malformed sheets are skipped independently. Without
retail UI art the panel retains basic fallback drawing and input.

## Remaining comparison work

The inactive Allied emblem is currently displayed even after a radar
structure is built. A live minimap, power meter, repair/sell actions and
icons, top-strip controls, the bottom command bar, exact text spacing and colors,
hover/pressed/disabled tab transitions, queue hold behavior and dynamic
sidebar scrolling remain UI fidelity work. Other factions and loose
per-file sidebar overrides are not yet selected by this Allied loader.

The production-clock mask order and geometry are retail-verified. The
exact original progress rounding, darkening table and tooltip appearance
still need an original-game observation. Infantry, building and HVA
cadence, factory doors and overlay depth remain tracked in #6. This slice
does not establish full UI or animation parity.
