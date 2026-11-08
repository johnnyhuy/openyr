#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-menu-input-20261108-0940b
source /workspace/openyr-env.sh
printf 'Source and tree\n'
git log -1 --format='%H %T'
printf 'Debug handler executable\n'
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
printf 'Synthetic native keyboard/pointer dispatch; actual temporary file; no Session or retail art\n'
OPENYR_KEYBOARD_TRACKING_PROOF=/workspace/scratch/keyboard-tracking-proof/frames mise exec -- cargo test -p yr-app --bin openyr --locked menu_keyboard_tracking -- --nocapture --test-threads=1
