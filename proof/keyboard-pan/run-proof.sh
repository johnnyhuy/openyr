#!/usr/bin/env bash
set -euo pipefail
source /workspace/openyr-env.sh
cd /workspace/openyr-menu-review-20261109-0400
git rev-parse HEAD HEAD^{tree}
test -z "$(git status --porcelain)"
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
OPENYR_KEYBOARD_PAN_PROOF=/workspace/scratch/keyboard-pan-proof/frames /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a surface_input::tests::keyboard_pan --nocapture --test-threads=1
