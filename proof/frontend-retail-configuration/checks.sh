#!/bin/bash
set -euo pipefail
source /workspace/openyr-env.sh
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/workspace/openyr-core-slice-20261122-07/.tmp/model-proof-target
set -x
git rev-parse HEAD HEAD^{tree}
if OPENYR_GAMEDIR=/workspace/openyr-441-intentionally-absent cargo test -p yr-app --lib frontend::tests::retail_marker_art_matches_original_sprite_and_digit_pixels --locked -- --exact --nocapture > .tmp/frontend-retail-441/missing-path.txt 2>&1; then
 echo 'ERROR: configured missing data unexpectedly passed'; exit 1
fi
python3 - <<'PY'
from pathlib import Path
s=Path('.tmp/frontend-retail-441/missing-path.txt').read_text()
assert 'running 1 test' in s and 'OPENYR_GAMEDIR is not a directory: /workspace/openyr-441-intentionally-absent' in s and '1 failed' in s
print('PASS: explicit missing data produced the expected failing assertion, not a skipped pass')
PY
env -u OPENYR_GAMEDIR cargo test -p yr-app --lib frontend::tests::retail_marker_art_matches_original_sprite_and_digit_pixels --locked -- --exact --nocapture
export OPENYR_GAMEDIR=/workspace/game-data/RA2
export OPENYR_BACKDROP_PROOF="$PWD/.tmp/frontend-retail-441/frames/backdrop"
export OPENYR_TITLE_PROOF="$PWD/.tmp/frontend-retail-441/frames/title"
export OPENYR_MARKER_PROOF="$PWD/.tmp/frontend-retail-441/frames/marker"
export OPENYR_CHOOSER_PROOF="$PWD/.tmp/frontend-retail-441/frames/chooser"
export OPENYR_SETUP_PROOF="$PWD/.tmp/frontend-retail-441/frames/setup"
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
python3 -m unittest discover -s scripts -p 'test_*.py' -v
