import sys, subprocess
# usage: grid.py img x y w h  -> prints chars per color
img,x,y,w,h=sys.argv[1],*map(int,sys.argv[2:6])
raw=subprocess.run(['convert',img,'-crop',f'{w}x{h}+{x}+{y}','+repage','-depth','8','rgb:-'],capture_output=True).stdout
pal={}
syms='.#o+*%@abcdefgh'
for r in range(h):
    s=''
    for c in range(w):
        p=raw[(r*w+c)*3:(r*w+c)*3+3]
        if p not in pal: pal[p]=syms[len(pal)]
        s+=pal[p]
    print(s)
