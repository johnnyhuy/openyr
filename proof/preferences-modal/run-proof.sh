#!/usr/bin/env bash
set -eu
cd /workspace/openyr-menu-flow-20261109-next
source /workspace/openyr-env.sh
printf 'Initial single-case regression before the fix:\n'
tail -18 /workspace/scratch/preferences-modal-before.log
printf '\nFinal debug native-handler executable:\n'
cat .tmp/preferences-modal-proof/debug-executable.json
printf '\nActual native Session recovery tests:\n'
.tmp/cargo-target/debug/deps/openyr-8539b05c03c1d83a preferences_error_modal --nocapture
printf '\nSeparate isolated release provenance and CLI launch:\n'
python3 scripts/proof_build.py verify --receipt .tmp/preferences-modal-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/preferences-modal-runtime/receipt.json -- --help
