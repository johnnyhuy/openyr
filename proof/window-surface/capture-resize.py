import argparse,ctypes,ctypes.util,hashlib,json,os,re,subprocess,time
from pathlib import Path
from PIL import Image
p=argparse.ArgumentParser();p.add_argument('binary');p.add_argument('label');p.add_argument('--fixed',action='store_true');a=p.parse_args()
out=Path('/workspace/scratch/surface-input-20261106')/a.label;out.mkdir(exist_ok=True)
env=dict(os.environ,DISPLAY=':105',OPENYR_GAMEDIR='/workspace/game-data/RA2',XDG_CACHE_HOME='/workspace/scratch/surface-input-20261106/cache')
server=native=video=None
try:
 server=subprocess.Popen(['/usr/lib/xorg/Xorg',':105','-config','/workspace/scratch/surface-input-20261106/xorg.conf','-logfile',str(out/'xorg-detail.log'),'-noreset','-nolisten','tcp','-ac'],env=env,stdout=(out/'xorg.log').open('w'),stderr=subprocess.STDOUT)
 x=ctypes.CDLL(ctypes.util.find_library('X11'));xt=ctypes.CDLL(ctypes.util.find_library('Xtst'))
 x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=ctypes.c_void_p;x.XFlush.argtypes=[ctypes.c_void_p]
 x.XMoveResizeWindow.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_int,ctypes.c_uint,ctypes.c_uint]
 x.XSetInputFocus.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.c_int,ctypes.c_ulong]
 xt.XTestFakeMotionEvent.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong]
 xt.XTestFakeButtonEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
 display=None
 for _ in range(50):
  display=x.XOpenDisplay(b':105')
  if display:break
  time.sleep(.1)
 assert display,'Xorg startup failed'
 native=subprocess.Popen([a.binary],env=env,stdout=(out/'native.log').open('w'),stderr=subprocess.STDOUT)
 time.sleep(5);assert native.poll() is None,'Native startup failed'
 tree=subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True)
 match=re.search(r'(0x[0-9a-f]+) "OpenYR',tree);assert match,tree
 window=int(match.group(1),16);x.XMoveResizeWindow(display,window,0,0,1280,800);x.XSetInputFocus(display,window,2,0);x.XFlush(display)
 def move(px,py):xt.XTestFakeMotionEvent(display,0,int(px),int(py),0);x.XFlush(display);time.sleep(.15)
 def button(number):xt.XTestFakeButtonEvent(display,number,1,0);xt.XTestFakeButtonEvent(display,number,0,0);x.XFlush(display);time.sleep(.35)
 def click(px,py):move(px,py);button(1)
 def capture(name):
  path=out/name;subprocess.run(['import','-window',hex(window),str(path)],env=env,check=True,timeout=10);return Image.open(path).convert('RGB')
 click(962,320);click(962,404);time.sleep(2);click(962,404);time.sleep(2)
 capture('chooser-full.png')
 video=subprocess.Popen(['ffmpeg','-y','-f','x11grab','-framerate','30','-video_size','1920x1200','-i',':105.0','-t','10','-c:v','libx264','-preset','ultrafast','-pix_fmt','yuv420p',str(out/'resize-input.mp4')],env=env,stdout=(out/'video.log').open('w'),stderr=subprocess.STDOUT)
 receipts=[]
 for width,height in [(1600,1000),(1920,1200)]:
  x.XMoveResizeWindow(display,window,0,0,width,height);x.XFlush(display);time.sleep(.8)
  scale=1;ox=(width-1280*scale)/2;oy=(height-800*scale)/2
  rect=tuple(int(v) for v in (ox+580*scale,oy+230*scale,ox+752*scale,oy+565*scale))
  move(width//2,5)
  initial=capture(f'{width}x{height}-initial.png');assert initial.size==(width,height)
  button(5);margin=capture(f'{width}x{height}-margin-wheel.png')
  move(ox+800*scale,oy+650*scale);button(5);valid=capture(f'{width}x{height}-valid-wheel.png')
  h=lambda im:hashlib.sha256(im.crop(rect).tobytes()).hexdigest()
  same=h(initial)==h(margin);changed=h(margin)!=h(valid)
  receipts.append(dict(size=[width,height],list_rect=rect,margin_preserves_list=same,valid_wheel_changes_list=changed,initial_list_sha256=h(initial),margin_list_sha256=h(margin),valid_list_sha256=h(valid)))
  assert same==a.fixed,receipts[-1]
  assert changed,receipts[-1]
 (out/'receipt.json').write_text(json.dumps(receipts,indent=2)+'\n');print(json.dumps(receipts),flush=True)
 video.wait(timeout=15);assert video.returncode==0
 print('Live resized-window input matrix passed:',a.label,flush=True)
finally:
 for process in [video,native,server]:
  if process is not None and process.poll() is None:
   process.terminate()
   try:process.wait(timeout=5)
   except subprocess.TimeoutExpired:process.kill()
