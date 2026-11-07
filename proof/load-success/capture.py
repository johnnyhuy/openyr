import ctypes,ctypes.util,hashlib,json,os,re,signal,subprocess,time
from pathlib import Path
from PIL import Image
root=Path('/workspace/scratch/load-success-20261107');out=root/'live'
repo=Path('/workspace/openyr-load-success-20261107');receipt=repo/'.tmp/runtime-final/receipt.json'
env=dict(os.environ,DISPLAY=':121',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME=str(root/'cache'))
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong]
xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=subprocess.Popen(['/usr/lib/xorg/Xorg',':121','-config',str(root/'xorg.conf'),'-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
native=None
try:
 for _ in range(80):
  d=x.XOpenDisplay(b':121')
  if d:break
  time.sleep(.1)
 assert d
 receipts=[]
 for w,h in [(640,480),(1280,800)]:
  folder=root/f'fixture-{w}';folder.mkdir(exist_ok=True);save=folder/'Checkpoint 07.oyrsave'
  save.write_bytes((root/'compositor/fixture.oyrsave').read_bytes())
  original=hashlib.sha256(save.read_bytes()).hexdigest()
  native=subprocess.Popen(['python3',str(repo/'scripts/proof_build.py'),'run','--receipt',str(receipt),'--','/workspace/game-data/RA2','EB1.MMX','--load',str(save)],cwd=repo,env=env,stdout=(out/f'{w}-native.log').open('w'),stderr=subprocess.STDOUT,start_new_session=True)
  win=None
  for _ in range(120):
   assert native.poll() is None,'guard/native launch failed'
   tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True);m=re.search(r'(0x[0-9a-f]+) "OpenYR',tree)
   if m:win=int(m[1],16);break
   time.sleep(.1)
  assert win
  time.sleep(2)
  x.XMoveResizeWindow(d,win,0,0,w,h);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.5)
  def key(name):
   code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()));assert code
   xt.XTestFakeKeyEvent(d,code,1,0);xt.XTestFakeKeyEvent(d,code,0,0);x.XFlush(d);time.sleep(.15)
  def capture(name):
   p=out/f'{w}x{h}-{name}.png';subprocess.run(['import','-window',hex(win),str(p)],env=env,check=True,timeout=10)
   im=Image.open(p).convert('RGB');assert im.size==(w,h);return im
  yellow=lambda im:sum(pixel==(255,255,0) for pixel in im.crop((8,8,min(w-168,536),64)).getdata())
  controls=lambda im: im.crop((w-148,166,w-20,197)).tobytes()
  initial=None
  for _ in range(60):
   im=capture('initial-poll')
   if sum(b>128 and b>r+20 for r,g,b in im.crop((w-148,166,w-20,197)).getdata())>100:
    initial=im;im.save(out/f'{w}x{h}-cli-start.png');break
   time.sleep(.1)
  assert initial is not None,'world controls did not finish drawing'
  base=yellow(initial)
  key('F9');capture('confirmation');key('Return')
  notice=None
  for _ in range(100):
   im=capture('poll')
   if controls(im)==controls(initial) and yellow(im)>base+50:
    notice=im;notice.save(out/f'{w}x{h}-loaded-notice.png');break
   time.sleep(.1)
  assert notice is not None,'no visible load-success notice'
  time.sleep(2.2);expired=capture('expired')
  assert yellow(expired)==base,('notice did not expire',yellow(expired),base)
  assert hashlib.sha256(save.read_bytes()).hexdigest()==original,'loading must not rewrite the save'
  receipts.append(dict(size=[w,h],initial_yellow=base,notice_yellow=yellow(notice),expired_yellow=yellow(expired),save_sha256=original,save_unchanged=True,entry='guarded CLI then F9/Enter async load'))
  os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10);native=None
 (out/'receipt.json').write_text(json.dumps(dict(program=json.loads(receipt.read_text()),runs=receipts),indent=2)+'\n')
 print(json.dumps(receipts))
finally:
 if native and native.poll() is None:os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10)
 server.terminate();server.wait(timeout=10)
