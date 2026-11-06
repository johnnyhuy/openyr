import ctypes,ctypes.util,json,os,re,subprocess,time
from pathlib import Path
from PIL import Image
out=Path('/workspace/scratch/compact-window-20261106/live');out.mkdir(exist_ok=True)
env=dict(os.environ,DISPLAY=':106',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME='/workspace/scratch/compact-window-20261106/cache')
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
xt.XTestFakeMotionEvent.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong];xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong];xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=subprocess.Popen(['/usr/lib/xorg/Xorg',':106','-config','/workspace/scratch/surface-input-20261106/xorg.conf','-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
native=None
try:
 for _ in range(50):
  d=x.XOpenDisplay(b':106')
  if d:break
  time.sleep(.1)
 assert d
 receipts=[]
 for label,binary in [('after','/workspace/openyr/target/release/openyr')]:
  for w,h in [(640,480),(1000,700)]:
   native=subprocess.Popen([binary],env=env,stdout=(out/f'{label}-{w}-native.log').open('w'),stderr=subprocess.STDOUT);time.sleep(5)
   assert native.poll() is None
   tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True);win=int(re.search(r'(0x[0-9a-f]+) "OpenYR',tree)[1],16)
   def resize(a,b):x.XMoveResizeWindow(d,win,0,0,a,b);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.8)
   def click(px,py):
    xt.XTestFakeMotionEvent(d,0,int(px),int(py),0);x.XFlush(d);time.sleep(.15)
    xt.XTestFakeButtonEvent(d,1,1,0);xt.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d);time.sleep(.5)
   def capture(page):
    name=f'{label}-{w}x{h}-{page}.png';subprocess.run(['import','-window',hex(win),str(out/name)],env=env,check=True,timeout=10);assert Image.open(out/name).size==(w,h);receipts.append(dict(file=name,size=[w,h],page=page))
   if label=='before':
    resize(1280,800);click(962,320);click(962,404);resize(w,h);capture('skirmish-clipped')
   else:
    resize(w,h);capture('main');s=min(w/800,h/600,1);ox=(w-800*s)/2;oy=(h-600*s)/2
    def menu(px,py):click(ox+px*s,oy+py*s)
    menu(722,220);menu(722,304);capture('skirmish');menu(722,304);capture('chooser');menu(722,556);menu(722,262);time.sleep(4);capture('match')
    code=x.XKeysymToKeycode(d,x.XStringToKeysym(b'Escape'));xt.XTestFakeKeyEvent(d,code,1,0);xt.XTestFakeKeyEvent(d,code,0,0);x.XFlush(d);time.sleep(.5);capture('pause');click(ox+(800 if w>=800 else 640)-82,oy+(600 if h>=600 else 480)-52);capture('resumed');click(w-48,208);capture('vehicles');xt.XTestFakeMotionEvent(d,0,220,220,0);xt.XTestFakeButtonEvent(d,3,1,0);x.XFlush(d);time.sleep(.2);xt.XTestFakeMotionEvent(d,0,350,300,0);x.XFlush(d);time.sleep(.2);xt.XTestFakeButtonEvent(d,3,0,0);x.XFlush(d);time.sleep(.3);capture('world-pan')
   native.terminate();native.wait(timeout=10);native=None
 (out/'receipt.json').write_text(json.dumps(receipts,indent=2)+'\n');print(json.dumps(receipts))
finally:
 if native and native.poll() is None:native.terminate()
 server.terminate();server.wait(timeout=10)
