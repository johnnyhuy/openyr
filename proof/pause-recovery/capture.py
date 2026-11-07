import ctypes, ctypes.util, hashlib, json, os, re, subprocess, time
from pathlib import Path
from PIL import Image
root=Path('/workspace/scratch/pause-recovery-20261107');out=root/'live'
env=dict(os.environ,DISPLAY=':117',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME=str(root/'cache'))
binary='/workspace/openyr/target/release/openyr'
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong
x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong]
xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=subprocess.Popen(['/usr/lib/xorg/Xorg',':117','-config',str(root/'xorg.conf'),'-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
native=None
try:
 for _ in range(80):
  d=x.XOpenDisplay(b':117')
  if d:break
  time.sleep(.1)
 assert d,'Xorg display unavailable'
 receipts=[]
 for w,h in [(640,480),(1280,800)]:
  parent=root/f'fixture-{w}'
  for i in range(42):parent=parent/(f'{i:02}-'+ 'W'*82)
  parent.mkdir(parents=True,exist_ok=True)
  target=parent/'checkpoint.oyrsave'
  subprocess.run([binary,'/workspace/game-data/RA2','EB1.MMX','--ticks','1','--save',str(target)],env=env,stdout=(out/f'{w}-fixture.log').open('w'),stderr=subprocess.STDOUT,check=True)
  original=target.read_bytes();save_hash=hashlib.sha256(original).hexdigest()
  native=subprocess.Popen([binary,'/workspace/game-data/RA2','EB1.MMX','--load',str(target)],env=env,stdout=(out/f'{w}-native.log').open('w'),stderr=subprocess.STDOUT)
  win=None
  for _ in range(100):
   assert native.poll() is None,'native app exited'
   tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True)
   match=re.search(r'(0x[0-9a-f]+) "OpenYR',tree)
   if match:win=int(match[1],16);break
   time.sleep(.2)
  assert win
  time.sleep(3)
  x.XMoveResizeWindow(d,win,0,0,w,h);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.7)
  def key(name):
   code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()))
   assert code
   xt.XTestFakeKeyEvent(d,code,1,0);xt.XTestFakeKeyEvent(d,code,0,0);x.XFlush(d);time.sleep(.3)
  def capture(name):
   path=out/f'{w}x{h}-{name}.png'
   subprocess.run(['import','-window',hex(win),str(path)],env=env,check=True,timeout=10)
   im=Image.open(path).convert('RGB');assert im.size==(w,h);return im
  # An ordinary atomic rename must fail when its destination is a directory.
  target.unlink();target.mkdir()
  key('F5');first=capture('first')
  key('End');last=capture('last')
  assert first.tobytes()!=last.tobytes(),'End must reveal later diagnostic text'
  key('Home');home=capture('home')
  assert first.tobytes()==home.tobytes(),'Home must reproduce the first frozen frame'
  time.sleep(.5);frozen=capture('frozen')
  assert home.tobytes()==frozen.tobytes(),'diagnostic should keep the match frozen'
  key('Escape');back=capture('back-to-save')
  assert back.tobytes()!=first.tobytes(),'Escape must return to the save page'
  assert target.is_dir()
  receipts.append(dict(size=[w,h],save_sha256=save_hash,target_characters=len(str(target)),first_matches_home=True,frame_frozen=True,end_changes_text=True,escape_returns=True))
  native.terminate();native.wait(timeout=10);native=None
  # Generated match files are not part of the proof bundle.
 (out/'receipt.json').write_text(json.dumps(dict(binary_sha256=hashlib.sha256(Path(binary).read_bytes()).hexdigest(),runs=receipts),indent=2)+'\n')
 print(json.dumps(receipts))
finally:
 if native and native.poll() is None:native.terminate();native.wait(timeout=10)
 server.terminate();server.wait(timeout=10)
