import ctypes,ctypes.util,json,os,re,subprocess,time
from pathlib import Path
from PIL import Image
out=Path('/workspace/scratch/hover-feedback-20261106/live');out.mkdir(exist_ok=True)
env=dict(os.environ,DISPLAY=':107',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME='/workspace/scratch/hover-feedback-20261106/cache')
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
xt.XTestFakeMotionEvent.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong];xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong];xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=subprocess.Popen(['/usr/lib/xorg/Xorg',':107','-config','/workspace/scratch/surface-input-20261106/xorg.conf','-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
native=None
try:
 for _ in range(50):
  d=x.XOpenDisplay(b':107')
  if d:break
  time.sleep(.1)
 assert d
 receipts=[]
 geo=json.loads(Path('/workspace/scratch/hover-feedback-20261106/compositor/native-geometry.json').read_text())
 x.XDefaultRootWindow.argtypes=[ctypes.c_void_p];x.XDefaultRootWindow.restype=ctypes.c_ulong
 for w,h in [(1280,800),(640,480)]:
  native=subprocess.Popen(['/workspace/openyr/target/release/openyr','/workspace/game-data/RA2','EB1.MMX'],env=env,stdout=(out/f'{w}-native.log').open('w'),stderr=subprocess.STDOUT);time.sleep(4)
  assert native.poll() is None
  tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True);win=int(re.search(r'(0x[0-9a-f]+) "OpenYR',tree)[1],16)
  x.XMoveResizeWindow(d,win,0,0,w,h);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.8)
  def move(px,py):xt.XTestFakeMotionEvent(d,0,int(px),int(py),0);x.XFlush(d);time.sleep(.12)
  def button(n,pressed):xt.XTestFakeButtonEvent(d,n,pressed,0);x.XFlush(d);time.sleep(.1)
  def capture(name):
   path=out/f'{w}x{h}-{name}.png';subprocess.run(['import','-window',hex(win),str(path)],env=env,check=True,timeout=10);im=Image.open(path).convert('RGB');assert im.size==(w,h);return im
  # Main c0b436d starts at this rendered-raster center; resizing retains it.
  initial=((geo['map_size'][0]-1112)/2,(geo['map_size'][1]-800)/2)
  ground=geo['targets'][0]['ground'];target=((w-168)/2,h/2)
  remaining=[ground[0]-target[0]-initial[0],ground[1]-target[1]-initial[1]]
  while abs(remaining[0])+abs(remaining[1])>1:
   delta=[max(-250,min(250,int(v))) for v in remaining]
   if delta==[0,0]:break
   move(350,350);button(3,1);move(350-delta[0],350-delta[1]);button(3,0)
   remaining=[v-dv for v,dv in zip(remaining,delta)]
  move(*target);hover=capture('hover-select')
  move(20,20);away=capture('away')
  move(*target);x.XSetInputFocus(d,x.XDefaultRootWindow(d),2,0);x.XFlush(d);time.sleep(.3);unfocused=capture('unfocused')
  x.XSetInputFocus(d,win,2,0);x.XFlush(d);move(*target);restored=capture('focus-restored')
  rect=(int(target[0]-40),int(target[1]-35),int(target[0]+40),int(target[1]+35))
  count=lambda im:sum(p==(64,255,128) for p in im.crop(rect).getdata())
  row=dict(size=[w,h],target=target,remaining_pan=remaining,hover_green=count(hover),away_green=count(away),unfocused_green=count(unfocused),restored_green=count(restored))
  receipts.append(row);assert row['hover_green']-row['away_green']>=15,row;assert row['unfocused_green']==row['away_green'],row;assert row['restored_green']>=15,row
  native.terminate();native.wait(timeout=10);native=None
 (out/'receipt.json').write_text(json.dumps(receipts,indent=2)+'\n');print(json.dumps(receipts))
finally:
 if native and native.poll() is None:native.terminate()
 server.terminate();server.wait(timeout=10)
