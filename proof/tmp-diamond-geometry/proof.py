import hashlib,json,resource,struct,subprocess,zlib
from pathlib import Path
repo=Path.cwd();root=repo/'.tmp/tmp-cli-proof';root.mkdir(exist_ok=False)
before=Path('/workspace/openyr-menu-audit-20261113-5/.tmp/menu-enter-scan/receipt.json')
after=repo/'.tmp/tmp-scan/receipt.json'
results=[]
def fixture(w,h):
    tile=struct.pack('<5I',1,1,w,h,20)+bytes(52)+bytes([1])*(w*h//2)
    palette=bytearray(768);palette[3]=63;data=tile+palette
    p=root/f'{w}x{h}.mix';p.write_bytes(struct.pack('<HI',2,len(data))+struct.pack('<3I',1,0,len(tile))+struct.pack('<3I',2,len(tile),len(palette))+data);return p

def limit():resource.setrlimit(resource.RLIMIT_AS,(512*1024**2,512*1024**2))
def run(label,receipt,worktree,asset,out,expected):
    command=['python3','scripts/proof_build.py','--repo',str(worktree),'run','--receipt',str(receipt),'--','render',str(asset)+':0x1',str(asset)+':0x2',str(out)]
    print('$ '+' '.join(command),flush=True)
    p=subprocess.run(command,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=15,preexec_fn=limit)
    print(p.stdout,end='',flush=True);print('exit',p.returncode,flush=True)
    assert p.returncode==expected,(label,p.returncode,p.stdout)
    results.append({'label':label,'command':command,'exit':p.returncode,'stdout':p.stdout})
    return p.stdout

def check_png(path,w,h):
    data=path.read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n';at=8;compressed=bytearray()
    while at<len(data):
        n=struct.unpack('>I',data[at:at+4])[0];kind=data[at+4:at+8];payload=data[at+8:at+8+n]
        assert struct.unpack('>I',data[at+8+n:at+12+n])[0]==zlib.crc32(kind+payload)
        if kind==b'IHDR':assert struct.unpack('>2I5B',payload)==(w,h,8,6,0,0,0)
        if kind==b'IDAT':compressed.extend(payload)
        at+=12+n
    pixels=zlib.decompress(compressed);assert len(pixels)==(1+4*w)*h
    opaque=0
    for y in range(h):
        row=pixels[y*(1+4*w):(y+1)*(1+4*w)];assert row[0]==0
        width=min(4*(y+1),4*(h-y-1));left=(w-width)//2
        for x in range(w):
            expected=bytes([255,0,0,255]) if left<=x<left+width else bytes(4)
            assert row[1+4*x:5+4*x]==expected,(w,h,x,y)
            opaque+=int(expected[3]>0)
    assert opaque==w*h//2
    return hashlib.sha256(data).hexdigest()

asset=fixture(2,1)
text=run('before malformed 2x1',before,Path('/workspace/openyr'),asset,root/'before-invalid.png',101)
assert 'panicked' in text and not (root/'before-invalid.png').exists()
for w,h in [(2,1),(6,3),(10,5)]:
    asset=fixture(w,h);out=root/f'after-invalid-{w}.png';marker=b'protected output bytes';out.write_bytes(marker)
    text=run(f'after malformed {w}x{h}',after,repo,asset,out,1)
    assert 'panicked' not in text and 'not an SHP or TMP' in text and out.read_bytes()==marker
for w,h in [(4,2),(8,4),(48,24),(60,30),(256,128)]:
    asset=fixture(w,h);a=root/f'before-{w}.png';b=root/f'after-{w}.png'
    run(f'before valid {w}x{h}',before,Path('/workspace/openyr'),asset,a,0)
    run(f'after valid {w}x{h}',after,repo,asset,b,0)
    digest=check_png(b,w,h);assert a.read_bytes()==b.read_bytes()
    print(f'{w}x{h}: exact centered red/transparent pixels and packed area; unchanged PNG SHA256 {digest}',flush=True)
(root/'results.json').write_text(json.dumps(results,indent=2)+'\n')
print('PASS: malformed headers fail cleanly; protected outputs survive; five valid raster controls are byte-identical.',flush=True)
