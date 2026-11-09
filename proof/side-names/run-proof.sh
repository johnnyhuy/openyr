#!/usr/bin/env bash
set -eu
cd /workspace/openyr-side-names-20261109-2000
source /workspace/openyr-env.sh
printf 'Provided retail PNG integrity (not a cloud retail render):\n'
python3 - <<'PY'
from pathlib import Path
import json,hashlib,struct
p=Path('.tmp/side-names-proof')
for e in json.loads((p/'author-release-metadata.json').read_text())['assets']:
    b=(p/e['name']).read_bytes()
    assert len(b)==e['size']
    assert 'sha256:'+hashlib.sha256(b).hexdigest()==e['digest']
    assert b[:8]==b'\x89PNG\r\n\x1a\n'
    print(e['name'],len(b),e['digest'],'PNG dimensions',struct.unpack('>II',b[16:24]))
PY
printf '\nIsolated mix-dump release provenance:\n'
python3 scripts/proof_build.py verify --receipt .tmp/side-names-runtime/receipt.json
printf '\nActual CLI names commands on authored MIX fixtures:\n'
python3 .tmp/side-names-proof/make-fixtures.py
python3 .tmp/side-names-proof/verify-cli.py
printf '\nLocked workspace/provenance results are retained in full-check.log. Retail scans were not rerun here.\n'
