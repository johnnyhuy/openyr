# Command chords stay outside world keyboard actions

Baseline test commit `2f52808885b3ba1f374091c68ccb6bff52cbe7c9` adds regressions to unchanged production main `d6eba84dae2d1ac1d09eedcbf40597b414888e87`. One positive menu/plain-command test passes and three regressions fail. Fixed source commit `fbb5a66226ab1bfc8c15efb618c8545063df5d32`, tree `3330fe106ae892a43c9171e03ab3619306f216a3`, passes all four tests and the full workspace.

The PNGs are actual native keyboard-dispatch and `App::compose_at` captures at simulation tick 3. An authored Session has selected unit 6, a pending Move, an armed attack-move mode and a left-drag gesture. Baseline Command+S appends Stop, clears the mode/gesture and displays a Stop notice. Fixed Command+S leaves the Move, mode, gesture, camera and selection unchanged at 640x480, 800x600 and 1280x800. Commands are captured before any simulation step; the baseline's unchanged simulation hash does not make its extra pending Stop harmless.

Baseline/fixed before frames have identical RGBA bytes. Each fixed before/after pair also has identical RGBA bytes and recorded metadata. The GIF compares common before, baseline after and fixed after from separate runs, preserving all source RGB pixels. Its 1,400 ms pacing is comparative presentation, not a recording of physical input or original-game timing.

All art is authored fallback content: gray entities, a flat background, a drag rectangle and numbered sidebar tabs. No retail font, cursor or artwork is used. `comparison.json` checks command/client-state differences and equal camera, tick, hash, selection and snapshot length. Full encoded Session bytes are asserted within each fixed regression; cross-run hash/length equality is not a full snapshot-byte comparison.

After sourcing `/workspace/openyr-env.sh`, with private `CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2` and local socket permission for the CLI tests, `mise run check` passed:

- `cargo test --workspace --locked`: 613 passed, zero failures, one existing native audio-device test ignored. Retail-labelled tests self-skip without data and are not retail passes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `python3 -m unittest discover -s scripts -p 'test_*.py' -v`: three passed. Deliberate invalid Rust in that fixture is an expected negative provenance test.

The four focused tests cover 60 layout/key/Command(+Control/Shift) cases, 27 actual save/preference/menu/camera cases, pan/zoom repeats after modifier release and fresh recovery, plain A/S/K/L, Ctrl+H, and menu Command+A replacement. Protected disk files are authored sentinel bytes, not retail saves; valid Session encoding is checked separately. The existing preference-error fixture now verifies rejected Command+M and its repeat before a fresh allowed M reaches the real writer and modal recovery, retaining its cleanup and protected-file assertions.

`baseline.log` records the expected one-pass/three-failure baseline from `OPENYR_COMMAND_WORLD_PROOF=$PWD/.tmp/baseline-proof CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2 cargo test -p yr-app --bin openyr command_world --locked -- --nocapture`. Baseline/fixed executable JSON files record the retained private test programs, source trees, sizes and SHA256 hashes. `fixed.typescript` records `OPENYR_COMMAND_WORLD_PROOF=$PWD/.tmp/fixed-proof script -q -e -c '.tmp/fixed-compositor-program command_world --nocapture' .tmp/fixed.typescript`, with four passes.

The separate release uses `CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/command-world-runtime-final`. `runtime.typescript` records `python3 scripts/proof_build.py verify --receipt .tmp/command-world-runtime-final/receipt.json` and `python3 scripts/proof_build.py run --receipt .tmp/command-world-runtime-final/receipt.json -- --help`. This verifies release provenance and help execution, separate from debug compositor fixtures.

Retail data is absent at documented paths and inspected workspace/mounts. No original executable, retail archive scan, live game window, retail sprite/cursor, physical Mac/Retina or audio-device check ran. Known Mac screenshot/recording and original-run permission blocks were not retried. Shared composition is the proof path here. Known release-upload HTTP 401 was not retried; authorized temporary PR-branch proof is removed before squash merge. Command isolation is native keyboard policy; pointer/wheel routing and original calibration remain outside this change under #97/#98.

`SHA256SUMS` covers payloads. Uploaded file sizes and Git blob IDs are verified against the pinned proof commit.
