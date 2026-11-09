#!/usr/bin/env bash
set -euo pipefail
source /workspace/openyr-env.sh
cd /workspace/openyr-loading-audit-20261109-0800
git rev-parse HEAD HEAD^{tree}
test -z "$(git status --porcelain)"
sha256sum /workspace/openyr/target/debug/deps/yr_app-3d65909a3deb91c6
OPENYR_SELECTION_CURSOR_PROOF=/workspace/scratch/selection-cursor-proof /workspace/openyr/target/debug/deps/yr_app-3d65909a3deb91c6 interaction::tests --nocapture --test-threads=1
cat /workspace/scratch/selection-cursor-proof/resolver.json
