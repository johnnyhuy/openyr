#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-core-input-20261108-1130
source /workspace/openyr-env.sh
printf 'Source and tree\n'
git log -1 --format='%H %T'
printf 'Debug handler executable\n'
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
printf 'Synthetic native dispatch and shared compositor; command intents only; no Session or retail art\n'
OPENYR_ATTACK_MOVE_KEY_PROOF=/workspace/scratch/attack-move-proof/frames mise exec -- cargo test -p yr-app --bin openyr --locked keyboard_input::tests::attack_move -- --nocapture --test-threads=1
