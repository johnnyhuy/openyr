#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-input-review-20261108-1330
source /workspace/openyr-env.sh
printf 'Source and tree\n'
git log -1 --format='%H %T'
printf 'Debug handler executable\n'
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
printf 'Native compositor and synthetic Sim picking; no complete Session or retail art\n'
OPENYR_SELECTION_BOX_PROOF=/workspace/scratch/selection-box-proof/frames mise exec -- cargo test -p yr-app --bin openyr --locked selection_box_tests -- --nocapture --test-threads=1
