"""Convert the unbroken tick export after capture has completed."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess

root = Path.cwd()
base = Path.home() / 'openyr-proof/current-gameplay-339'
frames = base / 'frames'
proof = root / 'proof/current-gameplay-339'
proof.mkdir(parents=True, exist_ok=True)
commands = []

def run(args):
    command = ['ffmpeg', '-hide_banner', '-y', *args]
    commands.append(command)
    subprocess.run(command, check=True)

# Frame 1 is the initial state; audio starts with the transition to tick 2.
run(['-framerate', '15', '-start_number', '2', '-i', str(frames / 'frame-%05d.png'),
     '-i', str(frames / 'audio.wav'), '-frames:v', '2999', '-c:v', 'libx264',
     '-crf', '18', '-preset', 'medium', '-pix_fmt', 'yuv420p', '-c:a', 'aac',
     '-b:a', '192k', '-movflags', '+faststart', str(proof / 'full-session.mp4')])
# Select intervals only after inspecting the source frames.
for name, first, count, tab in [('placement', 500, 120, ''), ('assault', 1400, 180, '-vehicles')]:
    run(['-framerate', '15', '-start_number', str(first), '-i', str(frames / f'frame-%05d{tab}.png'),
         '-frames:v', str(count), '-filter_complex',
         '[0:v]scale=640:400:flags=lanczos,split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=sierra2_4a',
         '-loop', '0', str(proof / f'{name}.gif')])
for tick, tab in [(2, '-vehicles'), (540, ''), (560, ''), (1500, '-vehicles'), (2480, ''), (3000, '')]:
    name = f'frame-{tick:05d}{tab}.png'
    run(['-i', str(frames / name), '-frames:v', '1', '-update', '1', str(proof / name)])
shutil.copy2(frames / 'audio-cues.tsv', proof)
shutil.copy2(base / 'capture.typescript', proof)
shutil.copy2(base / 'capture-command.json', proof)
shutil.copy2(base / 'window-smoke.log', proof)
shutil.copy2(root / '.tmp/gameplay-339-build/receipt.json', proof / 'build-receipt.json')
shutil.copy2(root / '.tmp/gameplay-339/capture.py', proof)
for name in ['test', 'clippy', 'fmt', 'proof-tests', 'retail-scan', 'outcome']:
    shutil.copy2(root / f'.tmp/gameplay-339/{name}.log', proof)
(proof / 'conversion-commands.json').write_text(json.dumps(commands, indent=2) + '\n')
manifest = {}
for path in sorted(proof.iterdir()):
    if path.is_file():
        manifest[path.name] = {'bytes': path.stat().st_size, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
(proof / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
