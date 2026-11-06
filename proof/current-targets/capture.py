import ctypes,ctypes.util,json,os,re,subprocess,time
from pathlib import Path
from PIL import Image
out=Path('/workspace/scratch/current-targets-20261106/live');out.mkdir(exist_ok=True)
env=dict(os.environ,DISPLAY=':112',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME='/workspace/scratch/current-targets-20261106/cache')
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
xt.XTestFakeMotionEvent.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong];xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong];xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=subprocess.Popen(['/usr/lib/xorg/Xorg',':112','-config','/workspace/scratch/surface-input-20261106/xorg.conf','-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
native=None
try:
 for _ in range(50):
  d=x.XOpenDisplay(b':112')
  if d:break
  time.sleep(.1)
 assert d
 receipts=[]
 import math
 geo=json.loads(Path('/workspace/scratch/current-targets-20261106/compositor/receipt.json').read_text())
 x.XDefaultRootWindow.argtypes=[ctypes.c_void_p];x.XDefaultRootWindow.restype=ctypes.c_ulong
 for w,h in [(1280,800),(640,480)]:
  native=subprocess.Popen(['/workspace/openyr/target/release/openyr','/workspace/game-data/RA2','EB1.MMX','--load','/workspace/scratch/current-targets-20261106/compositor/active.oyrsave'],env=env,stdout=(out/f'{w}-native.log').open('w'),stderr=subprocess.STDOUT);time.sleep(4)
  assert native.poll() is None
  tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True);win=int(re.search(r'(0x[0-9a-f]+) "OpenYR',tree)[1],16)
  x.XMoveResizeWindow(d,win,0,0,w,h);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.8)
  def move(px,py):xt.XTestFakeMotionEvent(d,0,int(px),int(py),0);x.XFlush(d);time.sleep(.12)
  def button(n,pressed):xt.XTestFakeButtonEvent(d,n,pressed,0);x.XFlush(d);time.sleep(.08)
  def click(px,py,n=1):move(px,py);button(n,1);button(n,0)
  def capture(name):
   path=out/f'{w}x{h}-{name}.png';subprocess.run(['import','-window',hex(win),str(path)],env=env,check=True,timeout=10);im=Image.open(path).convert('RGB');assert im.size==(w,h);return im
  camera=[(geo['map_size'][0]-1112)/2,(geo['map_size'][1]-800)/2]
  middle=[(geo['own_ground'][i]+geo['target_ground'][i])/2 for i in range(2)]
  desired=[max(0,min(middle[i]-[w-168,h][i]/2,geo['map_size'][i]-[w-168,h][i])) for i in range(2)]
  remaining=[desired[i]-camera[i] for i in range(2)]
  while abs(remaining[0])+abs(remaining[1])>1:
   delta=[max(-250,min(250,int(v))) for v in remaining]
   if delta==[0,0]:break
   move(350,350);button(3,1);move(350-delta[0],350-delta[1]);button(3,0)
   for i in range(2):camera[i]+=delta[i];remaining[i]-=delta[i]
  own=[geo['own_ground'][i]-camera[i] for i in range(2)];target=[geo['target_ground'][i]-camera[i] for i in range(2)]
  baseline=capture('unselected')
  click(*own);move(20,20);time.sleep(2.3);selected=capture('selected-after-notice-lifetime')
  move(w+20,h+20);outside=capture('pointer-outside')
  x.XSetInputFocus(d,x.XDefaultRootWindow(d),2,0);x.XFlush(d);time.sleep(.4);unfocused=capture('unfocused')
  x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.4);restored=capture('focus-restored-no-motion')
  click(20,20,n=3);deselected=capture('deselected')
  cx,cy=[math.floor(v+.5) for v in target];points=set()
  for sx in [-1,1]:
   for sy in [-1,1]:
    for step in range(6):points.add((cx+sx*20-sx*step,cy+sy*10));points.add((cx+sx*20,cy+sy*10-sy*step))
  assert len(points)==44
  count=lambda im:sum(im.getpixel((px,py))==(255,64,64) for py in range(cy-40,cy+40) for px in range(cx-40,cx+40))
  row=dict(size=[w,h],own=own,target=target,unselected=count(baseline),selected_after_2_3_seconds=count(selected),outside=count(outside),unfocused=count(unfocused),restored_without_motion=count(restored),deselected=count(deselected))
  receipts.append(row);assert row['selected_after_2_3_seconds']==row['outside']==row['restored_without_motion']==44,row;assert row['unselected']==row['unfocused']==row['deselected']==0,row
  native.terminate();native.wait(timeout=10);native=None
 (out/'receipt.json').write_text(json.dumps(receipts,indent=2)+'\n');print(json.dumps(receipts))
finally:
 if native and native.poll() is None:native.terminate()
 server.terminate();server.wait(timeout=10)
