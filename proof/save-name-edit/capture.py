import ctypes,ctypes.util,hashlib,json,os,re,signal,subprocess,sys,time
from pathlib import Path
from PIL import Image
root=Path('/workspace/scratch/save-name-edit-20261107');phase=sys.argv[1];assert phase in ['before','after']
repo=Path('/workspace/openyr-load-success-20261107') if phase=='before' else Path('/workspace/openyr-chooser-projection-20261107')
receipt=repo/('.tmp/runtime-final/receipt.json' if phase=='before' else '.tmp/runtime-after/receipt.json')
out=root/phase;out.mkdir(exist_ok=True)
env=dict(os.environ,DISPLAY=':123',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME=str(root/'cache'))
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong]
xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=subprocess.Popen(['/usr/lib/xorg/Xorg',':123','-config',str(root/'xorg.conf'),'-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
native=None
try:
 for _ in range(80):
  d=x.XOpenDisplay(b':123')
  if d:break
  time.sleep(.1)
 assert d
 receipts=[]
 for w,h in [(640,480),(1280,800)]:
  folder=root/f'{phase}-fixture-{w}';folder.mkdir(exist_ok=True);fixture=folder/'fixture.oyrsave';fixture.write_bytes((root/'fixture.oyrsave').read_bytes())
  native=subprocess.Popen(['python3',str(repo/'scripts/proof_build.py'),'run','--receipt',str(receipt),'--','/workspace/game-data/RA2','EB1.MMX','--load',str(fixture)],cwd=repo,env=env,stdout=(out/f'{w}-native.log').open('w'),stderr=subprocess.STDOUT,start_new_session=True)
  win=None
  for _ in range(120):
   assert native.poll() is None,'native guard/launch failed'
   tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True);m=re.search(r'(0x[0-9a-f]+) "OpenYR',tree)
   if m:win=int(m[1],16);break
   time.sleep(.1)
  assert win;time.sleep(2)
  x.XMoveResizeWindow(d,win,0,0,w,h);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.6)
  def event(name,on):
   code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()));assert code
   xt.XTestFakeKeyEvent(d,code,on,0);x.XFlush(d)
  def key(name,delay=.1):event(name,1);event(name,0);time.sleep(delay)
  def upper(letter,count=1):
   event('Shift_L',1)
   for _ in range(count):key(letter,.008)
   event('Shift_L',0);time.sleep(.2)
  def capture(name):
   p=out/f'{w}x{h}-{name}.png';subprocess.run(['import','-window',hex(win),str(p)],env=env,check=True,timeout=10)
   im=Image.open(p).convert('RGB');assert im.size==(w,h);return im
  key('Escape');key('Down');key('Down');key('Return')
  for _ in range(50):key('BackSpace',.005)
  upper('w',47);upper('a');a=capture('name-A')
  key('BackSpace');upper('b');b=capture('name-B')
  ox,oy=(0,0) if w==640 else (128,16)
  world=472 if w==640 else 856
  rect=(ox+38,oy+414,ox+world-38,oy+444)
  crop=lambda im:im.crop(rect).tobytes()
  assert sum(pixel==(255,232,0) for pixel in a.crop(rect).getdata())>100,'save editor was not active'
  changed=crop(a)!=crop(b)
  assert changed==(phase=='after' or w==1280),(phase,w,'edited suffix visibility',changed)
  key('BackSpace');capture('backspace');upper('b');again=capture('name-B-again')
  assert crop(again)==crop(b),'retyping should reproduce the same field'
  key('Return');capture('saved')
  path=folder/('W'*47+'B.oyrsave');assert path.is_file(),'complete typed filename was not saved'
  os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10);native=None
  roundtrip=folder/'roundtrip.oyrsave'
  command=['python3',str(repo/'scripts/proof_build.py'),'run','--receipt',str(receipt),'--','/workspace/game-data/RA2','EB1.MMX','--load',str(path),'--save',str(roundtrip)]
  subprocess.run(command,cwd=repo,env=env,stdout=(out/f'{w}-roundtrip.log').open('w'),stderr=subprocess.STDOUT,check=True)
  assert path.read_bytes()==roundtrip.read_bytes(),'native save roundtrip changed bytes'
  receipts.append(dict(phase=phase,size=[w,h],field_rect=rect,suffix_changes_visible=changed,retype_reproduces_pixels=True,saved_name=path.name,sha256=hashlib.sha256(path.read_bytes()).hexdigest(),roundtrip_identical=True))
 (out/'receipt.json').write_text(json.dumps(dict(build=json.loads(receipt.read_text()),runs=receipts),indent=2)+'\n');print(json.dumps(receipts))
finally:
 if native and native.poll() is None:os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10)
 server.terminate();server.wait(timeout=10)
