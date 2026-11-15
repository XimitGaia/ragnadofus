"""Convert approved concept cutouts into crisp native-scale game sprites."""
import json
import math
import struct
import zlib
from pathlib import Path
from generate import Im, OUT, anims, dirs

SOURCE=OUT/'source'
def read_png(path):
    b=path.read_bytes(); w,h=struct.unpack('>II',b[16:24]); i=8; payload=b''; channels=4 if b[25]==6 else 3
    while i<len(b):
        n=struct.unpack('>I',b[i:i+4])[0]; kind=b[i+4:i+8]
        if kind==b'IDAT': payload+=b[i+8:i+8+n]
        i+=n+12
    raw=zlib.decompress(payload); stride=w*channels; rows=[]; off=0; previous=bytearray(stride)
    for _ in range(h):
        mode=raw[off]; off+=1; row=bytearray(raw[off:off+stride]); off+=stride
        for x in range(stride):
            a=row[x-channels] if x>=channels else 0; up=previous[x]; ul=previous[x-channels] if x>=channels else 0
            if mode==1: row[x]=(row[x]+a)&255
            elif mode==2: row[x]=(row[x]+up)&255
            elif mode==3: row[x]=(row[x]+((a+up)//2))&255
            elif mode==4:
                p=a+up-ul; pa=abs(p-a); pb=abs(p-up); pc=abs(p-ul)
                row[x]=(row[x]+(a if pa<=pb and pa<=pc else up if pb<=pc else ul))&255
        if channels==3: row=bytearray(v for x in range(w) for v in (*row[x*3:x*3+3],255))
        rows.append(row); previous=row if channels==4 else raw[off-stride:off]
    return w,h,rows

def bbox(rows,x0,x1):
    h=len(rows); points=[]
    for y in range(h):
        row=rows[y]
        for x in range(x0,x1,3):
            if row[4*x+3]>=245:points.append((x,y))
    return (min(x for x,y in points),min(y for x,y in points),max(x for x,y in points)+1,max(y for x,y in points)+1)

def palette(rows,box,limit=32):
    x0,y0,x1,y1=box; colors=[]
    for y in range(y0,y1,5):
        r=rows[y]
        for x in range(x0,x1,5):
            i=x*4
            if r[i+3]>=245:colors.append(tuple(r[i:i+3]))
    groups=[colors]
    while len(groups)<limit:
        ix=max(range(len(groups)),key=lambda j:len(groups[j])*(max(max(c[k] for c in groups[j])-min(c[k] for c in groups[j]) for k in range(3))+1))
        group=groups.pop(ix)
        if len(group)<2:groups.append(group);break
        channel=max(range(3),key=lambda k:max(c[k] for c in group)-min(c[k] for c in group))
        group.sort(key=lambda c:c[channel]); mid=len(group)//2;groups.extend((group[:mid],group[mid:]))
    return [tuple(sum(c[k] for c in g)//len(g) for k in range(3)) for g in groups]

def cut(rows,box,w,h,pal):
    x0,y0,x1,y1=box; img=Im(w,h); scale=min(w/(x1-x0),h/(y1-y0)); dw=max(1,round((x1-x0)*scale)); dh=max(1,round((y1-y0)*scale)); ox=(w-dw)//2; oy=h-dh
    cache={}
    for y in range(dh):
        sy=min(y1-1,y0+int(y/scale)); row=rows[sy]
        for x in range(dw):
            sx=min(x1-1,x0+int(x/scale)); i=sx*4
            if row[i+3]<245:continue
            rgb=tuple(row[i:i+3]); q=cache.get(rgb)
            if q is None:
                q=min(pal,key=lambda c:sum((c[k]-rgb[k])**2 for k in range(3)));cache[rgb]=q
            j=((oy+y)*w+ox+x)*4;img.b[j:j+4]=bytes((*q,255))
    return img

def put(dst,src,ox,oy,recolor=False,shift=0,breath=0):
    for y in range(src.h):
        for x in range(src.w):
            i=(y*src.w+x)*4
            if src.b[i+3]==0:continue
            xx=ox+x+(shift if y>src.h*.65 else 0)
            yy=oy+y-round(breath*(1-y/max(1,src.h-1)))
            if not (0<=xx<dst.w and 0<=yy<dst.h):continue
            rgb=tuple(src.b[i:i+3]);out=rgb
            if recolor and rgb[0]>rgb[2]*1.12 and rgb[1]<rgb[0]*.91 and rgb[0]<200:
                out=(round(rgb[0]*.55),min(255,round(rgb[1]*1.26)),min(255,round(rgb[2]*1.52)))
            j=(yy*dst.w+xx)*4;dst.b[j:j+4]=bytes((*out,255))

hero_w,hero_h,hero_rows=read_png(SOURCE/'hero-turnaround.png')
hero_parts=[]; hero_pals=[]
for k in range(5):
    box=bbox(hero_rows,k*hero_w//5,(k+1)*hero_w//5)
    pal=palette(hero_rows,box);hero_pals.append(pal);hero_parts.append(cut(hero_rows,box,46,70,pal))
walk_parts=[]
for direction in dirs:
    width,height,rows=read_png(SOURCE/f'hero-walk-{direction}.png')
    pal=palette(rows,bbox(rows,0,width))
    poses=[]
    for frame in range(8):
        box=bbox(rows,frame*width//8,(frame+1)*width//8)
        poses.append(cut(rows,box,46,70,pal))
    walk_parts.append(poses)
attack_parts=[]
for direction in dirs:
    width,height,rows=read_png(SOURCE/f'hero-attack-{direction}.png')
    pal=palette(rows,bbox(rows,0,width))
    poses=[]
    for frame in range(5):
        box=bbox(rows,frame*width//5,(frame+1)*width//5)
        poses.append(cut(rows,box,60,70,pal))
    attack_parts.append(poses)
for anim,(frames,fps,loop) in anims.items():
    for npc in (False,True):
        sheet=Im(64*frames,480)
        for row,base_part in enumerate(hero_parts):
            for f in range(frames):
                lift=0
                if anim=='jump':lift=[0,4,10,4][f]
                if anim=='fall':lift=[9,5,0][f]
                part=(walk_parts[row][f] if anim=='walk' else
                      attack_parts[row][f] if anim=='attack' else
                      walk_parts[row][[0,2,4,6][f]] if anim=='jump' else
                      walk_parts[row][[1,3,0][f]] if anim=='fall' else base_part)
                stride=0
                ox=f*64+(2 if anim=='attack' else 9);oy=row*96+19-lift
                put(sheet,part,ox,oy,npc,stride,[0,1,2,1][f] if anim=='idle' else 0)
        sheet.save(('npc' if npc else 'hero')+'_'+anim+'.png')

assets={
 'tree-summer':('prop_tree.png',96,128),
 'tree-autumn':('prop_tree_autumn.png',96,128),
 'plant-bush':('prop_bush.png',64,48),
 'plant-fern':('prop_fern.png',64,48),
 'rock-moss':('prop_rock.png',64,48),
 'rock-field':('prop_rock_field.png',64,48),
 'monster':('monster_forest.png',96,96),
}
all_pal=[]
for src,(name,w,h) in assets.items():
 sw,sh,rows=read_png(SOURCE/(src+'.png'));box=bbox(rows,0,sw);pal=palette(rows,box);all_pal+=pal
 image=cut(rows,box,w,h,pal);image.save(name)
 # A source palette is kept for recolor and future indexed import.
 (OUT/(src+'.pal')).write_text('JASC-PAL\n0100\n'+str(len(pal))+'\n'+''.join('%d %d %d\n'%c for c in pal))
 if src=='monster':
    sheet=Im(96*4,96)
    for frame in range(4):put(sheet,image,frame*96,0 if frame%2==0 else -1)
    sheet.save('monster_idle.png')
    (OUT/'monster.json').write_text(json.dumps({'canvas':{'w':96,'h':96},'pivot':{'x':48,'y':88},'animations':{'idle':{'frames':4,'fps':5,'loop':True}}},indent=2)+'\n')
(OUT/'hero.pal').write_text('JASC-PAL\n0100\n'+str(len(hero_pals[0]))+'\n'+''.join('%d %d %d\n'%c for c in hero_pals[0]))

castle_views={
    'castle-cardinal.png':['front','right','back','left'],
    'castle-diagonal.png':['front_right','back_right','back_left','front_left'],
}
for source,names in castle_views.items():
    width,height,rows=read_png(SOURCE/source)
    for index,name in enumerate(names):
        box=bbox(rows,index*width//4,(index+1)*width//4)
        pal=palette(rows,box,64)
        cut(rows,box,192,192,pal).save(f'castle_{name}.png')
interior_w,interior_h,interior_rows=read_png(SOURCE/'castle-interior.png')
interior_box=bbox(interior_rows,0,interior_w)
interior_pal=palette(interior_rows,interior_box,64)
cut(interior_rows,interior_box,256,192,interior_pal).save('castle_interior.png')
(OUT/'castle.json').write_text(json.dumps({
    'exterior_canvas':{'w':192,'h':192},'exterior_pivot':{'x':96,'y':191},
    'views':['front','front_right','right','back_right','back','back_left','left','front_left'],
    'interior':'castle_interior.png','interior_canvas':{'w':256,'h':192},
    'rooms':1,'entry_side':'front'
},indent=2)+'\n')
