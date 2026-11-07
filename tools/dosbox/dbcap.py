#!/usr/bin/env python3
"""Drive QBasic in DOSBox-X headless (Xvfb) along a timeline, like `gorillas capture` does
for the port.

usage: dbcap.py OUTDIR TIMELINE PROGRAM.BAS

Timeline lines: `<t> key <xdotool-key>`, `<t> type <text>`, `<t> shot <name>`, `<t> end`.
Times count from the moment the program shows the intro's red stars minus 0.6 s (when the
port shows them), so both runs get keys at the same program-relative times. Set
DBCAP_NOSYNC=1 for programs without the intro.

Writes OUTDIR/<name>.png, OUTDIR/video.mkv (30 fps, lossless RGB) and OUTDIR/audio.raw
(SDL disk audio: 16-bit stereo, 44.1 kHz). Needs dosbox-x, Xvfb, xdotool, ImageMagick and
ffmpeg; the DOS drive is target/dosbox/dos (see fetch-qbasic.sh).
"""
import os, subprocess, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
DOS = os.environ.get("DOSBOX_DIR", os.path.join(HERE, "..", "..", "target", "dosbox", "dos"))
out, tl, prog = sys.argv[1], sys.argv[2], sys.argv[3]
os.makedirs(out, exist_ok=True)
events = []
for line in open(tl):
    line = line.rstrip("\n")
    if not line.strip() or line.startswith("#"):
        continue
    t, kind, *rest = line.split(" ", 2)
    events.append((float(t), kind, rest[0] if rest else ""))
disp = ":97"
xv = subprocess.Popen(["Xvfb", disp, "-screen", "0", "1024x768x24"], stderr=subprocess.DEVNULL)
time.sleep(1)
env = dict(os.environ, DISPLAY=disp, SDL_AUDIODRIVER="disk", SDL_DISKAUDIOFILE=f"{out}/audio.raw")
db = subprocess.Popen(["dosbox-x", "-conf", f"{HERE}/gorillas.conf", "-nogui", "-nomenu", "-fastlaunch",
                       "-c", f"mount c {os.path.abspath(DOS)}", "-c", "c:", "-c", f"QBASIC /RUN {prog}"],
                      env=env, stdout=open(f"{out}/dosbox.log", "w"), stderr=subprocess.STDOUT)
t0 = time.time()
vid = subprocess.Popen(["ffmpeg", "-loglevel", "error", "-y", "-f", "x11grab", "-framerate", "30",
                        "-video_size", "720x400", "-i", f"{disp}+0,0", "-c:v", "libx264rgb", "-qp", "0",
                        "-preset", "ultrafast", f"{out}/video.mkv"], env=env, stdin=subprocess.PIPE)
wid = None
# Sync: wait for the intro's red sparkle stars, which the port shows 0.6 s after power-on
# (the first SparklePause frame); count the timeline from that program-relative origin.
SYNC = float(os.environ.get("DBCAP_SYNC", "0.6"))
if os.environ.get("DBCAP_NOSYNC") is None:
    while time.time() - t0 < 60:
        raw = subprocess.run(["import", "-display", disp, "-window", "root", "-crop", "720x16+0+0", "-depth", "8", "rgb:-"],
                             env=env, capture_output=True).stdout
        if any(raw[i] > 150 and raw[i+1] < 60 and raw[i+2] < 60 for i in range(0, len(raw) - 2, 3)):
            t_sync = time.time()
            break
        time.sleep(0.02)
    else:
        db.kill(); vid.kill(); xv.kill()
        raise SystemExit("no intro")
    shift = t_sync - SYNC - t0
    print(f"sync at {t_sync - t0:.2f}s, shift {shift:.2f}s", flush=True)
    events = [(t + shift, k, a) for t, k, a in events]
    open(f"{out}/shift.txt", "w").write(f"{shift}\n")
def press(k):
    subprocess.run(["xdotool", "keydown", k], env=env)
    time.sleep(0.04)
    subprocess.run(["xdotool", "keyup", k], env=env)
for t, kind, arg in events:
    w = t - (time.time() - t0)
    if w > 0:
        time.sleep(w)
    if wid is None:
        wid = subprocess.run(["xdotool", "search", "--name", "DOSBox"], env=env, capture_output=True, text=True).stdout.split()[0]
        subprocess.run(["xdotool", "windowfocus", "--sync", wid], env=env)
    if kind == "key":
        press(arg)
    elif kind == "type":
        for ch in arg:
            k = {"\\n": "Return"}.get(ch, ch)
            press({" ": "space", ".": "period"}.get(k, k))
            time.sleep(0.06)
    elif kind == "shot":
        subprocess.run(["import", "-display", disp, "-window", wid, f"{out}/{arg}.png"], env=env, stderr=subprocess.DEVNULL)
        print(f"{time.time()-t0:7.3f} shot {arg}", flush=True)
    elif kind == "end":
        break
vid.communicate(b"q")
db.kill(); time.sleep(0.3); xv.kill()
