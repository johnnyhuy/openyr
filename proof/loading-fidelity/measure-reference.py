"""Extract the independent normalized original glyph/badge receipts; no game code."""
import argparse
import hashlib
import json
from pathlib import Path
from PIL import Image

parser = argparse.ArgumentParser()
parser.add_argument('reference', type=Path)
parser.add_argument('flag', type=Path, help='YRII.PCX from the supplied install')
args = parser.parse_args()
assert hashlib.sha256(args.reference.read_bytes()).hexdigest() == 'c803596685de291f2d50aa36ad3762140c3a49fbdb30b43a4dc2de5f3e8da768'
reference = Image.open(args.reference).convert('RGBA')
assert reference.size == (800, 600)
cases = [
    ('loading', (20, 300, 51, 17), (248, 20, 16)),
    ('specialty', (20, 90, 23, 16), (0, 0, 0)),
    ('brief', (20, 160, 400, 34), (248, 20, 16)),
    ('country', (714, 307, 30, 21), (248, 20, 16)),
    ('player', (174, 326, 69, 17), (56, 208, 40)),
]

def digest(values):
    result = 0xcbf29ce484222325
    for value in values:
        result = ((result ^ value) * 0x100000001b3) & ((1 << 64) - 1)
    return f'{result:016x}'

receipts = {}
for name, (x, y, w, h), color in cases:
    mask = bytes(int(tuple(v & m for v, m in zip(reference.getpixel((xx, yy))[:3], (248, 252, 248))) == color)
                 for yy in range(y, y + h) for xx in range(x, x + w))
    receipts[name] = {'rect': (x, y, w, h), 'color': color, 'foreground': sum(mask), 'mask_fnv1a': digest(mask)}
flag = Image.open(args.flag).convert('RGB')
assert flag.size == (47, 23)
points = [(x, y) for y in range(23) for x in range(47) if flag.getpixel((x, y)) != (255, 0, 255)]
values = bytes(value & bitmask for x, y in points for value, bitmask in
               zip(reference.getpixel((117 + x, 323 + y)), (248, 252, 248, 255)))
receipts['badge'] = {'position': (117, 323), 'source_pixels': len(points), 'rgba_fnv1a': digest(values)}
print(json.dumps(receipts, indent=2))
