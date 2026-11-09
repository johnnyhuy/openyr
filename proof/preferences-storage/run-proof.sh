#!/usr/bin/env bash
set -euo pipefail
cd /workspace/openyr-menu-recovery-20261109-1200
source /workspace/openyr-env.sh
export CARGO_TARGET_DIR="$PWD/.tmp/cargo-target"
export CARGO_BUILD_JOBS=2
printf 'Preferences storage regression proof, owned scratch files only\n'
printf 'Committed source: '
git rev-parse HEAD
printf 'Tested source tree: '
git rev-parse 'HEAD^{tree}'
printf '\nBefore fix: actual EdgeScroll route redirected a write through a preexisting symlink.\n'
sed -n '/running 1 test/,$p' /workspace/scratch/preferences-storage-proof/before.log
printf '\nAfter fix: actual native handler and private writer tests, debug profile.\n'
sha256sum .tmp/cargo-target/debug/deps/openyr-8539b05c03c1d83a
.tmp/cargo-target/debug/deps/openyr-8539b05c03c1d83a preferences_storage --nocapture
printf '\nClean committed isolated release receipt verification.\n'
python3 scripts/proof_build.py verify --receipt /workspace/scratch/preferences-storage-release/receipt.json
printf '\nVerified isolated release CLI smoke. This checks executable provenance/launch, not the preferences handler.\n'
python3 scripts/proof_build.py run --receipt /workspace/scratch/preferences-storage-release/receipt.json -- --help
printf '\nNo retail data, native window, physical Mac, Retina, audio, or original-game runtime checks were performed.\n'
