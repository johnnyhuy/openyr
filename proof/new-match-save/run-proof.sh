#!/usr/bin/env bash
set -eu
cd /workspace/openyr-menu-load-20261109-1600
source /workspace/openyr-env.sh
printf 'Initial before-fix F5 regression:\n'
tail -20 .tmp/new-match-save-proof/before-fix.log
printf '\nActual native debug-handler executable and tests:\n'
python3 - <<'PY'
from pathlib import Path
import hashlib,json,subprocess
record=json.loads(Path('.tmp/new-match-save-proof/debug-executable.json').read_text())
program=Path(record['path'])
assert program.stat().st_size==record['bytes']
assert hashlib.sha256(program.read_bytes()).hexdigest()==record['sha256']
print(json.dumps(record,indent=2),flush=True)
subprocess.run([str(program),'new_match_save_target','--nocapture'],check=True)
PY
printf '\nSeparate isolated release provenance and CLI launch:\n'
python3 scripts/proof_build.py verify --receipt .tmp/new-match-save-runtime/receipt.json
python3 scripts/proof_build.py run --receipt .tmp/new-match-save-runtime/receipt.json -- --help
