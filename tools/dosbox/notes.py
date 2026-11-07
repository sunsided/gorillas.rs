import sys, numpy as np, wave
f = sys.argv[1]
if f.endswith('.raw'):
    a = np.frombuffer(open(f,'rb').read(), np.int16).reshape(-1, 2)[:, 0].astype(float)
else:
    w = wave.open(f); a = np.frombuffer(w.readframes(w.getnframes()), np.int16).astype(float)
rate = 44100
a -= np.convolve(a, np.ones(441)/441, 'same')  # remove DC drift
win = 88  # 2 ms
n = len(a)//win
e = np.abs(a[:n*win]).reshape(n, win).max(axis=1)
thr = max(e.max()*0.05, 300)
on = e > thr
# merge gaps < 3 windows(6ms) for low freqs
segs=[]; i=0
while i<n:
    if on[i]:
        j=i
        while j<n and (on[j] or (j+12<n and on[j:j+12].any())): j+=1
        seg=a[i*win:j*win]; z=np.sum((seg[:-1]<0)&(seg[1:]>=0)); hz=z/(len(seg)/rate)
        segs.append((i*win/rate,(j-i)*win/rate,hz)); i=j
    else: i+=1
t0,t1=float(sys.argv[2]),float(sys.argv[3])
for s in segs:
    if t0<=s[0]<=t1: print(f"{s[0]:8.3f} dur={s[1]*1000:6.1f}ms hz={s[2]:6.0f}")
