#!/usr/bin/env bash
set -eu
cd /workspace/openyr-core-order-20261109-1800
source /workspace/openyr-env.sh
printf 'Initial seed failure on main before the fix:\n'
tail -18 /workspace/scratch/save-catalog-before.log
printf '\nExact private debug native-handler executable:\n'
cat .tmp/save-catalog-proof/debug-executable.json
printf '\nNative Session and metadata regressions:\n'
.tmp/cargo-target/debug/deps/openyr-8539b05c03c1d83a save_catalog_ --nocapture
printf '\nSeparate isolated release receipt and CLI smoke:\n'
python3 scripts/proof_build.py verify --receipt .tmp/save-catalog-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/save-catalog-runtime/receipt.json -- --help
