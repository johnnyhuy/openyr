from PIL import Image
from pathlib import Path
import json
root=Path('/workspace/scratch/current-targets-20261106/live');rows=[]
for w,h in [(1280,800),(640,480)]:
 cx=(w-168)//2;cy=h//2-30;states={}
 for name in ['unselected','selected-after-notice-lifetime','pointer-outside','unfocused','focus-restored-no-motion','deselected']:
  im=Image.open(root/f'{w}x{h}-{name}.png').convert('RGB');assert im.size==(w,h)
  mask={(x,y) for y in range(cy-40,cy+40) for x in range(cx-40,cx+40) if im.getpixel((x,y))==(255,64,64)}
  states[name]=mask
 for name in ['unselected','unfocused','deselected']:assert len(states[name])==0,(w,name,len(states[name]))
 visible=states['selected-after-notice-lifetime'];assert len(visible)==44
 assert visible==states['pointer-outside']==states['focus-restored-no-motion']
 left=min(x for x,y in visible);right=max(x for x,y in visible);top=min(y for x,y in visible);bottom=max(y for x,y in visible)
 assert (right-left,bottom-top)==(40,20)
 expected=set()
 for x,dx in [(left,1),(right,-1)]:
  for y,dy in [(top,1),(bottom,-1)]:
   for step in range(6):expected.add((x+dx*step,y));expected.add((x,y+dy*step))
 assert visible==expected
 rows.append(dict(size=[w,h],region=[cx-40,cy-40,cx+40,cy+40],observed_bounds=[left,top,right,bottom],state_red_pixels={name:len(mask) for name,mask in states.items()},visible_masks_identical=True,complete_corner_pattern=True))
(root/'receipt.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps(rows))
