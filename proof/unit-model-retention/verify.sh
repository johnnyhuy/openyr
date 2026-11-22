#!/bin/bash
set -eu
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR=/workspace/openyr-core-slice-20261122-07/.tmp/model-proof-target
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
set -x
git rev-parse HEAD HEAD^{tree}
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
python3 -m unittest discover -s scripts -p 'test_*.py' -v
