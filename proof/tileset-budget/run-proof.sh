#!/usr/bin/env bash
set -euo pipefail
source /workspace/openyr-env.sh
cd /workspace/openyr-core-audit-20261109-1000
git rev-parse HEAD HEAD^{tree}
test -z "$(git status --porcelain)"
sha256sum /workspace/openyr/target/debug/deps/yr_formats-0e772dec0ccadab9 /workspace/openyr/target/debug/deps/yr_render-ddfd1ae686f8de65 /workspace/openyr/target/debug/deps/yr_app-3d65909a3deb91c6
/workspace/openyr/target/debug/deps/yr_formats-0e772dec0ccadab9 tileset:: --nocapture --test-threads=1
/workspace/openyr/target/debug/deps/yr_render-ddfd1ae686f8de65 tileset_budget --nocapture --test-threads=1
/workspace/openyr/target/debug/deps/yr_app-3d65909a3deb91c6 tileset_budget --nocapture --test-threads=1
mise exec -- python3 scripts/proof_build.py verify --receipt .tmp/tileset-budget-release/receipt.json
set +e
mise exec -- python3 scripts/proof_build.py run --receipt .tmp/tileset-budget-release/receipt.json -- render-map /workspace/scratch/tileset-budget-proof/input SYNTHETIC.MAP /workspace/scratch/tileset-budget-proof/rejected.png > /workspace/scratch/tileset-budget-proof/cli-error.txt 2>&1
result=$?
set -e
cat /workspace/scratch/tileset-budget-proof/cli-error.txt
test "$result" -eq 1
test ! -e /workspace/scratch/tileset-budget-proof/rejected.png
printf 'Verified render-map exit code %s with no PNG output.\n' "$result"
