# Loose sidebar override proof

Source 72c15bf7c240b676edba8a6d57e6103499f1f15c, tree b90be2db29b9f356e317a2648d2d102378b3a5d2. This implements the existing #86 loose per-file criterion and fixes scoped optional-art geometry defect #358.

Eight 640x480 frames come from the native app's shared compositor with authored flat-color SHP/PAL/MIX fixtures and fallback entities/terrain. They contain no retail game bytes. Allied/Soviet archive frames use the selected side's red/blue first tab; sheet frames show a magenta loose replacement, palette frames show its cyan loose recoloring, and invalid frames show bounded fallback while the valid green neighboring tab stays in place. Other mounted archives deliberately contain misleading sidebar names, which must not supply the selected-side art.

`sidebar-overrides.typescript` records three native override tests and the loose-only VFS lookup test running from exact retained Cargo artifacts identified in `test-executables.json`. Tests cover both side geometries and two layouts, loose sheet/palette precedence, unaffected neighboring sheets, absent/malformed selected archives, malformed/dimension/count/hostile-canvas overrides, invalid palettes, all four tab boundaries and complete encoded Session preservation. Binaries stay outside git.

`geometry-baseline.log` records the intermediate malformed-art regression before the geometry correction: the Soviet tab-1 expected crop receives panel background instead of its green source pixels. The loose-loader implementation and new tests were already present; this is not a clean isolated old-main runtime or an original-game comparison.

`check.log` records CARGO_TARGET_DIR=$PWD/.tmp/check-target CARGO_BUILD_JOBS=2 mise run check: cargo test --workspace --locked, cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check and three Python proof-tool tests. The expected Rust compiler error inside a negative Python fixture is not a workspace failure. The validation summary records final totals.

`receipt.json` and `runtime.typescript` retain isolated release build provenance, verification and --help smoke. Help is not gameplay. No current retail scan/gameplay, original runtime, live window, physical Mac/Retina or audio-device check ran. The known retail paths and workspace/mount archive search found no game data. Previously blocked Mac capture/original-run and release-upload paths were not retried; this uses the authorized branch-only proof fallback. Every proof file is removed before squash merge, and the final tree must equal tested source.

Loose overrides are authoritative; invalid optional sheets are omitted, while an invalid palette leaves the safe fallback UI. Global font and ordinary VFS lookup, simulation and save encoding are unchanged. Remaining original loader exceptions, Yuri and physical/original fidelity work remain open. SHA256SUMS covers every payload except itself. No binaries or saves are uploaded.
