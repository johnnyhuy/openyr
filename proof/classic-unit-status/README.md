# Native selection and health proof

Source commit bf9e129fe4567c08155cff129cfd6784675819b8, tree
e96010ed4e50f67b2a2e7203b4d46335e3b86741. The final branch must return to this
exact tree after all proof files are removed. Squash merge only.

The PNGs come from the real App compositor and native input handlers using
public setup with authored INI/map/SHP/TMP bytes. Gray diamonds are fallback
entities, not retail artwork. The JSON sidecars report cursor Kind, selected
IDs, empty pending commands, tick, simulation hash and encoded Session size.
The GIF alternates the 640x480 selected/attack states without changing their
pixels. Its 1200 ms frame duration is presentation timing, not game timing.
No retail cursor sheet is supplied, so these images do not contain cursor art.

`test-executable.json` records the separately built debug test executable and
SHA256 used for these images. `receipt.json` records the isolated release
OpenYR executable. The release `--help` smoke in `runtime.typescript` verifies
that executable, not the visual fixture. Neither executable is uploaded.

## Validation

```
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2
mise run check
```

Passed: `cargo test --workspace --locked` (576 Rust passes, 0 failures, 1 existing
audio-device test ignored), `cargo clippy --workspace --all-targets --locked --
-D warnings`, `cargo fmt --all --check`, and three Python provenance tests.
The CLI socket tests require local socket access in this environment. A run in
the restricted socket sandbox failed with PermissionDenied; the same full check
passed with socket access enabled. Retail-labelled tests return early when data
is absent and their reported passes are not retail verification.

```
CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr --locked --no-run --message-format=json
OPENYR_WORLD_FEEDBACK_PROOF=$PWD/.tmp/status-proof-final <recorded test executable> hover_feedback::tests::ownership::unit_status::classic_status_tracks_selection_cursor_cancel_and_preserves_session --exact --nocapture
CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/status-runtime-final
python3 scripts/proof_build.py verify --receipt .tmp/status-runtime-final/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/status-runtime-final/receipt.json -- --help
```

The compositor matrix passes at 640x480, 800x600 and 1280x800, each at map zoom
0.5, 1 and 2. It checks exact selected corner positions shared with hover bounds,
open selection sides, visible health pips, Attack/Select cursor kinds,
right-click deselection, left-click reselection, identical encoded Session bytes
and no queued commands. Raster tests check zero/full health, damage boundaries,
world/sidebar clipping and malformed/nonfinite inputs.

## Comparison limits

The [public RA2 manual, printed pages 5-6](https://static.wikia.nocookie.net/cnc_gamepedia_en/images/3/3b/CCUC-RA2-pcMANOLeUS.pdf/revision/latest)
backs the input and cursor roles. The four public screenshots from the
[Steam store](https://store.steampowered.com/app/2229850/)
were inspected but hide interaction overlays and cannot calibrate their geometry.
Corner sizes, pip colors, thresholds, visibility and ground-relative placement
remain native policies pending original-game comparison (#97/#98).

Retail data is absent at the cloud and standard Mac paths. No retail archive
scan, retail sprite-overlap/cursor pixel check, original-game execution,
physical Mac/Retina input or audio-device check ran. Known permission-blocked
original/Mac capture paths and known HTTP 401 release uploads were not retried.
This uses the authorized temporary proof-branch fallback.
