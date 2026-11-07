import ctypes,ctypes.util,hashlib,json,os,re,signal,subprocess,sys,time
from pathlib import Path
root=Path('/workspace/scratch/confirmation-targets-20261108/v2');phase=sys.argv[1];assert phase in ['before','after']
repo=Path('/workspace/openyr-menu-keyboard-20261107-2') if phase=='before' else Path('/workspace/openyr-menu-confirmation-20261108')
receipt=repo/('.tmp/main-after-sweep/receipt.json' if phase=='before' else '.tmp/runtime/receipt.json')
out=root/phase;out.mkdir(exist_ok=False)
env=dict(os.environ,DISPLAY=':126',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME=str(root/'cache'))
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong]
xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=native=video=None
runs=[]
try:
 server=subprocess.Popen(['/usr/lib/xorg/Xorg',':126','-config',str(root/'xorg.conf'),'-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
 for _ in range(80):
  d=x.XOpenDisplay(b':126')
  if d:break
  time.sleep(.1)
 assert d
 def event(name,on):
  code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()));assert code
  xt.XTestFakeKeyEvent(d,code,on,0);x.XFlush(d)
 def key(name,delay=.12):event(name,1);event(name,0);time.sleep(delay)
 def keys(*names):
  for name in names:key(name)
 for width,height in [(640,480),(1280,800)]:
  folder=root/f'{phase}-fixture-{width}';folder.mkdir(exist_ok=False)
  data=Path('/workspace/scratch/save-name-edit-20261107/fixture.oyrsave').read_bytes()
  files=[folder/'fixture.oyrsave',folder/('W'*47+'A.oyrsave'),folder/('W'*47+'B.oyrsave'),folder/('W'*200+'Z.oyrsave')]
  for file in files:file.write_bytes(data)
  native=subprocess.Popen(['python3',str(repo/'scripts/proof_build.py'),'run','--receipt',str(receipt),'--','/workspace/game-data/RA2','EB1.MMX','--load',str(files[0])],cwd=repo,env=env,stdout=(out/f'{width}-native.log').open('w'),stderr=subprocess.STDOUT,start_new_session=True)
  win=None
  for _ in range(200):
   assert native.poll() is None,'guarded native launch exited'
   tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True);m=re.search(r'(0x[0-9a-f]+) "OpenYR',tree)
   if m:win=int(m[1],16);break
   time.sleep(.1)
  assert win;time.sleep(1);x.XMoveResizeWindow(d,win,0,0,width,height);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.4)
  started=time.monotonic();events=[]
  def capture(name):
   file=out/f'{width}-{name}.png';subprocess.run(['import','-window',hex(win),str(file)],env=env,check=True,timeout=10)
   assert subprocess.check_output(['identify','-format','%wx%h',str(file)],text=True)==f'{width}x{height}'
   events.append(dict(name=name,seconds=round(time.monotonic()-started,3)))
  if width==1280:
   video=subprocess.Popen(['ffmpeg','-y','-f','x11grab','-framerate','30','-video_size','1280x800','-i',':126.0+0,0','-c:v','libx264','-threads','2','-preset','fast','-crf','24','-pix_fmt','yuv420p','-movflags','+faststart',str(out/'confirmations.mp4')],env=env,stdout=(out/'video.log').open('w'),stderr=subprocess.STDOUT)
  # Pause -> Save; use an existing, legal 48-character filename.
  keys('Escape','Down','Down','Return')
  for _ in range(50):key('BackSpace',.004)
  event('Shift_L',1)
  for _ in range(47):key('w',.008)
  key('a');event('Shift_L',0);time.sleep(.2)
  capture('save-editor');key('Return');capture('pause-overwrite')
  # Escape cancels overwrite; select the distinct second slot for load.
  keys('Escape','Escape','Down','Return','Down','Down','Return');capture('pause-load')
  # Keep the running match, then choose the externally named 201-char save.
  keys('Escape','Tab','Return','Down','Return');capture('pause-delete-long')
  # Return through Abort -> Main Menu, preserving the temporary save directory.
  keys('Escape','Escape','Down','Down','Down','Down','Return','Down','Down','Return');time.sleep(2)
  keys('Tab','Return','Tab','Return','Down','Down','Return');capture('front-load')
  keys('Escape','Tab','Return','Down','Return');capture('front-delete-long')
  # Default No must cancel deletion. No file may have changed in any case.
  key('Return');capture('front-delete-declined')
  # Both confirmation captures must retain the same front-shell heading.
  scale=min(width/800,height/600,1);ox=(width-800*scale)/2;oy=(height-600*scale)/2
  geometry=f'{int(164*scale)}x{int(40*scale)}+{int(ox+635*scale)}+{int(oy)}'
  def header(label):return subprocess.check_output(['convert',str(out/f'{width}-{label}.png'),'-crop',geometry,'-depth','8','rgb:-'])
  assert header('front-load')==header('front-delete-long'),'front route entered a different screen'
  assert header('front-delete-long')!=header('front-delete-declined'),'No did not return to the front save list'

  if video:
   video.send_signal(signal.SIGINT);video.wait(timeout=15);assert video.returncode in (0,255);video=None
  os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10);native=None
  assert set(folder.glob('*.oyrsave'))==set(files),'unexpected saved file'
  assert all(file.read_bytes()==data for file in files),'a declined operation changed a file'
  runs.append(dict(size=[width,height],events=events,files_unchanged=True,names=[file.name for file in files],snapshot_sha256=hashlib.sha256(data).hexdigest()))
 (out/'receipt.json').write_text(json.dumps(dict(build=json.loads(receipt.read_text()),platform='Native Linux Xorg, XTest input; no physical Mac/Retina/audio/original-game timing check',runs=runs),indent=2)+'\n')
 print(json.dumps(runs),flush=True)
finally:
 if video and video.poll() is None:video.send_signal(signal.SIGINT);video.wait(timeout=15)
 if native and native.poll() is None:os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10)
 if server and server.poll() is None:server.terminate();server.wait(timeout=10)
