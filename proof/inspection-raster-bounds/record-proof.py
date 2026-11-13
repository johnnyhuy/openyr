import hashlib,json,resource,struct,subprocess,zlib
from pathlib import Path
repo=Path.cwd();root=repo/'.tmp/cli-proof';root.mkdir(exist_ok=False);results=[]
baseline=repo/'.tmp/baseline-cli/receipt.json';final=repo/'.tmp/final-cli/receipt.json'
def riff_asset(name,data):
 pal=bytearray(768);pal[3]=63;pal[7]=63;body=data+pal
 p=root/(name+'.mix');p.write_bytes(struct.pack('<HI',2,len(body))+struct.pack('<3I',1,0,len(data))+struct.pack('<3I',2,len(data),len(pal))+body);return str(p)+':0x1',str(p)+':0x2'
def tmp(cells,w=4,h=2):
 header=struct.pack('<4I',len(cells),1,w,h);base=16+4*len(cells);offsets=[];body=bytearray()
 for x,y,extra in cells:
  offsets.append(base+len(body));ex,ey,ew,eh=extra or (0,0,0,0)
  body.extend(struct.pack('<ii3Iii3I',x,y,52+w*h//2 if extra else 0,0,0,ex,ey,ew,eh,int(extra is not None))+bytes(12)+bytes([1])*(w*h//2)+bytes([2])*(ew*eh))
 return header+b''.join(struct.pack('<I',x) for x in offsets)+body

def empty_shp(w,h,n=1):return struct.pack('<4H',0,w,h,n)+bytes(24*n)
def colored_shp():
 d=bytearray(struct.pack('<4H',0,2,1,3))
 for offset in [80,0,82]:d.extend(struct.pack('<4HI',0,0,2,1,1)+bytes(8)+struct.pack('<I',offset))
 return bytes(d)+bytes([1,2,2,1])
def limits():
 resource.setrlimit(resource.RLIMIT_AS,(256*1024**2,256*1024**2));resource.setrlimit(resource.RLIMIT_CORE,(0,0))
def run(label,name,data,before=False,expected=0,protect=None):
 a,p=riff_asset(name,data);out=root/(label.replace(' ','-')+'.png')
 if protect is not None:out.write_bytes(protect)
 command=['python3','scripts/proof_build.py','--repo','/workspace/openyr' if before else str(repo),'run','--receipt',str(baseline if before else final),'--','render',a,p,str(out)]
 print('$ '+' '.join(command),flush=True)
 r=subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=10,preexec_fn=limits)
 print(r.stdout,end='',flush=True);print('exit',r.returncode,flush=True);assert r.returncode==expected,(label,r.stdout,r.returncode)
 if protect is not None and expected!=0:assert out.read_bytes()==protect
 results.append({'label':label,'command':command,'exit':r.returncode,'output':r.stdout,'destination_exists':out.exists(),'destination_sha256':hashlib.sha256(out.read_bytes()).hexdigest() if out.exists() else None})
 return out

def png(path,expected):
 b=path.read_bytes();assert b[:8]==b'\x89PNG\r\n\x1a\n';i=8;compressed=bytearray()
 while i<len(b):
  n=struct.unpack('>I',b[i:i+4])[0];kind=b[i+4:i+8];payload=b[i+8:i+8+n];assert zlib.crc32(kind+payload)==struct.unpack('>I',b[i+8+n:i+12+n])[0]
  if kind==b'IHDR':w,h,bits,color,_,_,_=struct.unpack('>2I5B',payload);assert(bits,color)==(8,6)
  if kind==b'IDAT':compressed.extend(payload)
  i+=12+n
 raw=zlib.decompress(compressed);assert len(raw)==h*(w*4+1)
 pixels=b''.join(raw[y*(w*4+1)+1:(y+1)*(w*4+1)] for y in range(h));assert all(raw[y*(w*4+1)]==0 for y in range(h));assert (w,h,pixels)==expected,(path,w,h)
 print(path.name,{'size':[w,h],'sha256':hashlib.sha256(b).hexdigest(),'independent_crc_rgba':'PASS'},flush=True);return b

red=bytes([255,0,0,255]);green=bytes([0,255,0,255]);clear=bytes(4)
def diamond(w,h):
 return b''.join(red if (w-min(4*(y+1),4*(h-y-1)))//2<=x<(w+min(4*(y+1),4*(h-y-1)))//2 else clear for y in range(h) for x in range(w))
controls=[('ordinary',tmp([(0,0,None)]),(4,2,red*4+clear*4)),('negative',tmp([(-4,-2,None)]),(4,2,red*4+clear*4)),('minimum',tmp([(-2**31,-2**31,None)]),(4,2,red*4+clear*4)),('extra',tmp([(0,0,(0,0,2,2))]),(4,2,green*2+red*2+green*2+clear*2)),('minimum-extra',tmp([(-2**31,-2**31,(-2**31,-2**31,2,2))]),(4,2,green*2+red*2+green*2+clear*2)),('empty-tmp',struct.pack('<5I',1,1,4,2,0),(1,1,clear)),('null-shp',empty_shp(0,0,2),(2,1,clear*2)),('colored-sheet',colored_shp(),(4,2,red+green+clear*2+green+red+clear*2)),('ordinary-256',tmp([(0,0,None)],256,128),(256,128,diamond(256,128)))]
for name,data,pixels in controls:
 before=run('before '+name,name,data,True);after=run('after '+name,name,data)
 assert png(before,pixels)==png(after,pixels)

for name,data,pixels in [('translated-max',tmp([(2**31-1,2**31-1,None)]),(4,2,red*4+clear*4)),('translated-max-extra',tmp([(2**31-1,2**31-1,(2**31-1,2**31-1,2,2))]),(4,2,green*2+red*2+green*2+clear*2)),('translated-max-256',tmp([(2**31-1,2**31-1,None)],256,128),(256,128,diamond(256,128)))]:
 before=run('before '+name,name,data,True);assert struct.unpack('>2I',before.read_bytes()[16:24])==(1,1)
 after=run('after '+name,name,data);png(after,pixels);assert before.read_bytes()!=after.read_bytes()

bad=[('huge-sheet',empty_shp(65535,65535)),('huge-span',tmp([(0,0,None),(2**31-5,0,None)])),('opposite-span',tmp([(-2**31,0,None),(2**31-1,0,None)])),('vertical-span',tmp([(0,-2**31,None),(0,2**31-1,None)])),('far-extra',tmp([(0,0,(2**31-1,-2**31,1,1))])),('zero-area-huge-extra',tmp([(0,0,(0,0,2**32-1,0))])),('allocation-limit',empty_shp(8192,8192))]
for name,data in [bad[0],bad[1],bad[-1]]:
 run('before '+name,name,data,True,250,protect=b'preserve earlier PNG')
for name,data in bad:
 for exists in [True,False]:
  path=run('after '+name+('-existing' if exists else '-absent'),name,data,False,1,protect=b'preserve earlier PNG' if exists else None)
  if not exists:assert not path.exists()
  record=results[-1];assert 'inspection raster' in record['output'] and 'panicked' not in record['output'] and 'memory allocation of' not in record['output']
(root/'results.json').write_text(json.dumps(results,indent=2)+'\n')
print('PASS: nine unchanged valid PNG controls, three corrected extreme translations, three bounded baseline aborts, fourteen clean final rejection/output-preservation cases.',flush=True)
