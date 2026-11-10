# Command-modified team chords

Baseline test commit `9be4f43db040f725251c3d02d13d925aedddd0a8` adds regressions to unchanged main production code `34632707e7949e8f489ce7de907325fe49b677aa`. Three tests fail and the positive classic-shortcut test passes. Fixed source commit `c8732feff89486ad9bec7367ca0df8202069630d`, tree `eac8df81e01247762ea4a8cd25384abaf9d6b214`, passes all four.

The PNGs are actual shared group-handler/compositor outputs at simulation tick 3, using authored Session/INI/SHP/TMP fixtures, gray fallback entity shapes and a flat background. No retail art is used. Keyboard-dispatch tests cover 108 layout/slot/chord combinations and nine empty-selection assignment cases. The camera/compositor test calls the shared group handler with logical 50 ms chord and 100 ms next-recall timestamps; these do not measure original or physical keyboard timing.

At 640x480, baseline Command+1 changes camera (444,110) to (44,-80), while the fixed chord retains (444,110) and clears old recall timing/notice. Selection, tick, hash, pending orders and snapshot length stay equal. Fixed 800x600 and 1280x800 cases also preserve their cameras. Full encoded Session bytes are compared inside the fixed regressions, not inferred from lengths or claimed equal across separate baseline/fixed runs.

Baseline and fixed 640-before frames have identical RGBA bytes. The GIF compares that common before frame, baseline after, and fixed after, preserving every source RGB pixel. Its 1,400 ms frame duration is presentation pacing across separate runs, not a runtime recording or original timing calibration.

The baseline's (44,-80) result also exposes separate #328: without a live Pixels surface, ordinary team centering uses the 1280x800 fallback rather than configured compact dimensions. This slice prevents unsupported Command chords from entering that path; it does not fix ordinary centering geometry. No physical native-window mismatch is established by that no-surface fixture.

After sourcing `/workspace/openyr-env.sh` and setting private `CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2`, `mise run check` passed:

- `cargo test --workspace --locked`: 606 passed, zero failures, one existing native audio-device test ignored. Retail-labelled tests self-skip without data and are not retail passes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `python3 -m unittest discover -s scripts -p 'test_*.py' -v`: three passed. Deliberate invalid Rust in the provenance fixture is an expected negative test.

`baseline-capture.log` records the expected one-pass/three-failure baseline from `OPENYR_TEAM_COMMAND_PROOF=$PWD/.tmp/baseline-proof CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr team_command --locked -- --nocapture`. `baseline-executable.json` records its copied executable hash, size, commit and tree.

`fixed-executable.json` records the final private test executable's hash, size and source tree. `fixed.typescript` records `OPENYR_TEAM_COMMAND_PROOF=$PWD/.tmp/fixed-proof script -q -e -c '.tmp/fixed-compositor-program team_command --nocapture' .tmp/fixed.typescript`, with four passing tests. All nine slots retain Ctrl assignment, Shift addition, plain/double recall, ignored repeat/synthetic guards and menu number text.

The separate actual release program uses `CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/team-command-runtime`. `runtime.typescript` records `python3 scripts/proof_build.py verify --receipt .tmp/team-command-runtime/receipt.json` and `python3 scripts/proof_build.py run --receipt .tmp/team-command-runtime/receipt.json -- --help`. This establishes release provenance and a help smoke check, separate from debug compositor fixtures.

Retail data is absent at both documented directories and inspected workspace/mounts. No retail archive scan, original executable, live game window, retail sprite/cursor, physical Mac/Retina or audio-device check ran. Known Mac screenshot/recording and original-run permission blocks were not retried. Shared composition is the proof route. Known release-upload HTTP 401 was not retried; the authorized temporary PR-branch fallback is used and every proof file is removed before squash merge. Unsupported Command isolation is native policy, not a claim about original modifier semantics. #97/#98 retain broader calibration and #328 retains the separate dimension defect.

`SHA256SUMS` covers payloads. Uploaded file sizes and Git blob IDs are verified against the pinned proof commit.
