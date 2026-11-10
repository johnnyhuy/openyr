# Configured dimensions for team centering

Baseline test commit `52ee41520b4b85cd31f85dc8780b6e5e129d708f` adds regressions over unchanged production main `d9c67d3fd7c800fccb9bfa6223968f17c244446f`. Two configured-size regressions fail; the default-size positive test passes. Fixed source commit `4ed8c4eb141a3e90b67611fcee49bb237946e2a1`, tree `03f5591286a933270fee59eae3efc095cb95bbd5`, passes all three tests.

The PNGs are actual shared resize/group-handler and `App::compose_at` outputs at simulation tick 3, using authored Sessions and fallback art. A selected unit's projected anchor is (600,320). At 640x480 the baseline camera (44,-80) uses the old 1280x800 fallback and puts the unit behind the sidebar. Fixed requests are (364,80) at 640x480, (284,20) at 800x600, and the unchanged (44,-80) at 1280x800. The selected screen anchors are the world viewport centers (236,240), (316,300) and (556,400).

These are raw centering requests before the existing redraw raster constraint. This slice changes the size lookup only, with no additional clamp or bounds policy. The camera helper retains the existing live-texture branch; this proof exercises its configured-size fallback without a Pixels surface and does not establish physical window/Retina delivery.

The comparative GIF uses baseline/fixed 640x480 frames and preserves every source RGB pixel. Its 1,400 ms frame duration is presentation pacing across separate runs. Logical 50 ms double-recall timestamps are test inputs, not original or physical runtime measurements. `comparison.json` checks the projected screen centers and equal metadata other than camera. Full encoded Session bytes are separately compared within each fixed regression; cross-run hash/length equality is not a full snapshot-byte comparison.

After sourcing `/workspace/openyr-env.sh`, with private `CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2`, `mise run check` passed:

- `cargo test --workspace --locked`: 609 passed, zero failures, one existing native audio-device test ignored. Retail-labelled tests self-skip without data and are not retail passes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `python3 -m unittest discover -s scripts -p 'test_*.py' -v`: three passed. Deliberate invalid Rust in that fixture is an expected negative provenance test.

The focused `cargo test -p yr-app --bin openyr team_centering --locked` passes all three tests: 162 size/zoom/single-or-mixed/slot cases, the default positive case, and three shared resize/compositor cases. Selection, assigned teams, zoom, pending Stop and complete encoded Sessions are preserved.

`baseline.log` records the expected one-pass/two-failure baseline from `OPENYR_TEAM_CENTERING_PROOF=$PWD/.tmp/baseline-proof CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr team_centering --locked -- --nocapture`. The baseline/fixed executable JSON files record separate retained test programs, source trees, sizes and SHA256 hashes. `fixed.typescript` records `OPENYR_TEAM_CENTERING_PROOF=$PWD/.tmp/fixed-proof script -q -e -c '.tmp/fixed-compositor-program team_centering --nocapture' .tmp/fixed.typescript`, with three passes.

The separate actual release uses `CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/team-centering-runtime`. `runtime.typescript` records `python3 scripts/proof_build.py verify --receipt .tmp/team-centering-runtime/receipt.json` and `python3 scripts/proof_build.py run --receipt .tmp/team-centering-runtime/receipt.json -- --help`. This verifies release provenance and help execution, separate from the debug compositor fixtures.

Retail data is absent at both documented paths and inspected workspace/mounts. No retail archive scan, original executable, live game window, retail sprite/cursor, physical Mac/Retina or audio-device check ran. Known Mac screenshot/recording and original-run permission blocks were not retried. Shared composition is the proof path here. Known release-upload HTTP 401 was not retried; authorized temporary PR-branch proof is removed before squash merge. Original bounds/centering and physical desktop calibration remain open under #97/#98.

`SHA256SUMS` covers payloads. Uploaded file sizes and Git blob IDs are verified against the pinned proof commit.
