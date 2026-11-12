import json
from pathlib import Path
import shlex
import subprocess

root = Path.cwd()
out = Path.home() / 'openyr-proof/current-gameplay-339'
receipt = root / '.tmp/gameplay-339-build/receipt.json'
command = ['python3', 'scripts/proof_build.py', 'run', '--receipt', str(receipt), '--',
           str(Path.home() / 'Games/RA2YR/prefix/drive_c/Westwood/RA2'), 'EB1.MMX',
           '--frames', str(out / 'frames'), *map(str, range(1, 3001))]
(out / 'capture-command.json').write_text(json.dumps(command, indent=2) + '\n')
print(shlex.join(command), flush=True)
subprocess.run(command, check=True)
