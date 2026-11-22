#!/bin/bash
set -eu
source /workspace/openyr-env.sh
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
set -x
git rev-parse HEAD HEAD^{tree}
git status --porcelain
rustc --version
cargo test --target-dir .tmp/model-proof-target -p yr-render --lib unit_model --locked -- --nocapture
