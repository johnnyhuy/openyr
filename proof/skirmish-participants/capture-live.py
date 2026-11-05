import ctypes
import ctypes.util
import os
from pathlib import Path
import subprocess
import time

out = Path('/workspace/scratch/skirmish-proof/live')
env = dict(os.environ, DISPLAY=':103', OPENYR_GAMEDIR='/workspace/game-data/RA2',
           XDG_CACHE_HOME='/workspace/scratch/vanilla-reference/cache',
           MESA_SHADER_CACHE_DIR='/workspace/scratch/vanilla-reference/cache/mesa')
server = native = recording = None
try:
    server_log = (out / 'live-xorg.log').open('w')
    server = subprocess.Popen(['/usr/lib/xorg/Xorg', ':103', '-config',
        '/workspace/scratch/vanilla-reference/xorg.conf', '-logfile', str(out / 'live-xorg-detail.log'),
        '-noreset', '-nolisten', 'tcp', '-ac'], stdout=server_log, stderr=subprocess.STDOUT, env=env)
    x11 = ctypes.CDLL(ctypes.util.find_library('X11'))
    xtst = ctypes.CDLL(ctypes.util.find_library('Xtst'))
    x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x11.XOpenDisplay.restype = ctypes.c_void_p
    x11.XFlush.argtypes = [ctypes.c_void_p]
    xtst.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_ulong]
    xtst.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
    display = None
    for _ in range(40):
        display = x11.XOpenDisplay(b':103')
        if display:
            break
        time.sleep(.1)
    assert display, 'Xorg did not become ready'
    native_log = (out / 'live-native.log').open('w')
    native = subprocess.Popen(['/workspace/openyr/target/release/openyr'],
        env=env, stdout=native_log, stderr=subprocess.STDOUT)
    time.sleep(5)
    assert native.poll() is None, 'Native window exited'

    def capture(name):
        path = out / name
        subprocess.run(['import', '-window', 'root', str(path)], env=env, check=True, timeout=10)
        colors = int(subprocess.check_output(['identify', '-format', '%k', str(path)], text=True))
        print(name, 'colors', colors, flush=True)
        assert colors > 100, 'Capture is blank'

    def click(x, y):
        xtst.XTestFakeMotionEvent(display, 0, x, y, 0)
        xtst.XTestFakeButtonEvent(display, 1, 1, 0)
        xtst.XTestFakeButtonEvent(display, 1, 0, 0)
        x11.XFlush(display)

    capture('live-01-main.png')
    click(962, 320)
    time.sleep(.5)
    capture('live-02-single-player.png')
    click(962, 404)
    time.sleep(2)
    capture('live-03-skirmish.png')
    click(540, 172)
    time.sleep(.2)
    capture('live-04-country-popup.png')
    click(540, 305)
    time.sleep(.2)
    click(685, 170)
    time.sleep(.2)
    click(685, 238)
    time.sleep(.2)
    click(319, 395)
    time.sleep(.2)
    capture('live-05-configured.png')
    video_log = (out / 'live-video.log').open('w')
    recording = subprocess.Popen(['ffmpeg', '-y', '-f', 'x11grab', '-framerate', '60',
        '-video_size', '1280x800', '-i', ':103.0', '-t', '10', '-c:v', 'libx264',
        '-preset', 'ultrafast', '-pix_fmt', 'yuv420p', str(out / 'live-start-flow.mp4')],
        env=env, stdout=video_log, stderr=subprocess.STDOUT)
    time.sleep(.4)
    click(962, 362)
    time.sleep(.1)
    capture('live-06-after-start.png')
    time.sleep(3)
    capture('live-07-match.png')
    recording.wait(timeout=15)
    assert recording.returncode == 0
    print('Live X11 screenshot and 60 fps start-flow capture succeeded.', flush=True)
finally:
    for process in [recording, native, server]:
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
