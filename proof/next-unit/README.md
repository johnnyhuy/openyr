# N creation-order selection proof

Source commit e0383ddca7e067eb279e732d40b33114ff5dc055; tested tree 5fc83eda10cf3efa67bed853091cbf9d3bc5204e.

The native keyboard dispatch and shared compositor produce four phases at 640x480, 800x600 and 1280x720: units 6, 8, 9, then 6. All frames are simulation tick 8. Unit 9 is an offscreen harvester. At 640x480 its camera reaches the map boundary. JSON receipts verify selection, notice, camera and unchanged simulation hash, snapshot length and pending Stop command. The tests additionally compare full encoded Session bytes.

cycle.gif preserves every decoded RGB pixel of the 640-wide PNGs. Its 1400 ms presentation pacing is not a runtime timing measurement or live recording. These are authored fixtures with fallback art, not retail game screenshots.

## Validation

After sourcing /workspace/openyr-env.sh:

- CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 mise run check: 617 Rust tests passed, zero failed, one existing audio-device test ignored; cargo test --workspace --locked, cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check and three Python proof-tool tests passed.
- OPENYR_NEXT_UNIT_PROOF=$PWD/.tmp/next-unit-proof script -q -e -c '.tmp/next-unit-compositor-program next_unit --nocapture' .tmp/compositor.typescript: four targeted tests passed; executable digest is recorded separately.
- CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/next-unit-runtime: isolated release build passed.
- python3 scripts/proof_build.py verify --receipt .tmp/next-unit-runtime/receipt.json: passed.
- python3 scripts/proof_build.py run --receipt .tmp/next-unit-runtime/receipt.json -- --help: passed. This separate release smoke does not exercise the fixture.

The manual establishes N and creation order. Stable unit IDs, filters, mixed/empty selection policy, bounded camera, input guards, cleanup and notification are documented native policies, not original-game calibration. M remains mute.

Retail data is absent. Retail-labelled tests self-skip. No retail runtime, original-game runtime, physical Mac, Retina, audio-device or live-window checks ran. Known blocked Mac capture paths and release uploads returning HTTP 401 were not retried. Proof is branch-only and removed before squash merge.
