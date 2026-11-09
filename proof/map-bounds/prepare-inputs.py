from pathlib import Path
import base64, struct
root=Path('/workspace/scratch/map-bounds-294-proof')
root.mkdir(parents=True,exist_ok=True)

def text_map(x,y):
    raw=struct.pack('<HHiBBB',x,y,0,0,0,0)+bytes(4)
    lzo=bytes([17+len(raw)])+raw+bytes([0x11,0,0])
    chunk=struct.pack('<HH',len(lzo),len(raw))+lzo
    return '[Map]\nTheater=TEMPERATE\nSize=0,0,2,2\nLocalSize=0,0,2,2\n[IsoMapPack5]\n1='+base64.b64encode(chunk).decode()+'\n'

def template(tx,ty,ex,ey):
    data=struct.pack('<5I',1,1,8,4,20)
    data+=struct.pack('<iiIIIiiiiI',tx,ty,68,0,0,ex,ey,1,1,1)
    data+=bytes([0,13,0,1,1,1,1,1,1,0,0,0])+bytes([1])*16+bytes([2])
    assert len(data)==89
    return data
palette=bytearray(768)
palette[3:6]=bytes([10,20,30]);palette[6:9]=bytes([40,50,60])
for name,params,cell in [
    ('small',(0,0,0,0),(1,1)),
    ('cancel-offset',(-(2**31),-(2**31),-(2**31),-(2**31)),(1,1)),
    ('huge-extra',(0,0,2**31-1,-(2**31)),(1,1)),
    ('huge-coordinates',(0,0,0,0),(65535,65535)),
]:
    path=root/'inputs'/name;path.mkdir(parents=True,exist_ok=True)
    (path/'TEMPERATMD.INI').write_text('[TileSet0000]\nFileName=Clear\nTilesInSet=1\n')
    (path/'ISOTEM.PAL').write_bytes(palette)
    (path/'RULESMD.INI').write_text('[General]\n')
    (path/'ARTMD.INI').write_text('[General]\n')
    (path/'CLEAR01.TEM').write_bytes(template(*params))
    (path/'SMALL.MAP').write_text(text_map(*cell))
