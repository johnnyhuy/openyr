# Initial map structure health (#346)

Authored fixtures use the public map/session setup and native shared compositor.
No retail models, art, original executable or physical window are represented.

- `frames/map-health-256-640.png`: full 100/100 green health pips.
- `frames/map-health-128-640.png`: half 50/100 yellow health pips.
- `frames/map-health-1-640.png`: low 1/100 red health pip.
- Zero health also starts at 1/100; overfull 65535 starts at 100/100.
- Each case also has a 1280x800 frame and JSON state receipt.
- These frames isolate hover health on an authored flat canvas. They contain no
  building sprite; damaged-art selection is checked through BuildingAnimation.
- Native assertions check the exact health-bar row and pip count, Attack resolver,
  both layouts, empty pending orders and complete Session-byte preservation.

`map-health.typescript` records native tests, match compatibility and simulation
bounds/repair. `test-executables.json` pins source and isolated Cargo artifacts
used for these captures; executable files are not uploaded.
`workspace-test.log` records all 636 passing Rust tests and one existing ignored
audio-device test. `clippy.log` records strict all-target clippy. The initial
mise check's CLI socket test needed local socket permissions; the full workspace
rerun passed with those permissions, without source changes. Formatting and three
proof-tool tests passed in that mise run; their output is retained separately.

Full-health supported setup retains old bytes and fingerprint. Affected old
matches reject before replacement; valid raw old simulation snapshots retain
encoded HP exactly. No migration rewrites saved maximum health.

The release receipt and help smoke establish isolated runtime provenance, not
retail gameplay. Retail data, original behavior calibration, physical Mac/Retina,
window capture and device playback were unavailable or not run. Known blocked
Mac capture and HTTP 401 release-upload paths were not retried.

All proof is pinned to its branch-only commit and removed from the final source
tree. Squash merge only. SHA256SUMS hashes every other payload file.
