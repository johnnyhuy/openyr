import ctypes,ctypes.util,hashlib,json,os,re,subprocess,sys,time
from pathlib import Path
from PIL import Image
root=Path('/workspace/scratch/quicksave-modal-20261107')
phase=sys.argv[1];assert phase in ['before','after']
out=root/phase;out.mkdir(exist_ok=True)
binary=str(root/'openyr-before') if phase=='before' else '/workspace/openyr/target/release/openyr'
env=dict(os.environ,DISPLAY=':119',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME=str(root/'cache'))
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong]
xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
xt.XTestFakeMotionEvent.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong]
server=subprocess.Popen(['/usr/lib/xorg/Xorg',':119','-config',str(root/'xorg.conf'),'-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
native=None
try:
 for _ in range(80):
  d=x.XOpenDisplay(b':119')
  if d:break
  time.sleep(.1)
 assert d,'Xorg unavailable'
 receipts=[]
 fixture=(root/'compositor/fixture.oyrsave').read_bytes()
 for w,h in [(640,480),(1280,800)]:
  target=root/f'{phase}-{w}.oyrsave';target.write_bytes(fixture)
  native=subprocess.Popen([binary,'/workspace/game-data/RA2','EB1.MMX','--load',str(target)],env=env,stdout=(out/f'{w}-native.log').open('w'),stderr=subprocess.STDOUT)
  win=None
  for _ in range(100):
   assert native.poll() is None,'native process exited'
   tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True)
   m=re.search(r'(0x[0-9a-f]+) "OpenYR',tree)
   if m:win=int(m[1],16);break
   time.sleep(.2)
  assert win
  time.sleep(2)
  x.XMoveResizeWindow(d,win,0,0,w,h);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.5)
  xt.XTestFakeMotionEvent(d,0,100,100,0);x.XFlush(d);time.sleep(.2)
  def key(name):
   code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()));assert code
   xt.XTestFakeKeyEvent(d,code,1,0);xt.XTestFakeKeyEvent(d,code,0,0);x.XFlush(d);time.sleep(.25)
  def capture(name):
   p=out/f'{w}x{h}-{name}.png';subprocess.run(['import','-window',hex(win),str(p)],env=env,check=True,timeout=10)
   im=Image.open(p).convert('RGB');assert im.size==(w,h);return im
  controls=lambda im: im.crop((w-148,166,w-20,197)).tobytes()
  for failed in [False,True]:
   if failed:target.unlink();target.mkdir()
   for mode,shortcut in [('repair','k'),('sell','l')]:
    name=f'{mode}-failed-{failed}'
    neutral=capture(name+'-neutral')
    key(shortcut);armed=capture(name+'-armed')
    assert controls(armed)!=controls(neutral),'mode must visibly arm before F5'
    key('F5');capture(name+'-result')
    key('Escape')
    if failed:key('Escape')
    key('Escape');resumed=capture(name+'-resumed')
    matches_neutral=controls(resumed)==controls(neutral)
    matches_armed=controls(resumed)==controls(armed)
    assert matches_neutral==(phase=='after'),(phase,w,name,matches_neutral)
    assert matches_armed==(phase=='before'),(phase,w,name,matches_armed)
    receipts.append(dict(phase=phase,size=[w,h],mode=mode,failed_save=failed,resumed_matches_neutral=matches_neutral,resumed_matches_armed=matches_armed,controls_region=[w-148,166,w-20,197]))
    if phase=='before':key(shortcut)
   if failed:target.rmdir()
  native.terminate();native.wait(timeout=10);native=None
 (out/'receipt.json').write_text(json.dumps(dict(binary_sha256=hashlib.sha256(Path(binary).read_bytes()).hexdigest(),runs=receipts),indent=2)+'\n')
 print(json.dumps(receipts))
finally:
 if native and native.poll() is None:native.terminate();native.wait(timeout=10)
 server.terminate();server.wait(timeout=10)
