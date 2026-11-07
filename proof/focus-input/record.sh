#!/bin/bash
set -eu
source /workspace/openyr-env.sh
export CARGO_TERM_COLOR=never NO_COLOR=1
cd /workspace/openyr-menu-input-20261108
git rev-parse HEAD HEAD^{tree}
mise exec -- python3 scripts/proof_build.py verify --receipt .tmp/focus-proof-20261108/receipt.json
mise exec -- python3 scripts/proof_build.py run --receipt .tmp/focus-proof-20261108/receipt.json -- --help
mise exec -- cargo test -p yr-app --bin openyr --locked keyboard_input::tests -- --nocapture --test-threads=1
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
