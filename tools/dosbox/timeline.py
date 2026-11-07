"""Frame-change timeline. usage: timeline.py (video.mkv | framesdir) [t0 t1]"""
import sys, os, subprocess, glob
import numpy as np
from PIL import Image
src = sys.argv[1]
t0 = float(sys.argv[2]) if len(sys.argv) > 2 else 0
t1 = float(sys.argv[3]) if len(sys.argv) > 3 else 1e9
def frames():
    if os.path.isdir(src):
        for i, f in enumerate(sorted(glob.glob(src + '/*.png'))):
            im = np.asarray(Image.open(f).convert('RGB'))
            yield i / 30, im
    else:
        p = subprocess.Popen(['ffmpeg', '-loglevel', 'error', '-i', src, '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'], stdout=subprocess.PIPE)
        i = 0
        while True:
            b = p.stdout.read(720 * 400 * 3)
            if len(b) < 720 * 400 * 3: break
            yield i / 30, np.frombuffer(b, np.uint8).reshape(400, 720, 3)
            i += 1
prev = None
for t, im in frames():
    if t < t0 or t > t1: continue
    blue = (im[:, :, 2] > 150) & (im[:, :, 0] < 50) & (im[:, :, 1] < 50)
    gfx = blue[:350, :640].mean() > 0.3
    a = im[:350, :640] if gfx else im
    if prev is not None and prev.shape == a.shape:
        d = np.any(a != prev, axis=2)
        n = int(d.sum())
        if n:
            ys, xs = np.nonzero(d)
            print(f"{t:7.3f} {'G' if gfx else 'T'} {n:6d} box=({xs.min()},{ys.min()})-({xs.max()},{ys.max()})")
    elif prev is not None:
        print(f"{t:7.3f} MODE {'G' if gfx else 'T'} {a.shape}")
    prev = a
