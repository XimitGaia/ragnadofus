"""Build detailed 8-direction hero sheets from original animation matrices."""
import json,struct,zlib
from collections import deque
from pathlib import Path

HERE=Path(__file__).parent; SRC=HERE.parent/'source'
W,H=320,512; PIVOT=(160,480)
RIGHT=['S','SE','E','NE','N']
ALL=['S','SE','E','NE','N','NW','W','SW']
COUNT={'idle':4,'walk':8,'attack':5,'jump':4,'fall':3}
FPS={'idle':5,'walk':9,'attack':11,'jump':8,'fall':8}
def read(path):
 b=path.read_bytes();w,h=struct.unpack('>II',b[16:24]);i=8;chunks=b''
 while i<len(b):
  n=int.from_bytes(b[i:i+4],'big')
  if b[i+4:i+8]==b'IDAT':chunks+=b[i+8:i+8+n]
  i+=n+12
 raw=zlib.decompress(chunks);stride=w*4;rows=[];previous=bytearray(stride);off=0
 for _ in range(h):
  mode=raw[off];off+=1;row=bytearray(raw[off:off+stride]);off+=stride
  for x in range(stride):
   a=row[x-4] if x>=4 else 0;u=previous[x];ul=previous[x-4] if x>=4 else 0
   if mode==1:row[x]=(row[x]+a)&255
   elif mode==2:row[x]=(row[x]+u)&255
   elif mode==3:row[x]=(row[x]+((a+u)//2))&255
   elif mode==4:
    p=a+u-ul;da=abs(p-a);du=abs(p-u);dl=abs(p-ul)
    row[x]=(row[x]+(a if da<=du and da<=dl else u if du<=dl else ul))&255
  rows.append(row);previous=row
 return w,h,rows
def frame(rows,x0,x1,flip=False,lift=0,breath=0):
 h=len(rows);seen=set();components=[]
 for y in range(h):
  for x in range(x0,x1):
   if (x,y) in seen or rows[y][x*4+3]<245:continue
   q=deque([(x,y)]);seen.add((x,y));component=[]
   while q:
    px,py=q.popleft();component.append((px,py))
    for nx,ny in ((px-1,py),(px+1,py),(px,py-1),(px,py+1)):
     if x0<=nx<x1 and 0<=ny<h and (nx,ny) not in seen and rows[ny][nx*4+3]>=245:
      seen.add((nx,ny));q.append((nx,ny))
   components.append(component)
 points=max(components,key=len);mask=set(points)
 x0=min(x for x,y in points);x1=max(x for x,y in points)+1
 y0=min(y for x,y in points);y1=max(y for x,y in points)+1
 scale=min(300/(x1-x0),440/(y1-y0));dw=round((x1-x0)*scale);dh=round((y1-y0)*scale)
 out=bytearray(W*H*4);ox=(W-dw)//2;oy=PIVOT[1]-dh-lift
 for yy in range(dh):
  sy=min(y1-1,y0+int(yy/scale));line=rows[sy]
  dy=oy+yy-round(breath*(1-yy/max(1,dh-1)))
  if not 0<=dy<H:continue
  for xx in range(dw):
   sx=min(x1-1,x0+int(xx/scale));i=sx*4
   if (sx,sy) not in mask:continue
   dx=ox+(dw-1-xx if flip else xx);j=(dy*W+dx)*4
   out[j:j+4]=bytes((*line[i:i+3],255))
 return out
def png(path,w,h,pixels):
 raw=b''.join(b'\0'+pixels[y*w*4:(y+1)*w*4] for y in range(h))
 def ch(t,v):return struct.pack('>I',len(v))+t+v+struct.pack('>I',zlib.crc32(t+v)&0xffffffff)
 path.write_bytes(b'\x89PNG\r\n\x1a\n'+ch(b'IHDR',struct.pack('>IIBBBBB',w,h,8,6,0,0,0))+ch(b'IDAT',zlib.compress(raw,6))+ch(b'IEND',b''))
sources={}
for d in RIGHT:
 for a in ('walk','attack'):sources[d,a]=read(SRC/f'hero-{a}-{d}.png')
sources['turnaround']=read(SRC/'hero-turnaround.png')
for a,n in COUNT.items():
 sheet=bytearray(W*n*H*8*4)
 for row,d in enumerate(ALL):
  original={'NW':'NE','W':'E','SW':'SE'}.get(d,d);flip=d not in RIGHT
  for f in range(n):
   if a in ('walk','attack'):
    sw,sh,lines=sources[original,a];cols=n;index=f
   elif a=='idle':
    sw,sh,lines=sources['turnaround'];cols=5;index=RIGHT.index(original)
   else:
    sw,sh,lines=sources[original,'walk'];cols=8
    index=([0,2,4,6][f] if a=='jump' else [1,3,0][f])
   lift=([0,10,26,9][f] if a=='jump' else [24,12,0][f] if a=='fall' else 0)
   cell=frame(lines,index*sw//cols,(index+1)*sw//cols,flip,lift,[0,1,2,1][f] if a=='idle' else 0)
   for y in range(H):
    src=y*W*4;dst=((row*H+y)*(W*n)+f*W)*4
    sheet[dst:dst+W*4]=cell[src:src+W*4]
 png(HERE/f'hero_{a}.png',W*n,H*8,sheet)
meta={'canvas':{'w':W,'h':H},'pivot':{'x':PIVOT[0],'y':PIVOT[1]},'directions':ALL,'animations':{a:{'frames':n,'fps':FPS[a],'loop':a in ('idle','walk')} for a,n in COUNT.items()}}
(HERE/'hero.json').write_text(json.dumps(meta,indent=2)+'\n')
shadow=bytearray(320*128*4)
for y in range(128):
 for x in range(320):
  if ((x-160)/90)**2+((y-64)/20)**2<=1:
   i=(y*320+x)*4;shadow[i:i+4]=bytes((35,35,31,95))
png(HERE/'hero_shadow.png',320,128,shadow)
