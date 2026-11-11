# Held sidebar tab ownership proof

Source 5a0df600ff28fc0bbe36a3ad680b45ac6c94868d, tree d0fce36f0815ddf561c424f9621e0ff7d0772460. This fixes #356 under #86/#97/#98.

`frames/held.png` and `frames/released.png` are 640x480 captures from actual native pointer handlers and the shared compositor. They use authored checker terrain and fallback entity/UI art, with no retail bytes. Held tab ownership removes prospective enemy brackets/health and advertises Pointer; release restores Attack/target hover without submitting an order. JSON receipts retain exact selection, queued Stop, camera, sim hash and snapshot size. The tests compare complete encoded Session bytes, not merely these summary fields.

`sidebar-ownership.typescript` records the retained native executable running `sidebar_tab_ownership --nocapture` with OPENYR_SIDEBAR_OWNERSHIP_PROOF set. Two tests pass: twelve select/attack layout/zoom sequences, six edge cases and 24 world/sidebar wheel events, plus released click/pan/zoom recovery. `test-executable.json` records the exact Cargo artifact from the workspace check log, size and SHA256. Binaries stay outside git.

`baseline.log` records both new regression tests failing against unchanged production code from main 47de98f, with only the test additions present. The failures show leaked Select and edge-scroll cursors. `baseline-executable.json` records that retained test artifact and the test source hash. This is a test-only baseline build, not an isolated clean runtime receipt.

`check.log` records CARGO_TARGET_DIR=$PWD/.tmp/check-target CARGO_BUILD_JOBS=2 mise run check: cargo test --workspace --locked passed 655 tests, zero failures, one existing audio-device test ignored; cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check and three Python proof-tool tests passed. The compiler error inside the negative proof fixture is expected and its containing Python test passed.

`runtime.typescript` and `receipt.json` record isolated release provenance verification and --help smoke. Help is not gameplay. No current retail scan/gameplay, original runtime, live window, physical Mac/Retina or audio-device check ran. Known retail paths and the workspace/mount archive search found no game data. Known blocked Mac capture/original-run and release-upload paths were not retried. This uses the authorized branch-only proof fallback, with all proof removed from the final source tree before squash merge.

No simulation, save schema, binding, camera speed or timing changes are introduced. Original gesture calibration and physical delivery remain open. SHA256SUMS covers all bundle payloads except itself. No binaries or saves are uploaded.
