#!/usr/bin/env bash
set -eu
cd /workspace/openyr-core-input-20261109-1430
source /workspace/openyr-env.sh
printf 'Authored native Session placement-mode regression and separate release provenance smoke\n'
git rev-parse HEAD HEAD^{tree}
printf '\nBefore-fix actual command leak (recorded expected test failure):\n'
tail -18 .tmp/placement-mode-proof/before-fix.log
printf '\nCurrent native debug handler executable and hash:\n'
program=$(cat .tmp/placement-test-program)
stat -c '%s bytes %n' "$program"
sha256sum "$program"
printf '\nActual native handler/compositor proof, authored fixtures without retail art:\n'
OPENYR_PLACEMENT_MODE_PROOF="$PWD/.tmp/placement-mode-proof" "$program" --exact hover_feedback::tests::ownership::placement::placement_mode_native_compositor_proof --nocapture
printf '\nFresh-click command receipt (intents before stepping the simulation):\n'
cat .tmp/placement-mode-proof/03-ordinary-move-intent.json
printf '\nSeparate isolated release provenance and real CLI launch:\n'
python3 scripts/proof_build.py verify --receipt .tmp/placement-mode-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/placement-mode-runtime/receipt.json -- --help
printf '\nFull workspace validation summary:\n'
python3 - <<'SUMMARY'
from pathlib import Path
import re
s=Path('.tmp/final-check.log').read_text()
r=[tuple(map(int,x)) for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',s)]
print('Rust passed/failed/ignored:',tuple(map(sum,zip(*r))))
for line in s.splitlines():
    if 'Ran 3 tests' in line or 'Finished in' in line:
        print(line)
SUMMARY
printf '\nRetail absent; no live-window, physical Mac/Retina, audio or original-game checks ran.\n'
