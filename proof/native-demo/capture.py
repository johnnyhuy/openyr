"""Live Linux Xorg demo. Run in the same task session as Xorg and FFmpeg."""
import ctypes
import ctypes.util
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

repo = Path(sys.argv[1]).resolve()
receipt = Path(sys.argv[2]).resolve()
out = Path(sys.argv[3]).resolve()
out.mkdir(parents=True, exist_ok=False)
env = dict(os.environ, DISPLAY=':124', OPENYR_GAMEDIR='/workspace/game-data/RA2',
           XDG_CACHE_HOME=str(out/'cache'), MESA_SHADER_CACHE_DIR=str(out/'cache/mesa'))
x = ctypes.CDLL(ctypes.util.find_library('X11'))
xt = ctypes.CDLL(ctypes.util.find_library('Xtst'))
x.XOpenDisplay.argtypes = [ctypes.c_char_p]
x.XOpenDisplay.restype = ctypes.c_void_p
x.XFlush.argtypes = [ctypes.c_void_p]
x.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
x.XStringToKeysym.argtypes = [ctypes.c_char_p]
x.XStringToKeysym.restype = ctypes.c_ulong
x.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
xt.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_ulong]
xt.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
xt.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
server = native = recording = None
start = time.monotonic()
events = []
try:
 server = subprocess.Popen(['/usr/lib/xorg/Xorg', ':124', '-config', '/workspace/scratch/save-name-edit-20261107/xorg.conf', '-logfile', str(out/'xorg-detail.log'), '-noreset', '-nolisten', 'tcp', '-ac'], env=env, stdout=(out/'xorg.log').open('w'), stderr=subprocess.STDOUT)
 for _ in range(80):
  display = x.XOpenDisplay(b':124')
  if display: break
  time.sleep(.1)
 assert display
 native = subprocess.Popen(['python3',str(repo/'scripts/proof_build.py'),'run','--receipt',str(receipt)],cwd=repo,env=env,stdout=(out/'native.log').open('w'),stderr=subprocess.STDOUT,start_new_session=True)
 win = None
 for _ in range(200):
  assert native.poll() is None, 'verified native app exited'
  tree = subprocess.check_output(['xwininfo','-root','-tree'],env=env,text=True)
  m = re.search(r'(0x[0-9a-f]+) "OpenYR',tree)
  if m: win=int(m[1],16); break
  time.sleep(.1)
 assert win
 x.XSetInputFocus(display,win,2,0);x.XFlush(display)
 geometry=subprocess.check_output(['xwininfo','-id',hex(win)],env=env,text=True)
 assert 'Width: 1280' in geometry and 'Height: 800' in geometry
 def click(px,py):
  xt.XTestFakeMotionEvent(display,0,px,py,0)
  xt.XTestFakeButtonEvent(display,1,1,0);xt.XTestFakeButtonEvent(display,1,0,0);x.XFlush(display)
 def key(name):
  code=x.XKeysymToKeycode(display,x.XStringToKeysym(name.encode()));assert code
  xt.XTestFakeKeyEvent(display,code,1,0);xt.XTestFakeKeyEvent(display,code,0,0);x.XFlush(display)
 def capture(name):
  subprocess.run(['import','-window',hex(win),str(out/(name+'.png'))],env=env,check=True,timeout=10)
  colors=int(subprocess.check_output(['identify','-format','%k',str(out/(name+'.png'))],text=True));assert colors>100
  events.append({'name':name,'seconds':round(time.monotonic()-start,3),'colors':colors})
 time.sleep(3)
 recording=subprocess.Popen(['ffmpeg','-y','-f','x11grab','-framerate','30','-video_size','1280x800','-i',':124.0+0,0','-c:v','libx264','-preset','veryfast','-crf','24','-pix_fmt','yuv420p','-movflags','+faststart',str(out/'native-demo.mp4')],env=env,stdout=(out/'video.log').open('w'),stderr=subprocess.STDOUT)
 start=time.monotonic();time.sleep(1);capture('main-menu')
 time.sleep(1);click(962,320);time.sleep(1);capture('single-player')
 click(962,404);time.sleep(1);capture('skirmish')
 time.sleep(1);click(962,362);time.sleep(.15);capture('loading')
 time.sleep(7);capture('gameplay')
 key('minus');time.sleep(.2);key('minus');time.sleep(.2);key('minus');time.sleep(.5);capture('overview')
 key('Right');time.sleep(.3);key('Escape');time.sleep(.5);capture('pause')
 time.sleep(2);key('Escape');time.sleep(2);capture('resumed')
 recording.send_signal(signal.SIGINT);recording.wait(timeout=15);assert recording.returncode in (0,255)
 metadata=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-show_format','-of','json',str(out/'native-demo.mp4')],text=True))
 assert float(metadata['format']['duration'])>15
 (out/'capture.json').write_text(json.dumps({'build':json.loads(receipt.read_text()),'platform':'Linux Xorg native window; synthetic XTest input; silent video','events':events,'video':metadata},indent=2)+'\n')
 print(json.dumps(events),flush=True)
finally:
 if recording and recording.poll() is None:recording.send_signal(signal.SIGINT);recording.wait(timeout=15)
 if native and native.poll() is None:os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10)
 if server and server.poll() is None:server.terminate();server.wait(timeout=10)
