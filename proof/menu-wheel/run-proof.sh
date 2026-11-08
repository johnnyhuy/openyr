#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-menu-wheel-20261108-0620
source /workspace/openyr-env.sh
printf 'Source and tree\n'
git log -1 --format='%H %T'
printf 'Debug handler test binary\n'
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
printf 'Synthetic native dispatch and shared fallback compositor, no Session or retail art\n'
OPENYR_MENU_WHEEL_PROOF=/workspace/scratch/menu-wheel-proof/frames mise exec -- cargo test -p yr-app --bin openyr --locked menu_wheel -- --nocapture --test-threads=1
