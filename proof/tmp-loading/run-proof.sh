#!/usr/bin/env bash
set -euo pipefail
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR=/workspace/openyr-tmp-loading-293-20261109-parallel/.tmp/cargo-target
export CARGO_BUILD_JOBS=2
cd /workspace/openyr-tmp-loading-293-20261109-parallel
git rev-parse HEAD HEAD^{tree}
test -z "$(git status --porcelain)"
sha256sum .tmp/cargo-target/debug/deps/yr_render-2cd18ce61d9dada1
.tmp/cargo-target/debug/deps/yr_render-2cd18ce61d9dada1 tmp_loading --nocapture --test-threads=1
mise exec -- python3 scripts/proof_build.py verify --receipt .tmp/tmp-loading-release/receipt.json
mise exec -- python3 scripts/proof_build.py run --receipt .tmp/tmp-loading-release/receipt.json -- render-map /workspace/scratch/tmp-293-proof/input-valid SYNTHETIC.MAP /workspace/scratch/tmp-293-proof/valid-control.png 1
test -s /workspace/scratch/tmp-293-proof/valid-control.png
sha256sum /workspace/scratch/tmp-293-proof/valid-control.png
printf 'The receipt-verified CLI image is a separate valid map-art control. Direct API tests above exercise build_terrain recovery.\n'
