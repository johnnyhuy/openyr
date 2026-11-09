from pathlib import Path
import struct, base64
root = Path('/workspace/scratch/tmp-293-proof/input-valid')
root.mkdir(exist_ok=True)
entry = struct.pack('<HHiBBB', 2, 3, 0, 0, 5, 0) + bytes(4)
lzo = bytes([17 + len(entry)]) + entry + bytes([0x11, 0, 0])
chunk = struct.pack('<HH', len(lzo), len(entry)) + lzo
(root / 'SYNTHETIC.MAP').write_text('[Map]\nTheater=TEMPERATE\nSize=0,0,8,8\nLocalSize=0,0,8,8\n[IsoMapPack5]\n1=' + base64.b64encode(chunk).decode() + '\n')
(root / 'TEMPERATMD.INI').write_text('[TileSet0000]\nFileName=Clear\nTilesInSet=1\n')
for name in ['RULESMD.INI', 'ARTMD.INI']:
    (root / name).write_text('; Synthetic empty definitions\n')
color = bytearray(768)
color[3:6] = bytes([63,63,0])
(root / 'ISOTEM.PAL').write_bytes(color)
tmp = bytearray(struct.pack('<IIIII',1,1,60,30,20) + bytes(52))
tmp[60:63] = bytes([3,14,7])
tmp.extend(bytes([1]) * 900)
(root / 'CLEAR01.TEM').write_bytes(tmp)
