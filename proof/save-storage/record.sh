#!/bin/bash
set -eu
source /workspace/openyr-env.sh
export CARGO_TERM_COLOR=never NO_COLOR=1
cd /workspace/openyr-save-safety-20261107-2150
git rev-parse HEAD HEAD^{tree}
mise exec -- python3 scripts/proof_build.py verify --receipt .tmp/save-proof-257-v2/receipt.json
mise exec -- python3 scripts/proof_build.py run --receipt .tmp/save-proof-257-v2/receipt.json -- --help
mise exec -- cargo test -p yr-app --lib --locked snapshot::storage::tests -- --nocapture --test-threads=1
sha256sum /workspace/openyr/target/debug/deps/yr_app-3d65909a3deb91c6
