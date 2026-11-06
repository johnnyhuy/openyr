import ctypes,ctypes.util,json,os,re,subprocess,time
from pathlib import Path
from PIL import Image
out=Path('/workspace/scratch/placement-feedback-20261106/live');out.mkdir(exist_ok=True)
env=dict(os.environ,DISPLAY=':111',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME='/workspace/scratch/placement-feedback-20261106/cache')
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
xt.XTestFakeMotionEvent.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong];xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong];xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=subprocess.Popen(['/usr/lib/xorg/Xorg',':111','-config','/workspace/scratch/surface-input-20261106/xorg.conf','-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
native=None
try:
 for _ in range(50):
  d=x.XOpenDisplay(b':111')
  if d:break
  time.sleep(.1)
 assert d
 receipts=[]
 import numpy as np
 geo=json.loads(Path('/workspace/scratch/placement-feedback-20261106/compositor/receipt.json').read_text())
 for w,h in [(1280,800),(640,480)]:
  native=subprocess.Popen(['/workspace/openyr/target/release/openyr','/workspace/game-data/RA2','EB1.MMX','--load','/workspace/scratch/placement-feedback-20261106/compositor/ready.oyrsave'],env=env,stdout=(out/f'{w}-native.log').open('w'),stderr=subprocess.STDOUT);time.sleep(4)
  assert native.poll() is None
  tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True);win=int(re.search(r'(0x[0-9a-f]+) "OpenYR',tree)[1],16)
  x.XMoveResizeWindow(d,win,0,0,w,h);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.8)
  def move(px,py):xt.XTestFakeMotionEvent(d,0,int(px),int(py),0);x.XFlush(d);time.sleep(.08)
  def button(n,pressed):xt.XTestFakeButtonEvent(d,n,pressed,0);x.XFlush(d);time.sleep(.06)
  def click(px,py,n=1):move(px,py);button(n,1);button(n,0)
  def capture(name):
   path=out/f'{w}x{h}-{name}.png';subprocess.run(['import','-window',hex(win),str(path)],env=env,check=True,timeout=10);im=Image.open(path).convert('RGB');assert im.size==(w,h);return im
  camera=[(geo['map_size'][0]-1112)/2,(geo['map_size'][1]-800)/2]
  def pan_to(ground):
   desired=[max(0,min(ground[i]-[w-168,h][i]/2,geo['map_size'][i]-[w-168,h][i])) for i in range(2)]
   remaining=[desired[i]-camera[i] for i in range(2)]
   while abs(remaining[0])+abs(remaining[1])>1:
    delta=[max(-250,min(250,int(v))) for v in remaining]
    if delta==[0,0]:break
    move(350,350);button(3,1);move(350-delta[0],350-delta[1]);button(3,0)
    for i in range(2):camera[i]+=delta[i];remaining[i]-=delta[i]
   return [ground[i]-camera[i] for i in range(2)]
  invalid_target=pan_to(geo['invalid_ground']);capture('ready')
  click(w-168+43,248);click(*invalid_target);invalid=capture('invalid-visible');time.sleep(2.2);invalid_expired=capture('invalid-expired')
  click(*invalid_target);click(*invalid_target,n=3);cancelled=capture('right-cancelled')
  valid_target=pan_to(geo['valid_ground']);click(w-168+43,248);move(*valid_target);capture('valid-ghost');click(*valid_target);valid=capture('valid-visible');time.sleep(2.2);valid_expired=capture('valid-expired')
  mask=lambda im:np.all(np.array(im.crop((8,8,300,40)))==[255,255,0],axis=2)
  row=dict(size=[w,h],invalid_target=invalid_target,valid_target=valid_target)
  for name,im in [('invalid',invalid),('valid',valid)]:
   expected=Image.open('/workspace/scratch/placement-feedback-20261106/compositor/'+str(w)+'x'+str(h)+'-'+name+'-visible.png').convert('RGB')
   row[name]=dict(glyph_pixels=int(mask(im).sum()),matches_shared=bool(np.array_equal(mask(im),mask(expected))))
   assert row[name]['matches_shared'] and row[name]['glyph_pixels']>100,row
  row['expired_invalid_glyph_pixels']=int(mask(invalid_expired).sum());row['expired_valid_glyph_pixels']=int(mask(valid_expired).sum());row['cancelled_glyph_pixels']=int(mask(cancelled).sum())
  assert row['expired_invalid_glyph_pixels']==row['expired_valid_glyph_pixels']==row['cancelled_glyph_pixels']==0,row
  receipts.append(row);native.terminate();native.wait(timeout=10);native=None
 (out/'receipt.json').write_text(json.dumps(receipts,indent=2)+'\n');print(json.dumps(receipts))
finally:
 if native and native.poll() is None:native.terminate()
 server.terminate();server.wait(timeout=10)
