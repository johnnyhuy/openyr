#!/usr/bin/env python3
import json
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parent
expected = json.loads((root / "expected-names.json").read_text())
for name, entries in expected.items():
    command = [sys.executable, "scripts/proof_build.py", "run", "--receipt",
               ".tmp/side-names-runtime/receipt.json", "--", "names", str(root / name)]
    result = subprocess.run(command, capture_output=True, text=True, check=True)
    print(result.stdout, end="")
    actual = {int(match[0], 16): match[1] for match in
              re.findall(r"^0x([0-9a-f]{8})\s+\d+\s+([^\n]+)$", result.stdout, re.M)}
    assert actual == dict(entries), (name, actual, entries)
    print(f"{name}: {len(actual)}/{len(entries)} literal IDs resolved exactly. Authored metadata only.")
