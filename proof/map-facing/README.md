# Map unit facing (#350)

These are authored format/model fixtures, not retail screenshots or original
runtime captures. The asymmetric VXL vehicle has a differently colored nose;
infantry uses a nonzero Ready start and directional SHP body/shadow frames;
aircraft uses a directional SHP without a sequence. The small native images
are pixel receipts, not a gameplay showcase.

`frames/VEH-0-map.png` and `frames/VEH-0-live.png` show static map composition
and the live decoded sprite for heading zero. Equivalent E1/ORCA pairs and
64/128/192 headings are retained. `cases.txt` records all 39 render cases.
The tests compare every opaque live body pixel at its ground anchor and every
SHP shadow pixel against static composition on visible nonblack terrain.
Fractional voxel edge colors and original-game parity are not claimed by those
opaque-pixel assertions. Opposite headings produce different authored sprites.
Invalid Ready and short-body cases verify the correct fallback shadow.

`map-facing.typescript` records four tests: 39 renderer cases plus short/invalid
SHP fallback; 48 public setup units (three types x sixteen raw-field cases);
and save compatibility/default-spawn/missing-id behavior. Complete match bytes
roundtrip without changing health, ownership, ids, positions or orders.
`test-executables.json` pins the source tree and copied private Cargo artifacts.
Executable files remain local and are not uploaded.

The existing native convention is zero along cell -y, 64 +x, 128 +y and 192 -x.
The map byte now enters that convention and static unit previews share the live
screen-octant/voxel adapters. This changes old direct-angle voxel and frame-zero
SHP previews. Original map compass interpretation/model front axes remain
uncalibrated; this proves native consistency. Unit body lifetime, map-house
remap and stale/double composition remain #351.

`check.log` records 645 Rust tests passed, zero failures, one existing ignored
audio-device test, strict clippy, formatting and three proof-tool tests through
mise run check. Retail-labelled cases self-skip without installed archives.
The isolated release receipt/help smoke establishes runtime provenance, not
retail gameplay. Affected old match setup keys reject before replacement;
unaffected heading-128 maps and raw old snapshots retain their saved state.

Retail data is absent. No original runtime, retail scan/gameplay, physical
Mac/Retina, live window or audio-device checks ran. Known blocked Mac capture
and HTTP 401 release-upload paths were not retried. Proof is pinned to its
branch-only commit and removed from the final source tree before squash merge.
SHA256SUMS hashes every other payload file.
