# Visible type selection proof

Source commit 76e3953ed4acce3d600b164d4d6dffe1966d8c35, tested tree be44591d013f49e43728160d747e5a566dcc8256. Remove every proof
file and return to this exact tree before squash merge. Neither executable is
uploaded; receipts and hashes describe the retained local copies.

The six PNGs use the real App compositor and native keyboard handler with
public setup and authored Session/INI/map/SHP/TMP fixtures. Gray diamonds are
fallback entities, not retail artwork. T expands the one selected owned unit
to its visible matching buddy. Enemy, unrelated-type and offscreen candidates
remain unselected. JSON sidecars record selected IDs, pending commands, tick,
simulation hash and encoded Session size.

The GIF preserves the two 640x480 PNG states pixel for pixel at simulation tick
6. Its 1400 ms duration is presentation timing, not game timing. Captures
also cover 800x600 and 1280x800. No retail cursor sheet or game artwork is used.

## Validation

```
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2
mise run check
```

Passed `cargo test --workspace --locked` (584 Rust passes, 0 failures, 1 existing
native audio-device test ignored), `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo fmt --all --check`, and three Python provenance
tests. Local socket access was enabled for the CLI regressions. Retail-labelled
tests return early without data; their reported passes are not retail checks.

```
CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr --locked --no-run --message-format=json
OPENYR_WORLD_FEEDBACK_PROOF=$PWD/.tmp/type-proof-final <recorded executable> hover_feedback::tests::ownership::type_selection:: --nocapture
CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/type-runtime
python3 scripts/proof_build.py verify --receipt .tmp/type-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/type-runtime/receipt.json -- --help
```

Four targeted tests pass: visible expansion across three layouts and zooms 1,
2 and 4; 18 right/bottom-edge cases at zoom 0.25, 1 and 4; mixed kinds, retained
offscreen seeds and invalid-ID pruning; old-gesture/mode cleanup with queued
Stop preservation; repeat/focus/synthetic/modifier/outcome/missing-data guards;
and normal T text in Save. Exact encoded Session bytes and camera positions
remain unchanged by selection.

`test-executable.json` identifies the private-target debug test executable used
for the compositor proof. `receipt.json` identifies the isolated release OpenYR
executable, verified through `--help` in `runtime.typescript`. That smoke does
not execute the visual fixture.

## Source and limits

The public [RA2 manual, Basic Keyboard Functions, printed page 1](https://static.wikia.nocookie.net/cnc_gamepedia_en/images/3/3b/CCUC-RA2-pcMANOLeUS.pdf/revision/latest)
maps T to Type select. It does not establish the detailed selection policies.
Viewport-only expansion, retained offscreen seeds, mixed-kind behavior, count
notice timing and selection audio remain native choices pending original
comparison. A second T repeats viewport expansion; all-map expansion is not
implemented in this slice. Two other public hotkey-guide pages returned HTTP
403 and were not used as behavior evidence.

Retail data is absent at the cloud and standard Mac paths. No retail archive
scan, original-game execution/timing, retail sprite/cursor, physical Mac/Retina
or audio-device check ran. Known permission-blocked original/Mac capture paths
and HTTP 401 release uploads were not retried. This uses the user-authorized
temporary proof-branch fallback. The broader #97/#98 calibration stays open.
