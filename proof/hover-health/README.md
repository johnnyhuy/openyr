# Hover health proof

Source commit 2600996db486c0533af9405187cfdfa6e6b4b2aa; tested source tree 04fade3f60f5bd50e341ca35651ce15f89b2640f.

Native shared composition shows the existing health pips on resolved healthy hover targets and removes them after pointer movement away. Five cases at each of three layouts cover owned selection, hostile units, sell, full-health unavailable repair, and hostile structures. All captures are simulation tick 4. Fifteen receipt pairs retain target, camera/zoom, selection, pending Stop, simulation hash and snapshot length. The tests also compare complete encoded Session bytes. PNGs are losslessly recompressed without resizing; comparison.json records original and uploaded SHA256 values. GIF frames preserve source RGB pixels exactly. Their 1400 ms pacing is presentation only, not runtime timing or a live recording.

These are authored Session/map/art fixtures with fallback units and sidebar. No retail cursor sheet or game screenshot is supplied. The compositor executable was copied from the exact Cargo test artifact reported by the successful full check; its size and SHA256 are recorded. The separate isolated release runtime is verified and exercised with --help, not used to render the fixture.

## Validation

After sourcing /workspace/openyr-env.sh:

- CARGO_TARGET_DIR=/workspace/openyr-next-unit-20261110-5/.tmp/cargo-target CARGO_BUILD_JOBS=2 mise run check: passed. Includes cargo test --workspace --locked (619 Rust tests passed, zero failures, one existing audio-device test ignored), cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check and three Python proof-tool tests. Negative proof-tool fixtures intentionally emit compilation errors and pass their expected-failure assertions.
- CARGO_TARGET_DIR=/workspace/openyr-next-unit-20261110-5/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr hover_health --locked: two tests passed. On unchanged production code, the healthy-hover regression fails and the prior visibility/guard regression passes; baseline.log retains this result.
- OPENYR_HOVER_HEALTH_PROOF=$PWD/.tmp/hover-health-proof script -q -e -c '.tmp/hover-health-compositor-program hover_health --nocapture' .tmp/compositor.typescript: two tests passed, covering 45 layout/zoom/target cases, thirteen guards, persistent target isolation and prior selected/damaged visibility.
- CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/hover-health-runtime: isolated release build passed.
- python3 scripts/proof_build.py verify --receipt .tmp/hover-health-runtime/receipt.json: passed.
- python3 scripts/proof_build.py run --receipt .tmp/hover-health-runtime/receipt.json -- --help: passed.

Health visibility remains a documented native classic-style policy pending original-game observation. The cursor resolver, geometry, thresholds, selection, orders, simulation and save schemas are unchanged. Retail data is absent and retail-labelled tests self-skip. No original-game runtime, retail scan/runtime, live-window, physical Mac, Retina or audio-device checks ran. Known permission-blocked Mac capture paths and HTTP 401 release uploads were not retried. Proof is branch-only and must be removed before squash merge.
