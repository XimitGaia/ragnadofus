"""Export editable Aseprite files; hero/NPC sheets become real timeline frames."""
import json
import struct
import zlib
from pathlib import Path

ROOT=Path(__file__).parent
def word(n): return struct.pack('<H',n)
def dword(n): return struct.pack('<I',n)
def chunk(kind,payload): return dword(len(payload)+6)+word(kind)+payload

def png_pixels(path):
    data=path.read_bytes(); w,h=struct.unpack('>II',data[16:24]); pos=8;compressed=b''
    while pos<len(data):
        length=struct.unpack('>I',data[pos:pos+4])[0]
        if data[pos+4:pos+8]==b'IDAT':compressed+=data[pos+8:pos+8+length]
        pos+=length+12
    raw=zlib.decompress(compressed)
    assert all(raw[y*(w*4+1)]==0 for y in range(h)),path
    return w,h,[raw[y*(w*4+1)+1:(y+1)*(w*4+1)] for y in range(h)]

def export(path):
    w,h,rows=png_pixels(path)
    character=path.name.startswith(('hero_','npc_')) and path.stem!='hero_shadow'
    if character:
        who,animation=path.stem.split('_',1)
        info=json.loads((ROOT/f'{who}.json').read_text())['animations'][animation]
        frames=info['frames'];fps=info['fps'];canvas_w=64;canvas_h=480
        assert w==frames*64 and h==canvas_h
    else:
        frames=1;fps=10;canvas_w=w;canvas_h=h
    name=path.stem.encode('utf8')
    layer=word(3)+word(0)+word(0)+word(0)+word(0)+word(0)+bytes([255,0,0,0])+word(len(name))+name
    frame_data=[]
    for f in range(frames):
        pixels=b''.join(row[f*64*4:(f+1)*64*4] for row in rows) if character else b''.join(rows)
        cel=word(0)+struct.pack('<hh',0,0)+bytes([255])+word(0)+struct.pack('<h',0)+bytes(5)+word(canvas_w)+word(canvas_h)+pixels
        chunks=([chunk(0x2004,layer)] if f==0 else [])+[chunk(0x2005,cel)]
        body=b''.join(chunks)
        frame_data.append(dword(16+len(body))+word(0xF1FA)+word(len(chunks))+word(round(1000/fps))+word(0)+dword(len(chunks))+body)
    data=b''.join(frame_data)
    header=bytearray(128)
    struct.pack_into('<IHHHHH',header,0,128+len(data),0xA5E0,frames,canvas_w,canvas_h,32)
    struct.pack_into('<H',header,18,round(1000/fps))
    struct.pack_into('<H',header,32,256)
    header[34]=1;header[35]=1
    path.with_suffix('.ase').write_bytes(header+data)

for path in ROOT.glob('*.png'):
    export(path)
