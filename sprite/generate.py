"""Original, deterministic 1x pixel sprites for the art-guide prototype."""
import json, math, struct, zlib
from pathlib import Path

OUT=Path(__file__).parent
P={'outline':(43,44,39),'dark':(62,78,47),'grass':(125,153,43),'leaf':(166,189,51),'sun':(203,209,75),'cream':(238,225,172),'dirt':(139,102,63),'sand':(211,176,108),'rock':(102,107,100),'rocklight':(166,167,144),'wood':(101,68,47),'coat':(96,70,63),'coatlight':(139,99,74),'gold':(216,162,72),'skin':(220,169,126),'hair':(226,224,204),'hairshade':(171,173,158),'blue':(74,120,151),'bluelight':(119,170,177),'roof':(139,68,53),'rooflight':(187,104,65),'grassdark':(91,129,38),'grasslight':(188,203,62),'sandlight':(230,195,129),'rockdark':(77,82,79),'white':(246,239,205)}
class Im:
 def __init__(self,w,h): self.w=w; self.h=h; self.b=bytearray(w*h*4)
 def dot(self,x,y,c,a=255):
  if 0<=x<self.w and 0<=y<self.h:
   i=(y*self.w+x)*4; self.b[i:i+4]=bytes((*P[c],a))
 def rect(self,x,y,w,h,c,a=255):
  for yy in range(y,y+h):
   for xx in range(x,x+w): self.dot(xx,yy,c,a)
 def oval(self,x,y,rx,ry,c,a=255):
  for yy in range(y-ry,y+ry+1):
   for xx in range(x-rx,x+rx+1):
    if (xx-x)**2*ry*ry+(yy-y)**2*rx*rx<=rx*rx*ry*ry:self.dot(xx,yy,c,a)
 def paste(self,o,x,y):
  for yy in range(o.h):
   for xx in range(o.w):
    i=(yy*o.w+xx)*4
    if o.b[i+3]:
     j=((y+yy)*self.w+x+xx)*4; self.b[j:j+4]=o.b[i:i+4]
 def save(self,n):
  raw=b''.join(b'\0'+self.b[y*self.w*4:(y+1)*self.w*4] for y in range(self.h))
  def ch(t,d):return struct.pack('>I',len(d))+t+d+struct.pack('>I',zlib.crc32(t+d)&0xffffffff)
  (OUT/n).write_bytes(b'\x89PNG\r\n\x1a\n'+ch(b'IHDR',struct.pack('>IIBBBBB',self.w,self.h,8,6,0,0,0))+ch(b'IDAT',zlib.compress(raw,9))+ch(b'IEND',b''))
def hero(d,a,f,npc=False):
 c=Im(64,96); count={'idle':4,'walk':8,'attack':5,'jump':4,'fall':3}[a]
 bob=round(math.sin(f*2*math.pi/count)*2) if a in ('idle','walk') else 0
 lift=round(8*math.sin(f*math.pi/3)) if a=='jump' else ([8,4,0][f] if a=='fall' else 0)
 step=round(math.sin(f*2*math.pi/count)*4) if a=='walk' else 0
 x=32+({'S':0,'SE':2,'E':4,'NE':2,'N':0}[d]); foot=88-lift; back=d in ('N','NE')
 c.rect(x-10-step,foot-8,7,8,'outline'); c.rect(x+3+step,foot-8,7,8,'outline')
 c.rect(x-9-step,foot-10,5,8,'wood'); c.rect(x+4+step,foot-10,5,8,'wood')
 c.rect(x-13,foot-43+bob,26,33,'outline'); c.rect(x-11,foot-41+bob,22,28,'blue' if npc else 'coat')
 c.rect(x-8,foot-37+bob,16,20,'bluelight' if npc else 'coatlight'); c.rect(x-10,foot-22+bob,20,2,'gold')
 for xx in (x-16,x+11):
  c.rect(xx,foot-38+bob,5,18,'outline'); c.rect(xx+1,foot-37+bob,3,14,'blue' if npc else 'coatlight'); c.rect(xx+1,foot-23+bob,3,3,'skin')
 if a=='attack':
  reach=[0,4,9,5,1][f]; c.rect(x+16,foot-39-reach,3,19,'rocklight'); c.rect(x+15,foot-40-reach,5,2,'gold')
 c.oval(x,foot-54+bob,12,12,'outline'); c.oval(x,foot-54+bob,10,10,'skin')
 c.oval(x,foot-62+bob,12,9,'hairshade'); c.rect(x-10,foot-67+bob,20,7,'hair')
 c.rect(x-12,foot-64+bob,4,9,'hair'); c.rect(x+8,foot-64+bob,4,8,'hair')
 if back:c.rect(x-8,foot-57+bob,16,5,'hairshade')
 else:
  c.rect(x-5,foot-54+bob,2,2,'outline')
  if d!='E':c.rect(x+4,foot-54+bob,2,2,'outline')
 return c
