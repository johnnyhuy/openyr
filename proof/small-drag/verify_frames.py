from pathlib import Path
import json
from PIL import Image, ImageDraw, ImageFont
root = Path(__file__).parent
names = ['01-pressed', '02-threshold-crossed', '03-dragged', '04-released']
expected = [(1000, 1000), (997, 997), (976, 976), (976, 976)]
frames = [Image.open(root / 'frames' / (name + '.png')).convert('RGBA') for name in names]
for name, camera in zip(names, expected):
    state = json.loads((root / 'frames' / (name + '.json')).read_text())
    assert state['synthetic_scene'] and not state['session_present']
    assert state['camera'] == list(camera), state
    assert state['selected'] == [17]
    assert state['pending'] == '[Stop { id: 17 }]'
assert frames[0].tobytes() != frames[2].tobytes()
assert frames[2].crop((24, 24, 472, 480)).tobytes() == frames[0].crop((0, 0, 448, 456)).tobytes()
assert frames[2].tobytes() == frames[3].tobytes()
receipt = {'synthetic_scene': True, 'session_present': False, 'diagonal_pointer_steps': 24,
           'zoom': 1, 'camera_positions': expected, 'screen_translation': [24, 24],
           'aligned_world_pixels': 448 * 456, 'selection_and_pending_preserved': True}
(root / 'frame-verification.json').write_text(json.dumps(receipt, indent=2) + '\n')
font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf', 16)
pages = []
for name, frame, camera in zip(names, frames, expected):
    canvas = Image.new('RGB', (1280, 550), '#101820')
    canvas.paste(frames[0].convert('RGB'), (0, 42))
    canvas.paste(frame.convert('RGB'), (640, 42))
    draw = ImageDraw.Draw(canvas)
    draw.text((12, 12), 'Synthetic grid: press reference (1000,1000)', font=font, fill='white')
    draw.text((652, 12), f'{name}: camera {camera}', font=font, fill='white')
    draw.text((12, 528), 'Shared app compositor. No session, retail assets or physical desktop input.', font=font, fill='white')
    pages.append(canvas)
pages[2].save(root / 'comparison.png')
pages[0].save(root / 'camera.gif', save_all=True, append_images=pages[1:], duration=[1500,1500,3500,1500], loop=0)
print(json.dumps(receipt))
