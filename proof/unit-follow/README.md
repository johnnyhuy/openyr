# F selected-unit follow proof

Source commit e7d6c2afb1168efcf1532398fadf01a6ac6662db; tested source tree 39ab2df5b3b0177eb8f1fba4fd304139c0428351.

Shared native keyboard and redraw-camera handlers follow moving owned unit 6 from raw subcell coordinates, including terrain height 4 and existing raster/oversized bounds. Authored selection deliberately includes enemy 7 to verify that it stays unchanged while F chooses only an owned target. Each layout records simulation ticks 3, 6, 9, 12 and 15. All three layouts have identical positions, simulation hashes, snapshot lengths, selection and pending Stop at each corresponding tick. Camera updates preserve complete post-step Session bytes in the tests; hashes change between ticks because the fixture deliberately advances simulation to move the unit.

At 640x480 the initial camera is (364,20), then (214,110) at tick 9. At 1280x800 the final camera clamps to (0,0), so exact centering gives way to the existing raster constraint. The target is stationary by the final capture and a no-change camera update requests no new composition.

PNGs are losslessly recompressed; original/uploaded SHA256 values are retained in comparison.json. GIFs preserve RGB pixels from ticks 3, 6, 9 and 12 exactly. Their 1400 ms presentation pacing is not runtime timing, original cadence or a live recording. These are authored Session/art/map fixtures with a colored grid and fallback units/sidebar, not retail screenshots or cursor art.

## Validation

After sourcing /workspace/openyr-env.sh:

- CARGO_TARGET_DIR=/workspace/openyr-next-unit-20261110-5/.tmp/cargo-target CARGO_BUILD_JOBS=2 mise run check: passed. Includes cargo test --workspace --locked (623 Rust tests passed, zero failed, one existing audio-device test ignored), cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check and three Python proof-tool tests. Expected-failure proof-tool fixtures deliberately emit compiler errors.
- CARGO_TARGET_DIR=/workspace/openyr-next-unit-20261110-5/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr unit_follow --locked: four tests passed.
- OPENYR_UNIT_FOLLOW_PROOF=$PWD/.tmp/unit-follow-proof script -q -e -c '.tmp/unit-follow-compositor-program unit_follow --nocapture' .tmp/compositor.typescript: four tests passed. Covers nine moving layout/zoom sequences, sixty cancellation cases, eleven keyboard guards, mixed/ineligible selection, mode/READY/team retention, actual sidebar production and READY pickup.
- CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/unit-follow-runtime: isolated release build passed.
- python3 scripts/proof_build.py verify --receipt .tmp/unit-follow-runtime/receipt.json: passed.
- python3 scripts/proof_build.py run --receipt .tmp/unit-follow-runtime/receipt.json -- --help: passed. The separate release smoke does not render the test fixture.

The retained compositor executable comes from the exact Cargo test artifact reported by the full check; its size and SHA256 are recorded. The isolated release runtime has a separate provenance receipt. F following is supported by the public manual; target choice, toggle/cancellation, guards, centering/bounds and notices are native policies pending original observation. No save/simulation fields or commands were added.

Retail data is absent and retail-labelled tests self-skip. No retail scan/runtime, original-game runtime, live-window, physical Mac, Retina or audio-device checks ran. Known permission-blocked Mac capture paths and HTTP 401 release uploads were not retried. Proof is branch-only and removed before squash merge.
