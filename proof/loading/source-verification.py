import hashlib, struct, sys
from pathlib import Path
shp, pal = (Path(p).read_bytes() for p in sys.argv[1:])
assert struct.unpack_from("<4H", shp) == (0, 800, 600, 1)
assert struct.unpack_from("<4H", shp, 8) == (0, 0, 800, 600)
assert shp[16] == 2 and len(pal) == 768
rows = []
pos = 32
for y in range(600):
    size = struct.unpack_from("<H", shp, pos)[0]
    assert size == 802
    rows.append(shp[pos + 2:pos + size])
    pos += size
assert pos == len(shp)
hash_value = 0xcbf29ce484222325
for y in range(32, 345):
    for x in range(450, 766):
        index = rows[y][x]
        rgb = [(v << 2) | (v >> 4) for v in pal[index * 3:index * 3 + 3]] if index else [0, 0, 0]
        for byte in [rgb[0] & 248, rgb[1] & 252, rgb[2] & 248, 255]:
            hash_value = ((hash_value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
assert hash_value == 0x97061e52e7661e85
print("Country panel: 98908 pixels, FNV-1a", format(hash_value, "016x"))
print("SHP SHA256:", hashlib.sha256(shp).hexdigest())
print("PAL SHA256:", hashlib.sha256(pal).hexdigest())
print("Source art comparison only. Original runtime layout and timing are not established.")
