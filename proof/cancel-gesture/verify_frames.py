from pathlib import Path
import json
from PIL import Image, ImageDraw, ImageFont
root = Path(__file__).parent
names = ['01-box-drag', '02-right-pressed', '03-right-released', '04-late-left-release']
images = [Image.open(root / 'frames' / (name + '.png')).convert('RGBA') for name in names]
states = [json.loads((root / 'frames' / (name + '.json')).read_text()) for name in names]
white = [sum(pixel == (255,255,255,255) for pixel in image.getdata()) for image in images]
assert white[0] > 0 and white[1:] == [0,0,0]
assert images[1].tobytes() == images[2].tobytes() == images[3].tobytes()
for i,state in enumerate(states):
    assert state['synthetic_scene'] and not state['session_present']
    assert state['pending'] == '[Stop { id: 17 }]'
    if i: assert state['left_down'] is None and not state['radar_drag']
assert states[0]['selected'] == states[1]['selected'] == [17]
assert states[2]['selected'] == states[3]['selected'] == []
receipt={'synthetic_scene':True,'session_present':False,'rubber_band_pixels':white,
         'cancelled_frames_identical':True,'queued_sentinel_unchanged':True,
         'selection_clears_on_original_right_release_policy':True}
(root/'frame-verification.json').write_text(json.dumps(receipt,indent=2)+'\n')
font=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf',16)
pages=[]
for name,image,count in zip(names,images,white):
    canvas=Image.new('RGB',(1280,550),'#101820');canvas.paste(images[0].convert('RGB'),(0,42));canvas.paste(image.convert('RGB'),(640,42))
    draw=ImageDraw.Draw(canvas);draw.text((12,12),'Synthetic canvas: unfinished box drag',font=font,fill='white');draw.text((652,12),f'{name}: {count} band pixels',font=font,fill='white');draw.text((12,528),'Actual shared compositor. No session, entities, retail art or physical input.',font=font,fill='white');pages.append(canvas)
pages[1].save(root/'comparison.png')
pages[0].save(root/'cancel.gif',save_all=True,append_images=pages[1:],duration=[1500,2000,1500,1500],loop=0)
print(json.dumps(receipt))
