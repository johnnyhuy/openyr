# Held-key repeat proof

Source commit e9bb9f0fffd2086b9b48a1f9003620ef0d35406b, tested tree 134f6ea27985a44d4c20f3403cc8afdba92360d5.
The final branch must return to this exact tree after removal of every proof
file. Squash merge only.

Nine PNGs come from the real App compositor and native keyboard/pointer handlers
using authored Session/INI/map/SHP/TMP fixtures. JSON sidecars report cursor Kind,
selection, pending commands, tick, simulation hash and encoded Session size.
The Save name stays `Keep this name` after old held repeats; a fresh Z followed
by an ordinary repeat edits it to `Keep this namezz`. The country frame retains
the pointer-selected Russia after an old held navigation repeat.

The GIF alternates the two 640x480 Save states without changing their pixels.
Its 1400 ms duration is presentation timing, not game timing. No retail cursor,
menu artwork or original-game capture is supplied.

## Verification

```
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2
mise run check
```

Passed `cargo test --workspace --locked` (580 Rust passes, 0 failures, 1 existing
native audio-device test ignored), `cargo clippy --workspace --all-targets
--locked -- -D warnings`, `cargo fmt --all --check` and three Python provenance
tests. Local socket access was enabled for the CLI socket regressions. Retail
labelled tests return early without data; their reported passes are not retail
verification.

```
CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr --locked --no-run --message-format=json
OPENYR_WORLD_FEEDBACK_PROOF=$PWD/.tmp/modal-proof-final <recorded executable> hover_feedback::tests::ownership::modal_repeat:: --nocapture
CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/modal-runtime
python3 scripts/proof_build.py verify --receipt .tmp/modal-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/modal-runtime/receipt.json -- --help
```

Four targeted tests passed. They cover Save editing, 24 keyboard/pointer
pan/zoom resume cases, dropdown closure, focus return without synthetic events
and controlled prepared-match commits at 640x480, 800x600 and 1280x800.
Release/fresh press recovery and normal same-editor repeats remain functional.
Selection, pending valid orders, saved bytes and encoded Sessions follow their
existing transition policies. Prepared results are controlled channel values,
not retail asset loading or save reconstruction.

`test-executable.json` records the separately built debug test executable and
SHA256 used for the visual proof. `receipt.json` records the isolated release
OpenYR executable, smoked through `--help` in `runtime.typescript`; that smoke
does not execute the visual fixture. Neither executable is uploaded.

`baseline.log` records the new regression failing before the production fix:
`Keep this namez` instead of `Keep this name`. It is a test reproduction, not a
provenance-attested original-game or release capture.

## Limits

Retail data is absent at the cloud and standard Mac paths. No retail archive
scan, physical Mac/Retina input, audio-device, original-game timing/runtime or
retail sprite/cursor check ran. Known permission-blocked original/Mac capture
paths and known HTTP 401 release uploads were not retried. This uses the
user-authorized temporary proof-branch fallback. Original visual calibration
and the broader #96/#97/#98 acceptance criteria remain open.
