#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-save-list-keys-20261108-0745
source /workspace/openyr-env.sh
printf 'Source and tree\n'
git log -1 --format='%H %T'
printf 'Debug handler executable\n'
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
printf 'Synthetic native keyboard dispatch; actual temporary files; no Session or retail art\n'
OPENYR_LIST_KEYS_PROOF=/workspace/scratch/list-keys-proof/frames mise exec -- cargo test -p yr-app --bin openyr --locked menu_flow::tests::list_keys -- --nocapture --test-threads=1
mise exec -- cargo test -p yr-app --lib --locked slot_navigation_bounds_large_deltas -- --nocapture
