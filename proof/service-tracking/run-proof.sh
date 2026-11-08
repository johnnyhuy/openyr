#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-core-review-20261109-0100
source /workspace/openyr-env.sh
printf 'Source and tree\n'
git log -1 --format='%H %T'
printf 'Debug handler executable\n'
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
printf 'Native pointer/keyboard dispatch and shared fallback composer; no Session or retail art\n'
OPENYR_SERVICE_TRACKING_PROOF=/workspace/scratch/service-tracking-proof/frames mise exec -- cargo test -p yr-app --bin openyr --locked service_mode_pan_cleanup -- --nocapture --test-threads=1
