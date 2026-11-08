#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-selection-review-20261109-0010
source /workspace/openyr-env.sh
printf 'Source and tree\n'
git log -1 --format='%H %T'
printf 'Debug picker test executable\n'
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
printf 'Native picker component against synthetic Sim positions and movement; no complete Session or retail assets\n'
OPENYR_SUBCELL_SELECTION_PROOF=/workspace/scratch/subcell-selection-proof/receipts mise exec -- cargo test -p yr-app --bin openyr --locked selection_box_tests -- --nocapture --test-threads=1
