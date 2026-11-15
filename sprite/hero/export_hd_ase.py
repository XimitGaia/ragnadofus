"""Create editable Aseprite timelines for the detailed hero sheets."""
import json,struct,zlib
from pathlib import Path
p=Path(__file__).parent;meta=json.loads((p/'hero.json').read_text())
def w(n):return struct.pack('<H',n)
def d(n):return struct.pack('<I',n)
def chunk(t,b):return d(len(b)+6)+w(t)+b
for anim,info in meta['animations'].items():
 path=p/f'hero_{anim}.png';data=path.read_bytes();width,height=struct.unpack('>II',data[16:24]);pos=8;z=b''
 while pos<len(data):
  n=struct.unpack('>I',data[pos:pos+4])[0]
  if data[pos+4:pos+8]==b'IDAT':z+=data[pos+8:pos+8+n]
  pos+=n+12
 raw=zlib.decompress(z);rows=[raw[y*(width*4+1)+1:(y+1)*(width*4+1)] for y in range(height)]
 frames=info['frames'];cw=320;ch=4096;assert width==cw*frames and height==ch
 name=path.stem.encode();layer=w(3)+w(0)+w(0)+w(0)+w(0)+w(0)+bytes([255,0,0,0])+w(len(name))+name
 out=[]
 for f in range(frames):
  pixels=b''.join(row[f*cw*4:(f+1)*cw*4] for row in rows)
  cel=w(0)+struct.pack('<hh',0,0)+bytes([255])+w(0)+struct.pack('<h',0)+bytes(5)+w(cw)+w(ch)+pixels
  body=(chunk(0x2004,layer) if f==0 else b'')+chunk(0x2005,cel)
  count=2 if f==0 else 1
  out.append(d(16+len(body))+w(0xF1FA)+w(count)+w(round(1000/info['fps']))+w(0)+d(count)+body)
 payload=b''.join(out);header=bytearray(128)
 struct.pack_into('<IHHHHH',header,0,128+len(payload),0xA5E0,frames,cw,ch,32)
 struct.pack_into('<H',header,18,round(1000/info['fps']))
 struct.pack_into('<H',header,32,256);header[34]=1;header[35]=1
 path.with_suffix('.ase').write_bytes(header+payload)
path=p/'hero_shadow.png';data=path.read_bytes();width,height=struct.unpack('>II',data[16:24]);pos=8;z=b''
while pos<len(data):
 n=struct.unpack('>I',data[pos:pos+4])[0]
 if data[pos+4:pos+8]==b'IDAT':z+=data[pos+8:pos+8+n]
 pos+=n+12
raw=zlib.decompress(z);pixels=b''.join(raw[y*(width*4+1)+1:(y+1)*(width*4+1)] for y in range(height))
name=b'hero_shadow';layer=w(3)+w(0)+w(0)+w(0)+w(0)+w(0)+bytes([255,0,0,0])+w(len(name))+name
cel=w(0)+struct.pack('<hh',0,0)+bytes([255])+w(0)+struct.pack('<h',0)+bytes(5)+w(width)+w(height)+pixels
body=chunk(0x2004,layer)+chunk(0x2005,cel);frame=d(16+len(body))+w(0xF1FA)+w(2)+w(100)+w(0)+d(2)+body
header=bytearray(128);struct.pack_into('<IHHHHH',header,0,128+len(frame),0xA5E0,1,width,height,32)
struct.pack_into('<H',header,18,100);struct.pack_into('<H',header,32,256);header[34]=1;header[35]=1
path.with_suffix('.ase').write_bytes(header+frame)
