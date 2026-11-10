# Gameplay-first README review

Tested source commit df5212239cc923fbd17c8f90157747915c528063, tree 341c502a067e4debdfcdcee874f7e3e8eed26f1d. This documentation-only revision changes README order and capture provenance, with no engine/simulation code change. Media is pinned to separate branch-only commit 371bb427c434218bc571a819ff7797b3472725e7 and absent from the final source tree.

Desktop (1280x1400) and mobile (390x1000) screenshots show the README rendered from GitHub's GFM Markdown API HTML with local preview CSS. This is a local README preview, not a capture of the GitHub website's exact CSS. Playwright uses the installed /usr/bin/chromium; remote image requests are fulfilled with the exact bytes freshly downloaded from the seven public README image URLs. media-urls.json retains sizes, hashes and decoded dimensions/frame counts. The CI badge is a local placeholder. Gameplay GIFs are derived/reused earlier native gameplay captures, not a new runtime recording.

## Validation

- CARGO_TARGET_DIR=/workspace/openyr-next-unit-20261110-5/.tmp/cargo-target CARGO_BUILD_JOBS=2 mise run check, after sourcing /workspace/openyr-env.sh: passed. Includes cargo test --workspace --locked (623 Rust tests passed, zero failed, one existing audio-device test ignored), cargo clippy --workspace --all-targets --locked -- -D warnings, cargo fmt --all --check and three proof-tool tests.
- gh api markdown --input <JSON containing README text, mode=gfm, context=johnnyhuy/openyr>: passed; output retained as readme-render.html.
- script -q -e -c 'node .tmp/readme-browser.cjs' .tmp/browser.typescript: passed. Gameplay is the first section, all seven table images decode, no MP4 playback links appear, legal context remains, and mobile content stays within the viewport/table bounds. Browser screenshots were inspected.
- All three gameplay source download sizes match release metadata. All GIFs decode to multiple distinct frames: combat 36, movement 8, base operations 9. Combat conversion and source/output SHA256 values are retained in the pinned media manifest; other GIFs are byte-identical to their original captures.

Fresh current-build retail gameplay recording remains blocked in #339 because the retail install/archives are absent. No current runtime gameplay, physical Mac, Retina or audio checks were run for this documentation update. Known permission-blocked Mac captures and HTTP 401 release uploads were not retried. Browser proof and retail-derived GIFs stay outside the final branch tree and main; squash merge only.
