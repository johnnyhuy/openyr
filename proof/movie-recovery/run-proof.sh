#!/usr/bin/env bash
set -euo pipefail
source /workspace/openyr-env.sh
cd /workspace/openyr-core-audit-20261109-0600
git rev-parse HEAD HEAD^{tree}
test -z "$(git status --porcelain)"
sha256sum /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a
OPENYR_MOVIE_RECOVERY_PROOF=/workspace/scratch/movie-recovery-proof/frames /workspace/openyr/target/debug/deps/openyr-8539b05c03c1d83a movie_failure_recovery --nocapture --test-threads=1
