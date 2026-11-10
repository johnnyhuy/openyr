# Battlefield selection with P

Source commit `77d217a7c8b2404ec7fe8d494fea8fb057861094`, source tree `c063e66f78e75ee1f3332ae73acf58fcc36731aa`, implements scoped #325 under #97.

The before/after PNGs are actual keyboard-handler and `App::compose_at` captures at simulation tick 8, at 640x480, 800x600 and 1280x800. The fixture intentionally starts with owned, enemy, neutral, dead, building and stale IDs in selection to exercise pruning. P replaces that set with living owned IDs [6,8,9], including the distant harvester. Two selected units are visible; the third is offscreen at zoom 1. The two-frame 640x480 GIF preserves every source RGB pixel; 1,400 ms frame duration is presentation pacing, not original timing.

All artwork is authored fallback content: a flat background, gray entity diamonds, numbered sidebar tabs and no retail font/cursor. The test uses a full authored Session with public setup and snapshot encoding. `comparison.json` checks equal tick, hash, camera, pending commands and snapshot length across each pair; the Rust assertions separately compare full encoded Session bytes. These are native routing/composition checks, not original/retail sprite parity.

The [original Yuri's Revenge manual, printed page 5](https://gamepedia.cursecdn.com/cnc_gamepedia_en/8/8e/Red_Alert_2_-_Yuri%27s_Revenge_-_English.pdf) establishes P and battlefield scope. Ownership/role filtering, modifier/mode/repeat policy, notice wording/lifetime and selection audio triggering remain native policies pending controlled original comparison.

After sourcing `/workspace/openyr-env.sh`, with `CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2`, `mise run check` passed:

- `cargo test --workspace --locked`: 602 passed, zero failures, one existing native audio-device test ignored. Retail-labelled tests self-skip without data and are not retail passes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `python3 -m unittest discover -s scripts -p 'test_*.py' -v`: three passed. Deliberate invalid Rust output is an expected negative provenance test.

The focused `cargo test -p yr-app --bin openyr all_selection --locked` passed all four tests. It covers nine layout/zoom cases, twelve mode/gesture/team cases, twelve input guards, menu text, single/empty feedback, retained READY work, and exact Session invariance.

`compositor-executable.json` records the copied private Cargo test executable, source tree, size and SHA256. `compositor.typescript` records `OPENYR_ALL_SELECTION_PROOF=$PWD/.tmp/all-selection-proof script -q -e -c '.tmp/all-selection-compositor-program all_selection --nocapture' .tmp/compositor.typescript`. This debug test executable exercises the actual handlers/compositor and is distinct from the isolated release program.

The separate release uses `CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/all-selection-runtime`, followed by `python3 scripts/proof_build.py verify --receipt .tmp/all-selection-runtime/receipt.json` and `python3 scripts/proof_build.py run --receipt .tmp/all-selection-runtime/receipt.json -- --help`. Its receipt and runtime recording establish source/executable provenance and a help smoke check, not a retail match.

Retail data is absent at both documented directories and the inspected workspace/mounts. No retail archive scan, original executable, retail sprite/cursor, live game window, physical macOS/Retina or audio-device check ran. Known Mac screenshot/recording and original-run permission blocks were not retried. The shared compositor is the proof path here. Known release-upload HTTP 401 was not retried; the authorized temporary PR-branch fallback is used and every proof file is removed before squash merge. `SHA256SUMS` covers payloads; remote file sizes and Git blob IDs are verified against the pinned proof commit.
