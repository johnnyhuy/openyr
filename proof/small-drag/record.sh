#!/bin/bash
set -eu
source /workspace/openyr-env.sh
export CARGO_TERM_COLOR=never NO_COLOR=1
cd /workspace/openyr-pointer-gesture-20261108-0010
git rev-parse HEAD HEAD^{tree}
mise exec -- python3 scripts/proof_build.py verify --receipt .tmp/small-drag-proof-259/receipt.json
mise exec -- python3 scripts/proof_build.py run --receipt .tmp/small-drag-proof-259/receipt.json -- --help
OPENYR_SMALL_DRAG_PROOF=/workspace/scratch/small-drag-proof/frames mise exec -- cargo test -p yr-app --bin openyr --locked surface_input::tests::drag -- --nocapture --test-threads=1
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
