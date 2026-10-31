# Contextual cursor canvases

YR resolves `MOUSE.SHA` from `RA2MD.MIX/CONQMD.MIX`; RA2 resolves
`MOUSE.SHP` from `RA2.MIX/CONQUER.MIX`. Both are ordinary TS/RA2 SHP
canvases with index 0 transparent. The YR file has 517 frames of 55x43;
the RA2 file has 450. Independent decoding shows that the first 450 YR
canvases match RA2 exactly. `MOUSEPAL.PAL` is a 768-byte palette in
`RA2.MIX/CACHE.MIX`; the sidebar and theater palettes are incorrect here.

The application reads through the VFS, preferring `MOUSE.SHA` and using
`MOUSE.SHP` only when it is absent. It accepts the observed canvas and
frame counts before decoding, bounding the RGBA cache to 4,890,820 bytes.
It reuses the hostile-header SHP parser. A malformed/missing optional
sheet or palette retains the system pointer rather than hiding it.

## Native state mapping

The following zero-based frame groups were inspected in decoded contact
sheets. Their shapes and colors support these native assignments; an
original-game action-by-action comparison is still needed under #98.

| Native state | First frame | Count | Inspected shape |
|---|---|---|---|
| Pointer | 0 | 1 | White arrow, top-left origin |
| Select | 18 | 13 | Green contracting ring |
| Move | 31 | 10 | Green rotating outward arrows |
| Blocked | 41 | 1 | Slashed movement arrows |
| Attack | 53 | 5 | Red targeting ring |
| Attack-move | 404 | 9 | Rotating red/green target |
| Repair | 170 | 20 | Silver wrench |
| Invalid repair | 190 | 1 | Slashed wrench |
| Sell | 129 | 10 | Silver dollar sign |
| Invalid sale | 149 | 1 | Slashed dollar sign |

The sheet does not encode action names, hotspots or frame durations.
OpenYR places the arrow's authored origin at the hit point and centers
radial icons at (27,21), inferred from the 55x43 canvas geometry. It plays
one frame per two simulation ticks on an absolute clock. These are explicit
provisional policies, not observed original hotspots or cadence. Drawing
never advances simulation state or consumes its RNG. Each icon keeps its
source UI-pixel size independently of map zoom.

## Input and cursor agreement

A shared resolver chooses selection or an available command. Both hover
feedback and left-click orders use it. Ownership, live entities, armed
attackers and reachable movement goals are checked. The route preview uses
the same nearest-passable goal substitution and pathfinder as simulation
movement. It ignores temporary unit occupancy; units can move or die before
an order executes. A blocked group gives no command when none can act; mixed
groups send commands only to eligible units. Previewing paths leaves the
state hash unchanged.

The [original manual](https://static.wikia.nocookie.net/cnc_gamepedia_en/images/3/3b/CCUC-RA2-pcMANOLeUS.pdf/revision/latest)
specifies left-click selection/orders and right-click cancellation. It
also describes green movement, a red slash for unavailable movement, red
targeting, the repair wrench and dollar-sign sale mode. The native input
now follows those buttons, retaining right-drag camera panning. Shift-click
toggles owned units. Ctrl+Shift gives attack-move; A remains a native shortcut.
Box dragging selects; releasing a camera drag cannot issue a world order.
The sidebar and powered radar keep their existing left-click workflows.

World picking now inverts the height lift used to draw objects. Overlapping
ground diamonds choose the frontmost matching cell. Box-selection anchors
apply that lift too. Synthetic checks reproduce the old level-4 error at
all supported map scales and check the corrected cell centre.

The software cursor draws last in the shared window/headless composition,
after tooltips and selection outlines. Frame coordinates come from the same
surface-to-pixel conversion as input. The OS pointer is hidden only while
valid art is active inside the focused game surface; letterbox regions,
leave, focus loss and shutdown restore it. Focus loss clears held keys,
modifiers, drags and command modes while preserving selection. Leaving the
window cancels pointer gestures so an outside release cannot leave a stale
click to issue later. Native dialogs before/after the event loop have no
custom pointer owner.

## Checks and remaining observations

Retail tests compare every RGBA pixel of all 517 cached frames to separate
SHP/palette decoding, the 450-frame prefix to RA2, and every phase of each
used state at its final hotspot-relative coordinates. Synthetic tests cover
bad headers/dimensions/counts, truncated payloads, bad palettes, clipping,
non-finite coordinates, pointer ownership, stable clocks, invalid routes,
weapon/ownership filtering, focus reset and cancellation.

The full #98 requirement remains open for original hotspot/cadence and
state mapping comparisons, action-specific harvesting feedback, minimap
miniatures, edge-scroll arrows, range distinctions and future menu/modality
integration. Pointer ownership and coordinate behavior still need physical
Retina/resizing/focus exercises. No original implementation code was used.
