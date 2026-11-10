# Supported map-unit lifetime (#351)

Authored native shared-compositor fixtures, not retail gameplay or a live
window recording. A remapped asymmetric VXL vehicle, directional SHP infantry
and SHP aircraft share a small authored clear-terrain patch. The cyan diamond
is the intentionally retained unsupported-art static fallback, not a live unit.

`frames/640-healthy.png`, `640-damaged.png`, `640-moved.png`, `640-restored.png`,
`640-owned.png` and `640-dead.png` record the state sequence. Equivalent
1280x800 frames and per-state JSON are retained. `640-overlap.png` checks two
neutral SHP units with coincident shadows. In the dead frames, all supported
bodies/shadows are gone while unsupported static art remains by policy.

Native assertions compare each visible unit's body, fractional coverage and
shadow pixels against the independently sampled background and decoded source
art. They check original-house colors after movement/load, owner colors after
reassignment, one darkening for overlapping shadows, unchanged pending input
and exact Session bytes for every draw. Original and moved neighborhoods must
return to background after removal. A missing sprite bank has marker fallback.
No death sheet is supplied in these fixtures; finite death effects and original
death-animation behavior are not established by these captures.

`unit-lifetime.typescript` records four native tests and a metadata/save test.
`test-executables.json` pins source and copied private Cargo artifacts.
Executable files remain local. `check.log` records 650 Rust tests passed,
zero failures, one existing ignored audio-device test, strict clippy, fmt and
three proof-tool tests through mise run check. Retail cases self-skip.

`state-comparison.json` records eight identical parent/current state-and-input
receipts from the existing unit-health scenario. `baseline-executables.json`
pins the retained parent-main test artifact and its SHA256. Each comparison
includes every JSON field, such as simulation hash, snapshot byte length, tick,
selection, pending commands and cursor. This is a state comparison, not an
image comparison or a claim that receipt hashes are full saved-file hashes.
The separate metadata test proves exact match bytes and setup keys are
unaffected by adding/removing house identity; movement/load tests roundtrip
complete current saves. No schema or compatibility migration is introduced.

Live/background eligibility is checked per type at eight native directions
and the initial voxel pose, with bounded SHP dimensions/frames and decodable
bodies/shadows. Unsupported/theater-only art stays static, including its old
lifetime limitations. Future animation poses, retail performance, original
palette/compass/depth/cadence and physical input remain unverified.

The release receipt/help smoke proves isolated runtime provenance, not game
playability. Retail data is absent. No retail scan/current gameplay, original
runtime, physical Mac/Retina, live window or audio-device check ran. Known
blocked Mac capture and HTTP 401 release uploads were not retried.
Proof stays in a pinned branch-only commit and is removed before squash merge.
SHA256SUMS hashes every other payload file.
