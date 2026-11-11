# Sidebar category press proof

Source 793d2bff30745e34e529d16d432e77de3d959f60, tree 035e96d9edc03dc1bdebc8246be3afe270130966.

The seven 640x480 frames come from the native app's shared compositor and actual pointer handlers, using original test-only flat-color SHP/MIX sheets. They contain no retail artwork and are not gameplay or original-game parity evidence. The second tab is inactive in 00, held in 01, selected while the pointer is outside in 02, held again on reentry in 03, and normally selected after release in 04. Frames 05/06 show cancellation and no rearming on reentry.

The same test verifies exact interior source-color crops for all four tabs, both initially selected/inactive press variants, and Allied 28x27 / Soviet 32x28 geometry at 640x480 and 1280x800. Five cancellation routes and missing-art fallback are checked separately. Full encoded Session bytes, queued orders, selection and camera remain unchanged by the press sequences.

`sidebar-press.typescript` records the retained native test executable running `sidebar_tab_press --nocapture` with OPENYR_SIDEBAR_PRESS_PROOF set to the frame directory. `test-executable.json` identifies the exact Cargo artifact from the full workspace check log and its retained size/SHA256. The executable itself stays outside git.

`check.log` records `CARGO_TARGET_DIR=$PWD/.tmp/check-target CARGO_BUILD_JOBS=2 mise run check`: cargo test --workspace --locked (653 passed, zero failed, one existing audio-device test ignored), cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check, and three Python proof-tool tests passed. The compiler failure in the Python negative fixture is expected; its containing test passed.

`runtime.typescript` and `receipt.json` record an isolated release build verification and --help smoke check. This establishes executable provenance, not gameplay. No current retail archive scan, physical Mac/Retina, original-game runtime, live-window or audio-device check ran. The known retail paths and workspace/mount archive search found no game data. Known blocked Mac capture/original-run and release-upload paths were not retried; this uses the authorized branch-only proof fallback.

Press variant choice, immediate activation and reentry behavior are native policies pending original comparison. Hover/disabled policies and broader #86 criteria remain open. SHA256SUMS covers every bundle payload except itself. No binaries or saved games are included.