dirs=['S','SE','E','NE','N']; anims={'idle':(4,5,True),'walk':(8,9,True),'attack':(5,11,False),'jump':(4,8,False),'fall':(3,8,False)}
for a,(frames,fps,loop) in anims.items():
 for npc in (False,True):
  s=Im(frames*64,480)
  for row,d in enumerate(dirs):
   for col in range(frames):s.paste(hero(d,a,col,npc),col*64,row*96)
  s.save(('npc' if npc else 'hero')+'_'+a+'.png')
meta={'canvas':{'w':64,'h':96},'pivot':{'x':32,'y':88},'directions':dirs,'mirror_for':{'SW':'SE','W':'E','NW':'NE'},'animations':{a:{'frames':v[0],'fps':v[1],'loop':v[2]} for a,v in anims.items()}}
for n in ('hero','npc'):(OUT/(n+'.json')).write_text(json.dumps(meta,indent=2)+'\n')
s=Im(64,32); s.oval(32,16,18,5,'outline',100);s.save('hero_shadow.png')
def tile(mat,v=0):
 c=Im(64,32); colors={'grass':('grass','leaf','sun'),'dirt':('sand','dirt','sandlight'),'stone':('rocklight','rock','cream'),'floor':('sand','dirt','cream')}[mat]
 for y in range(32):
  for x in range(64):
   z=abs(x-32)/32+abs(y-16)/16
   if z<1:
    noise=((x*37+y*53+v*97)^(x*y*3))%31
    c.dot(x,y,colors[1] if noise<7 else colors[2] if noise>27 else colors[0])
 for k in range(17):
  x=7+(k*19+v*11)%50;y=5+(k*7+v*3)%23
  if abs(x-32)/32+abs(y-16)/16<.7:
   if mat=='grass':
    c.rect(x,y,2,1,'grassdark');c.rect(x+1,y-2,1,2,'grasslight')
    if k==3+v:c.rect(x+2,y-3,2,2,'white')
   elif mat=='dirt':c.rect(x,y,3,1,'sandlight' if k%2 else 'dirt')
   else:c.rect(x,y,3,2,'rock' if k%2 else 'cream')
 return c
for mat in ('grass','dirt','stone'):
 for v in range(3):tile(mat,v).save(f'tile_{mat}_{v}.png')
 tile(mat).save(f'elevation_{mat}_top.png')
 face=Im(64,48);face.paste(tile(mat),0,0)
 for y in range(16,32):
  for x in range(64):
   if abs(x-32)/32+abs(y-16)/16<1:face.dot(x,y+16,'dirt' if mat!='stone' else 'rock')
 face.save(f'elevation_{mat}_face.png')
 ramp=tile(mat,1);ramp.rect(29,8,6,2,'cream');ramp.save(f'elevation_{mat}_ramp.png')
tile('floor').save('house_floor.png')
def wall(k):
 c=Im(64,64)
 if k=='horizontal':c.rect(5,30,54,24,'outline');c.rect(7,32,50,19,'rocklight')
 elif k=='vertical':c.rect(27,7,18,50,'outline');c.rect(29,9,14,46,'rocklight')
 else:c.rect(25,25,22,31,'outline');c.rect(27,27,18,27,'rocklight')
 return c
for k in ('horizontal','vertical','corner_ne','corner_nw','corner_se','corner_sw'):wall(k).save('wall_'+k+'.png')
wall('horizontal').save('house_wall.png')
door=wall('horizontal');door.rect(24,32,16,25,'outline');door.rect(26,34,12,22,'wood');door.rect(35,44,2,2,'gold');door.save('house_door.png')
roof=Im(96,64)
for y in range(8,48):
 span=round((y-8)*1.2) if y<28 else round((48-y)*2.4)
 roof.rect(48-span,y,span*2,1,'rooflight' if (y+2)%6==0 else 'roof')
 if y>14 and span>6:
  for x in range(48-span+3,48+span-3,9):
   roof.rect(x+(y//3)%2*3,y,4,1,'rooflight' if y%3==0 else 'wood')
roof.save('house_roof.png')
tree=Im(96,128);tree.rect(44,70,11,51,'outline');tree.rect(46,71,7,48,'wood')
for x,y,r,col in [(30,55,24,'dark'),(65,53,25,'grass'),(48,32,29,'leaf'),(26,37,16,'leaf'),(70,30,19,'sun'),(45,18,17,'sun')]:tree.oval(x,y,r,round(r*.7),col)
tree.save('prop_tree.png')
rock=Im(64,48);rock.oval(32,31,23,12,'outline');rock.oval(31,27,20,12,'rock');rock.rect(18,19,20,5,'rocklight');rock.save('prop_rock.png')
bush=Im(64,48)
for x,y,r,col in [(19,30,15,'dark'),(43,29,16,'grass'),(31,22,18,'leaf'),(20,18,10,'sun'),(43,17,11,'leaf')]:bush.oval(x,y,r,round(r*.7),col)
for x,y in [(16,20),(38,14),(46,28)]:bush.rect(x,y,3,3,'cream')
bush.save('prop_bush.png')
(OUT/'shared.pal').write_text('JASC-PAL\n0100\n'+str(len(P))+'\n'+''.join('%d %d %d\n'%rgb for rgb in P.values()))
