import ctypes, ctypes.util, hashlib, json, os, re, signal, subprocess, sys, time
from pathlib import Path
root=Path('/workspace/scratch/name-shortcuts-20261107');phase=sys.argv[1]
assert phase in ['before','after']
repo=Path('/workspace/openyr-mise-demo-20261107') if phase=='before' else Path('/workspace/openyr-menu-keyboard-20261107-2')
receipt=repo/('.tmp/runtime-llvm-final/receipt.json' if phase=='before' else '.tmp/runtime-final/receipt.json')
out=root/phase;out.mkdir(exist_ok=False)
env=dict(os.environ,DISPLAY=':125',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME=str(root/'cache'))
x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p
for fn,args in [('XFlush',[ctypes.c_void_p]),('XMoveResizeWindow',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]),('XSetInputFocus',[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong])]:getattr(x,fn).argtypes=args
x.XStringToKeysym.argtypes=[ctypes.c_char_p];x.XStringToKeysym.restype=ctypes.c_ulong;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong]
xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
xt.XTestFakeMotionEvent.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong]
xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server=native=video=None
rows=[]
try:
 server=subprocess.Popen(['/usr/lib/xorg/Xorg',':125','-config',str(root/'xorg.conf'),'-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
 for _ in range(80):
  d=x.XOpenDisplay(b':125')
  if d:break
  time.sleep(.1)
 assert d
 def event(name,on):
  code=x.XKeysymToKeycode(d,x.XStringToKeysym(name.encode()));assert code
  xt.XTestFakeKeyEvent(d,code,on,0);x.XFlush(d)
 def key(name,delay=.08):event(name,1);event(name,0);time.sleep(delay)
 def shortcut(mod,letter):event(mod,1);time.sleep(.1);key(letter);event(mod,0);time.sleep(.15)
 def type_text(text):
  for ch in text:key(ch,.015)
  time.sleep(.2)
 def click(px,py):
  xt.XTestFakeMotionEvent(d,0,px,py,0);xt.XTestFakeButtonEvent(d,1,1,0);xt.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d);time.sleep(.2)
 def launch(args,w,h,label):
  global native
  native=subprocess.Popen(['python3',str(repo/'scripts/proof_build.py'),'run','--receipt',str(receipt),'--',*args],cwd=repo,env=env,stdout=(out/(label+'-native.log')).open('w'),stderr=subprocess.STDOUT,start_new_session=True)
  win=None
  for _ in range(200):
   assert native.poll() is None,'verified native app exited'
   tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True);m=re.search(r'(0x[0-9a-f]+) "OpenYR',tree)
   if m:win=int(m[1],16);break
   time.sleep(.1)
  assert win
  time.sleep(1);x.XMoveResizeWindow(d,win,0,0,w,h);x.XSetInputFocus(d,win,2,0);x.XFlush(d);time.sleep(.3)
  return win
 def stop():
  global native
  os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10);native=None
 def capture(win,label):
  p=out/(label+'.png');subprocess.run(['import','-window',hex(win),str(p)],env=env,check=True,timeout=10);return p
 def pixels(p,rect):
  a,b,w,h=rect;return subprocess.check_output(['convert',str(p),'-crop',f'{w}x{h}+{a}+{b}','-depth','8','rgb:-'])
 def selected(p,rect):
  data=pixels(p,rect);return sum(data[i:i+3]==bytes([32,56,88]) for i in range(0,len(data),3))
 for w,h in [(640,480),(1280,800)]:
  for mod in ['Control_L','Super_L']:
   label=f'{w}-{mod}';folder=root/(phase+'-'+label);folder.mkdir(exist_ok=False)
   fixture=folder/'fixture.oyrsave';fixture.write_bytes(Path('/workspace/scratch/save-name-edit-20261107/fixture.oyrsave').read_bytes())
   win=launch(['/workspace/game-data/RA2','EB1.MMX','--load',str(fixture)],w,h,label)
   if w==1280 and mod=='Super_L':
    video=subprocess.Popen(['ffmpeg','-y','-f','x11grab','-framerate','30','-video_size','1280x800','-i',':125.0+0,0','-c:v','libx264','-threads','2','-preset','fast','-crf','24','-pix_fmt','yuv420p','-movflags','+faststart',str(out/'save-shortcuts.mp4')],env=env,stdout=(out/'video.log').open('w'),stderr=subprocess.STDOUT)
   key('Escape');key('Down');key('Down');key('Return')
   for _ in range(50):key('BackSpace',.004)
   type_text('oldcheckpoint');capture(win,label+'-initial')
   shortcut(mod,'a');a=capture(win,label+'-selected')
   rect=(38,414,396,30) if w==640 else (166,430,780,30)
   highlighted=selected(a,rect);assert (highlighted>1000)==(phase=='after'),(phase,label,highlighted)
   shortcut(mod,'c');b=capture(win,label+'-unsupported')
   if phase=='after':assert pixels(a,rect)==pixels(b,rect),'unsupported shortcut changed selection/name'
   type_text('newcheckpoint');capture(win,label+'-replacement')
   key('Return');capture(win,label+'-saved');time.sleep(.3)
   if video:
    video.send_signal(signal.SIGINT);video.wait(timeout=15);assert video.returncode in (0,255);video=None
   stop()
   saves=[p for p in folder.glob('*.oyrsave') if p!=fixture];assert len(saves)==1
   path=saves[0]
   if phase=='after':assert path.name=='newcheckpoint.oyrsave',path.name
   else:assert path.stem.startswith('oldcheckpoint') and path.stem.endswith('newcheckpoint'),path.name
   target=folder/'roundtrip.oyrsave';subprocess.run(['python3',str(repo/'scripts/proof_build.py'),'run','--receipt',str(receipt),'--','/workspace/game-data/RA2','EB1.MMX','--load',str(path),'--save',str(target)],cwd=repo,env=env,stdout=(out/(label+'-roundtrip.log')).open('w'),stderr=subprocess.STDOUT,check=True)
   assert path.read_bytes()==target.read_bytes()
   rows.append(dict(kind='save',size=[w,h],modifier=mod,selected_pixels=highlighted,saved_name=path.name,roundtrip_identical=True,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
 win=launch([],1280,800,'skirmish');time.sleep(2);click(962,320);click(962,404);time.sleep(2)
 for row in [0,1]:
  click(310,170+26*row);type_text('oldplayer');shortcut('Super_L','a');a=capture(win,f'player-{row}-selected')
  rect=(301,161+26*row,147,21);highlighted=selected(a,rect)
  assert (highlighted>1000)==(phase=='after'),(phase,row,highlighted)
  type_text('newplayer');capture(win,f'player-{row}-replacement')
  rows.append(dict(kind='player',row=row,modifier='Super_L',selected_pixels=highlighted))
 stop()
 (out/'receipt.json').write_text(json.dumps(dict(build=json.loads(receipt.read_text()),platform='Linux Xorg/XTest, silent video; no physical Mac/Retina/audio/original runtime',runs=rows),indent=2)+'\n')
 print(json.dumps(rows),flush=True)
finally:
 if video and video.poll() is None:video.send_signal(signal.SIGINT);video.wait(timeout=15)
 if native and native.poll() is None:stop()
 if server and server.poll() is None:server.terminate();server.wait(timeout=10)
