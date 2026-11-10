# Classic presentation proof

Source commit ad1e1a858cd0d8267bb68f5c845dcce739089db2; tested tree 2ec63227bde70ae310490c98682a48c71c02df6a. Retained test executables come from exact Cargo artifact paths in the successful fresh-private-target check, with SHA256 and sizes recorded in compositor-executables.json. The isolated release runtime has its own receipt and verified --help smoke; it does not render these test fixtures.

## Units and selection

unit-comparison.png places unchanged native atlas pixels side by side. Both atlases use the same authored body/turret/barrel fixture at eight facings and three house colors, not retail models. Source bounds and image anchors are identical. Before has zero fractional body pixels; after has 1,842, with no SHP shadow-marker collision. The fixture's dominant house channels exceed their base 240 in 516 before pixels and zero after pixels; peak RGB decreases from 249 to 237. Other source palette entries are retained. This demonstrates the native softer house ramp and lighting, not original VPL/normal-table parity.

The baseline is unchanged renderer production commit c9f730f plus the retained authored test patch. baseline.log contains the expected failed new coverage assertion after exporting all baseline views. The current border test separately verifies partial coverage at canvas edges without opaque clipping and preserves the ground origin.

selection/ holds three layout pairs for selected-health and hostile-hover composition. Selected units have no white outline, retain health feedback and normal cancel/reselection/cursor behavior. Full Session bytes stay exact. The white-outline slice was merged separately as PR #345. That worker's separate retail headless release is https://github.com/johnnyhuy/openyr/releases/tag/untagged-f19622c3290ef9a17009 . Four asset download sizes, decoded dimensions and SHA256 values are recorded in upstream-selection-assets.json; those runtime runs were not performed in this cloud environment.

## Buildings and menus

buildings/ holds tick 4/5 pairs at 640x480, 800x600 and 1280x800. A 5x5 authored neutral SHP sentinel advances its idle active layer without production, retaining original map-house color and the unsupported baked-art fallback. Selected unit 6 and pending Stop(6) persist. Simulation hashes change with the deliberate tick advance, while each composition preserves complete post-step Session bytes/hash. Destruction leaves no baked body. This tests live-layer routing, not a representative retail building screenshot.

menu/ holds four authored SDBTNANM phase images and synthetic audio.wav/audio-cues.tsv. Logical phases 30/34/42/43 sample held/accepted control frames 4/8/16/normal. Exactly two accepted MenuClick cues produce nonzero stereo PCM (2,940 frames at 44,100 Hz); disabled/cancelled/repeated/muted inputs remain silent. No audio device or retail sound sample was exercised. Button frame order/cadence and persistent-control pulse are native policies pending original observation; page navigation stays immediate. No departing control paints over a new page.

The GIFs preserve source RGB pixels exactly; their 400 ms presentation pacing is not a real-time recording or original cadence. Raw PNGs and receipts remain authoritative. PNG recompression is lossless. All artwork and audio here are authored fixtures; no game assets or binaries belong on main.

## Validation

After sourcing /workspace/openyr-env.sh:

- CARGO_TARGET_DIR=$PWD/.tmp/check-target CARGO_BUILD_JOBS=2 mise run check: passed. Includes cargo test --workspace --locked (632 passed, zero failed, one existing audio-device test ignored), cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check and three proof-tool tests. Expected-failure proof-tool fixtures deliberately emit compiler errors.
- OPENYR_UNIT_APPEARANCE_PROOF=$PWD/.tmp/visual-proof/units-after .tmp/programs/yr_render unit_appearance --nocapture: three tests passed, 24 model views plus ramp/border checks.
- OPENYR_BUILDING_IDLE_PROOF=$PWD/.tmp/visual-proof/buildings .tmp/programs/openyr building_idle --nocapture: two tests passed.
- OPENYR_MENU_CLICK_PROOF=$PWD/.tmp/visual-proof/menu .tmp/programs/yr_app menu_click --nocapture, then the same with .tmp/programs/openyr: two library and one native route/audio tests passed.
- OPENYR_WORLD_FEEDBACK_PROOF=$PWD/.tmp/visual-proof/selection .tmp/programs/openyr classic_status --nocapture: nine layout/zoom cases passed.
- CARGO_BUILD_JOBS=2 python3 scripts/proof_build.py build --output .tmp/presentation-runtime-ready: isolated release build passed.
- python3 scripts/proof_build.py verify --receipt .tmp/presentation-runtime-ready/receipt.json: passed.
- python3 scripts/proof_build.py run --receipt .tmp/presentation-runtime-ready/receipt.json -- --help: passed.

The final integrated check uses a private target after a shared-cache collision was detected during provisional parallel work. Superseded pre-integration checks are not used as final evidence. Retail data is absent here and retail-labelled tests self-skip. No current retail/original-game runtime, live-window, physical Mac/Retina or audio-device checks ran in this cloud environment. Known permission-blocked Mac capture paths and HTTP 401 release uploads were not retried. Fresh gameplay recording remains in #339; the separate initial map-health defect is #346. Proof is branch-only and removed before squash merge.
