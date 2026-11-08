#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-menu-scroll-20261108-agent
source /workspace/scratch/menu-agent-env.sh
printf 'Source commit and tree:\n'
git log -1 --format='%H %T'
printf '\nIsolated release executable receipt verification:\n'
mise exec -- python3 scripts/proof_build.py verify --receipt .tmp/save-identity-proof-265-final-b7ef55a/receipt.json
printf '\nReceipt-covered CLI smoke only (no retail data):\n'
mise exec -- python3 scripts/proof_build.py run --receipt .tmp/save-identity-proof-265-final-b7ef55a/receipt.json -- --help
printf '\nDebug handler/compositor tests, separate from the release executable receipt:\n'
OPENYR_SAVE_IDENTITY_PROOF=/workspace/scratch/save-identity-proof-265/frames mise exec -- cargo test -p yr-app --bin openyr --locked menu_flow::tests::save_identity -- --nocapture --test-threads=1
printf '\nExact handler test executable hash:\n'
sha256sum .tmp/target-menu-scroll/debug/deps/openyr-8539b05c03c1d83a
printf '\nLimits: No Session or retail artwork; no actual retail overwrite/reconstruction, live-window or physical Mac/Retina input check. The tests verify actual temporary-file sibling bytes and exact confirmation/Commit intent.\n'
