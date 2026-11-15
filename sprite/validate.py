"""Check sprite contract from docs/arte/guia-de-arte.md."""
import json
import struct
import zlib
import hashlib
from pathlib import Path

p=Path(__file__).parent
folders={'hero','npc','monster','castle','terrain','architecture','props'}
pngs=[q for q in p.rglob('*.png') if q.relative_to(p).parts[0] in folders]
assert len(pngs)==56, len(pngs)
for path in pngs:
    b=path.read_bytes(); assert b[:8]==b'\x89PNG\r\n\x1a\n',path
    w,h=struct.unpack('>II',b[16:24]); assert b[25]==6,path
    pos=8;payload=b''
    while pos<len(b):
        n=struct.unpack('>I',b[pos:pos+4])[0]
        if b[pos+4:pos+8]==b'IDAT':payload+=b[pos+8:pos+8+n]
        pos+=n+12
    raw=zlib.decompress(payload); assert len(raw)==h*(w*4+1),path
    if path.name!='hero_shadow.png':
        assert all(raw[y*(w*4+1)+1+4*x+3] in (0,255) for y in range(h) for x in range(w)),path
    ase=path.with_suffix('.ase');a=ase.read_bytes()
    assert struct.unpack('<I',a[:4])[0]==len(a) and a[4:6]==bytes.fromhex('e0a5'),ase
for who in ('hero','npc'):
    meta=json.loads((p/who/f'{who}.json').read_text())
    assert meta['canvas']=={'w':64,'h':96} and meta['pivot']=={'x':32,'y':88}
    assert meta['directions']==['S','SE','E','NE','N']
    for anim,desc in meta['animations'].items():
        b=(p/who/f'{who}_{anim}.png').read_bytes()
        assert struct.unpack('>II',b[16:24])==(64*desc['frames'],480)
        pos=8;compressed=b''
        while pos<len(b):
            n=struct.unpack('>I',b[pos:pos+4])[0]
            if b[pos+4:pos+8]==b'IDAT':compressed+=b[pos+8:pos+8+n]
            pos+=n+12
        raw=zlib.decompress(compressed);stride=64*desc['frames']*4+1
        hashes=[]
        for f in range(desc['frames']):
            pixels=b''.join(raw[y*stride+1+f*256:y*stride+1+(f+1)*256] for y in range(480))
            hashes.append(hashlib.sha256(pixels).digest())
        assert len(set(hashes))>=min(3,desc['frames']),(who,anim)
        ase=(p/who/f'{who}_{anim}.ase').read_bytes()
        assert struct.unpack('<HHH',ase[6:12])==(desc['frames'],64,480),anim
for mat in ('grass','dirt','stone'):
    for v in range(3):
        b=(p/'terrain'/'ground'/f'tile_{mat}_{v}.png').read_bytes()
        assert struct.unpack('>II',b[16:24])==(64,32)
castle=json.loads((p/'castle'/'castle.json').read_text())
assert len(castle['views'])==8 and castle['rooms']==1
assert all((p/'castle'/f'castle_{view}.png').exists() for view in castle['views'])
assert len([q for q in p.rglob('*.ase') if q.relative_to(p).parts[0] in folders])==len(pngs)
print(f'{len(pngs)} PNGs and matching ASE files valid')
