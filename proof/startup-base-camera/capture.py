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
env = dict(os.environ, DISPLAY=':139', OPENYR_GAMEDIR='/workspace/game-data/RA2',
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
 server = subprocess.Popen(['/usr/lib/xorg/Xorg', ':139', '-config', str(repo/'.tmp/xorg-restored.conf'), '-logfile', str(out/'xorg-detail.log'), '-noreset', '-nolisten', 'tcp', '-ac'], env=env, stdout=(out/'xorg.log').open('w'), stderr=subprocess.STDOUT)
 for _ in range(80):
  display = x.XOpenDisplay(b':139')
  if display: break
  time.sleep(.1)
 assert display
 native = subprocess.Popen(['python3',str(repo/'scripts/proof_build.py'),'run','--receipt',str(receipt),'--','/workspace/game-data/RA2','EB1.MMX'],cwd=repo,env=env,stdout=(out/'native.log').open('w'),stderr=subprocess.STDOUT,start_new_session=True)
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
 recording=subprocess.Popen(['ffmpeg','-y','-f','x11grab','-framerate','30','-video_size','1280x800','-i',':139.0+0,0','-c:v','libx264','-preset','veryfast','-crf','24','-pix_fmt','yuv420p','-movflags','+faststart',str(out/'native-demo.mp4')],env=env,stdout=(out/'video.log').open('w'),stderr=subprocess.STDOUT)
 start=time.monotonic();capture('initial-gameplay')
 actions=[{'type':'capture','name':'automatic-base'}, {'type':'key','name':'p'}, {'type':'capture','name':'selected-army'}, {'type':'click','x':520,'y':400}, {'type':'wait','seconds':8}, {'type':'capture','name':'movement'}, {'type':'key','name':'a'}, {'type':'click','x':650,'y':300}, {'type':'wait','seconds':6}, {'type':'capture','name':'attack-move'}, {'type':'key','name':'q'}, {'type':'click','x':1150,'y':250}, {'type':'wait','seconds':5}, {'type':'capture','name':'production'}, {'type':'key','name':'Escape'}, {'type':'wait','seconds':2}, {'type':'capture','name':'pause'}, {'type':'key','name':'Escape'}, {'type':'wait','seconds':2}, {'type':'capture','name':'resumed'}]
 (out/'actions.json').write_text(json.dumps(actions,indent=2)+'\n')
 for action in actions:
  if action['type']=='key':key(action['name'])
  elif action['type']=='click':click(action['x'],action['y'])
  elif action['type']=='wait':time.sleep(action['seconds'])
  elif action['type']=='capture':capture(action['name'])
  else:raise ValueError('unknown action')
  assert native.poll() is None,'app exited during gameplay'
 recording.send_signal(signal.SIGINT);recording.wait(timeout=15);assert recording.returncode in (0,255)
 metadata=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-show_format','-of','json',str(out/'native-demo.mp4')],text=True))
 assert float(metadata['format']['duration'])>15
 (out/'capture.json').write_text(json.dumps({'build':json.loads(receipt.read_text()),'platform':'Linux Xorg native window; synthetic XTest input; silent video','events':events,'video':metadata},indent=2)+'\n')
 print(json.dumps(events),flush=True)
finally:
 if recording and recording.poll() is None:recording.send_signal(signal.SIGINT);recording.wait(timeout=15)
 if native and native.poll() is None:os.killpg(native.pid,signal.SIGTERM);native.wait(timeout=10)
 if server and server.poll() is None:server.terminate();server.wait(timeout=10)
