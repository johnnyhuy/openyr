#!/usr/bin/env bash
set -euo pipefail
source /workspace/openyr-env.sh
cd /workspace/openyr-core-camera-20261109-0200
git rev-parse HEAD HEAD^{tree}
test -z "$(git status --porcelain)"
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
OPENYR_CAMERA_KEY_PROOF=/workspace/scratch/camera-key-proof/frames /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a camera_tracking --nocapture --test-threads=1
