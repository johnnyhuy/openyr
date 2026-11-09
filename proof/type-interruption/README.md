# Type double-press interruption proof

Fixed source commit ba20f0a16149e023b19b1497e0b2b0f79c026323, tested tree 860e97903249f3af7f7cf194379b53a8349fbef0. Remove all
proof files and restore this exact tree before squash merge. Neither executable
is uploaded; hashes identify the retained local copies.

Baseline test commit a4ea3d13ea7b9f243da79f918caa3178a3df1969 adds only the regression module to
production base 411e64788eb1b5f48eebe26705e88eaefa9311df. Its clean committed
private-target test build reports one pass and three expected failures in
`baseline-test.log`. `baseline-executable.json` records that executable/hash.
The fixed private-target test executable is recorded in `test-executable.json`.

The baseline/fixed 640x480 frames come from the actual App compositor and native
H handler with authored Session/INI/map/SHP/TMP fixtures. T, H, T selects three
units before the fix, adding an offscreen match, and two viewport matches after
the fix. `comparison.json` verifies identical camera, pending commands, tick,
simulation hash and encoded Session size between the two runs. Fixed frames
also cover 800x600 and 1280x800. Gray diamonds are fallback entities; no retail
art or cursor sheet is used.

The second T has an injected logical time of 50 ms after the first; composition
uses logical time 60 ms. These control the native test window and notice, not
original-game runtime. The GIF preserves the two 640x480 frames pixel for pixel;
its 1400 ms duration is presentation timing. Both captures are at simulation
tick 5. Fixed regressions verify complete Session byte invariance; sidecars
record the hash and encoded length rather than full snapshot files.

## Validation

```
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2
mise run check
```

Passed `cargo test --workspace --locked` (594 Rust passes, 0 failures, 1 existing
native audio-device test ignored), `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo fmt --all --check` and three Python provenance
tests. Local socket access was enabled for CLI regressions. Retail-labelled
tests return early without data; their reported passes are not retail checks.

```
CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr --locked --no-run --message-format=json
OPENYR_TYPE_INTERRUPTION_PROOF=$PWD/.tmp/final-proof <recorded executable> hover_feedback::tests::ownership::type_interruption:: --nocapture
CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/interruption-runtime
python3 scripts/proof_build.py verify --receipt .tmp/interruption-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/interruption-runtime/receipt.json -- --help
```

All four targeted tests pass after the fix: 18 fresh-key sequences, 12 accepted
pointer/wheel sequences, three tracked T chords and seven positive guard cases.
Ordinary consecutive T still expands globally. Repeats, focus-sync input,
rejected clicks/wheels and pointer motion retain their guarded behavior.
Selection adds no commands and preserves camera and complete Session bytes.

`receipt.json` records the isolated release OpenYR executable, verified through
`--help` in `runtime.typescript`. That release smoke does not execute the visual
fixture. The baseline failure and final capture commands are also recorded in
their respective executable metadata.

## Limits

The interruption set and unchanged 400 ms interval are native policies pending
original calibration. No original executable was run or inspected. Retail data
is absent in this environment at the standard cloud and Mac paths. No retail
archive scan, retail sprite/cursor pixels, physical Mac/Retina, audio-device or
original-game runtime/timing check ran. Known permission-blocked original/Mac
captures and HTTP 401 release uploads were not retried. This uses the authorized
temporary proof-branch fallback. Broad #97/#98 criteria remain open.
