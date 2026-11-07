from pathlib import Path
import re
import textwrap
from PIL import Image, ImageDraw, ImageFont
root = Path(__file__).parent
font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf', 15)
frames = []
for filename, title in [('before-guard.log', 'Before guard: actual failing regression output'), ('terminal.typescript', 'After guard: recorded native-dispatch tests and executable receipt')]:
    text = root.joinpath(filename).read_text().replace('\r', '')
    text = re.sub(r'\x1b\[[0-9;]*[A-Za-z]', '', text)
    if filename.endswith('typescript'):
        text = text[text.index('3e5be5a'):text.index('Script done')]
    rows = [part for line in text.splitlines() for part in (textwrap.wrap(line, 135, replace_whitespace=False, drop_whitespace=False) or [''])]
    for start in range(0, len(rows), 34):
        image = Image.new('RGB', (1280, 720), '#101820')
        draw = ImageDraw.Draw(image)
        draw.text((24, 18), f'{title} ({start // 34 + 1})', font=font, fill='#7fc8ff')
        for index, row in enumerate(rows[start:start+34]):
            draw.text((24, 58 + index * 19), row, font=font, fill='#e0e8ef')
        frames.append(image)
frames[-1].save(root / 'terminal.png')
frames[0].save(root / 'terminal.gif', save_all=True, append_images=frames[1:], duration=6000, loop=0)
