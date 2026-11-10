# Authored initial map unit health (#349)

Pre-fix regression: production main 3bfb385 plus fixture-only commit 6604d77
spawns a health=128 MTNK at 100/100 instead of 50/100. `baseline.log` records
that expected failure. Its executable was not retained; do not treat that log
as a retail or original-game capture.

The final native tests use public parsed-map session setup and the shared
compositor. No retail unit sprites/models, original executable or physical
window are represented. The flat canvas and geometric fallback diamond isolate
health-status behavior; full-health neutral bodies ordinarily remain in the
static map background, so a full-health test canvas may show only status.

- `frames/map-unit-health-MTNK-256-640.png`: 100/100, 18 green pip pixels.
- `frames/map-unit-health-MTNK-128-640.png`: 50/100, 10 yellow pip pixels.
- `frames/map-unit-health-MTNK-1-640.png`: 1/100, two red pip pixels.
- E1 infantry and ORCA aircraft half-health frames verify their yellow pips.
- Half-health cases also have 1280x800 frames and state JSON.
- All 30 native cases (three types x five fractions x two layouts) assert the
  exact health row, Attack resolver, selection, pending orders and complete
  Session-byte preservation. Only eight representative frames are retained.

`map-unit-health.typescript` records five targeted tests, including parsed rows,
match compatibility, integer bounds, full-spawn byte equivalence, missing-id
no-op and actual 50-damage lethality with identical continued/restored state.
`test-executables.json` pins source tree and private-target Cargo artifacts used
for captures. The executable files are retained locally, not uploaded.
`check.log` records 641 Rust tests passed, one existing ignored audio-device test,
strict clippy, formatting and three proof-tool tests from mise run check.

The isolated runtime receipt and help smoke establish release provenance,
not retail gameplay. Unaffected old matches retain setup keys; affected old
matches reject before replacing a live session. Valid raw old simulation
snapshots retain encoded HP exactly. No schema or silent migration changed.

Retail archives are absent in this environment. No fresh retail scan/gameplay,
original runtime, physical Mac/Retina, window capture or audio-device checks ran.
Known blocked Mac capture and HTTP 401 release-upload paths were not retried.

All proof is pinned to a branch-only commit and dropped from the final source
tree before squash merging. SHA256SUMS hashes every other payload file.

Separate follow-ups #350 and #351 track ignored authored facing and baked unit
body lifetime/double composition. These health-status fixtures do not validate
retail map-body/shadow lifetime or original orientation parity.
