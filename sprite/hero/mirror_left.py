import struct,zlib
from pathlib import Path
p=Path(__file__).parent
for who in ('hero','npc'):
 d=p if who=='hero' else p.parent/'npc'
 for anim in ('idle','walk','attack','jump','fall'):
  b=(d/f'{who}_{anim}.png').read_bytes();w,h=struct.unpack('>II',b[16:24]);i=8;c=b''
  while i<len(b):
   n=int.from_bytes(b[i:i+4],'big');t=b[i+4:i+8]
   if t==b'IDAT':c+=b[i+8:i+8+n]
   i+=n+12
  raw=zlib.decompress(c);rows=[raw[y*(w*4+1)+1:(y+1)*(w*4+1)] for y in range(h)]
  for name,source in [('SW',1),('W',2),('NW',3)]:
   frames=w//64;out=[]
   for y in range(96):
    row=rows[source*96+y];line=bytearray()
    for f in range(frames):
     cell=row[f*256:(f+1)*256]
     for x in range(63,-1,-1):line+=cell[x*4:x*4+4]
    out.append(b'\0'+line)
   def chunk(t,v):return len(v).to_bytes(4,'big')+t+v+(zlib.crc32(t+v)&0xffffffff).to_bytes(4,'big')
   png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,96,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b''.join(out),9))+chunk(b'IEND',b'')
   (d/f'{who}_{anim}_{name}.png').write_bytes(png)
