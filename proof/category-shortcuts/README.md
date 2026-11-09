# Classic sidebar category shortcuts

Source commit `987a9b1efa429d288184d2b78364b3533ae143b8` implements #323, a slice of #86/#97. Source tree `bc7a0107ff00a31aec23939b8a8275ccc3040a0a` is retained after proof removal.

`structures.png`, `defences.png`, `infantry.png` and `vehicles.png` are actual `App::compose_at` outputs after native Q/W/E/R keyboard dispatch at simulation tick 3, 640x480. The four-frame GIF contains exactly those RGB pixels. Its one-second frame duration is presentation pacing, not a measurement of input or original animation timing. Each accompanying JSON records the selected units, pending command, simulation hash and snapshot length.

These are authored Session/INI/SHP/TMP fixtures, numbered fallback tabs, blank fallback cameos and a solid authored background. They contain no retail artwork. The mouse/key test compares complete framebuffer bytes at 640x480, 800x600 and 1280x800. It proves routing to the current category compositor, not original skin fidelity. Full encoded Session bytes are compared in the tests, not inferred from snapshot lengths.

The original Yuri's Revenge manual, printed page 3, establishes Q/W/E/R as Structures/Defence/Infantry/Vehicles. The downloaded PDF is 635,927 bytes, SHA256 `359694c4f42cf241dad9a0dc283edbbd95322f79581a8cc55b58485099c502e4`. [Public manual](https://gamepedia.cursecdn.com/cnc_gamepedia_en/8/8e/Red_Alert_2_-_Yuri%27s_Revenge_-_English.pdf). Original mode, modifier, repeat and transition calibration remain open.

The check log records `mise run check`, with `CARGO_TARGET_DIR=$PWD/.tmp/cargo-target CARGO_BUILD_JOBS=2`, after sourcing `/workspace/openyr-env.sh`. Its tasks passed:

- `cargo test --workspace --locked`: 598 passed, zero failures, one existing native audio-device test ignored. Retail-labelled tests self-skip without installed data and are not retail passes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `python3 -m unittest discover -s scripts -p 'test_*.py' -v`: three passed. Deliberate compilation errors in the provenance fixture are expected negative tests.

`compositor-executable.json` records the private Cargo test executable's hash, size and source tree. Its retained copy runs four `sidebar_keys` tests with `OPENYR_CATEGORY_KEYS_PROOF=$PWD/.tmp/category-proof`, recorded by `script -q -e -c '.tmp/category-compositor-program sidebar_keys --nocapture' .tmp/category-proof.typescript`. This is a debug test executable exercising actual handlers/composition, separate from the release program below.

`runtime-receipt.json` records the isolated actual release OpenYR build from `CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/category-runtime-final`. `runtime.typescript` records receipt verification and execution through `python3 scripts/proof_build.py run --receipt .tmp/category-runtime-final/receipt.json -- --help`. This smoke check verifies the release program and provenance, not a retail match or the visual test fixture.

Retail data is absent at both documented paths and the inspected workspace/mounts. No retail archive scan, retail sprite/cursor comparison, original executable, live game window, physical Mac/Retina input or audio-device check ran. Known Mac Screen Recording and original-run permission blocks were not retried. The shared compositor is the proof route here. Known release-upload HTTP 401 was not retried; these files use the authorized temporary PR-branch fallback and are removed from the final tree before squash merge.

`SHA256SUMS` covers all payload files. Uploaded Git blob IDs and sizes are checked against the pinned commit. The source tree must match the tested tree after every proof file is removed.
