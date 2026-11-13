import hashlib,json,struct,subprocess,wave,resource
from pathlib import Path
repo=Path.cwd();root=repo/'.tmp/cli-proof';root.mkdir(exist_ok=False)
baseline=Path('/workspace/openyr-core-review-20261113-6/.tmp/tmp-rebased-scan/receipt.json')
final=repo/'.tmp/final-cli/receipt.json';records=[]
def run(label,receipt,worktree,args,code):
    cmd=['python3','scripts/proof_build.py','--repo',str(worktree),'run','--receipt',str(receipt),'--']+list(map(str,args))
    print('$ '+' '.join(cmd),flush=True)
    def limits():resource.setrlimit(resource.RLIMIT_AS,(512*1024**2,512*1024**2))
    p=subprocess.run(cmd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=30,preexec_fn=limits)
    print(p.stdout,end='',flush=True);print('exit',p.returncode,flush=True)
    assert p.returncode==code,(label,p.stdout,p.returncode)
    records.append({'label':label,'command':cmd,'exit':p.returncode,'output':p.stdout})
    return p.stdout

def riff(chunks):
    body=b'WAVE'+b''.join(k+struct.pack('<I',len(v))+v+(b'\0' if len(v)%2 else b'') for k,v in chunks)
    return b'RIFF'+struct.pack('<I',len(body))+body

def asset(name,data):
    p=root/(name+'.mix');p.write_bytes(struct.pack('<HI',1,len(data))+struct.pack('<3I',1,0,len(data))+data)
    return str(p)+':0x1'

def pcm(ch,rate,bits):
    block=ch*bits//8
    body=(bytes([0,128,255]) if bits==8 else struct.pack('<3h',-32768,0,32767))*ch
    return riff([(b'fmt ',struct.pack('<HHIIHH',1,ch,rate,rate*block,block,bits)),(b'data',body)])

for ch,rate in [(1,2**31),(1,2**32-1),(2,2**30),(2,2**31-1)]:
    a=asset(f'high-{ch}-{rate}',pcm(ch,rate,8));out=root/'protected.wav'
    out.write_bytes(b'preserve earlier export')
    run('baseline corrupt export',baseline,Path('/workspace/openyr'),['wav',a,out],0)
    data=out.read_bytes();out_rate,byte_rate=struct.unpack('<II',data[24:32]);assert byte_rate!=(out_rate*ch*2)
    print('baseline corrupt header:',{'rate':out_rate,'byte_rate':byte_rate,'expected':out_rate*ch*2},flush=True)
    for exists in [True,False]:
        if exists:out.write_bytes(b'preserve earlier export')
        else:out.unlink()
        text=run('final rejects before opening output',final,repo,['wav',a,out],1)
        assert 'WAV byte rate exceeds' in text and 'panicked' not in text
        assert out.read_bytes()==b'preserve earlier export' if exists else not out.exists()

controls=[]
for ch in [1,2]:
    for bits in [8,16]:controls.append((f'pcm-{ch}-{bits}',asset(f'pcm-{ch}-{bits}',pcm(ch,22050,bits))))
    controls.append((f'edge-{ch}',asset(f'edge-{ch}',pcm(ch,(2**32-1)//(ch*2),8))))
    fmt=struct.pack('<HHIIHH',17,ch,22050,19600,8*ch,4)
    body=b''.join(struct.pack('<hBB',p,0,0) for p in [100,-100][:ch])+bytes(ch*4)
    controls.append((f'ima-{ch}',asset(f'ima-{ch}',riff([(b'fmt ',fmt),(b'fact',struct.pack('<I',3)),(b'data',body)]))))
controls.extend([('retail-wav','/workspace/game-data/RA2/langmd.mix:AUDIOMD.MIX/CEVA060.WAV'),('retail-aud','/workspace/game-data/RA2/ra2.mix:LOCAL.MIX/INTRO.AUD'),('retail-bank','/workspace/game-data/RA2/language.mix:AUDIO.MIX/AUDIO.BAG#ichratta')])
for name,a in controls:
    old=root/(name+'-before.wav');new=root/(name+'-after.wav')
    run(name+' baseline',baseline,Path('/workspace/openyr'),['wav',a,old],0)
    run(name+' final',final,repo,['wav',a,new],0)
    assert old.read_bytes()==new.read_bytes()
    with wave.open(str(new),'rb') as wav:
        assert wav.getsampwidth()==2 and wav.getnchannels() in [1,2]
        print(name,{'frames':wav.getnframes(),'channels':wav.getnchannels(),'rate':wav.getframerate(),'sha256':hashlib.sha256(new.read_bytes()).hexdigest()},flush=True)
    if name.startswith('ima-'):
        assert new.read_bytes()[44:]==struct.pack('<'+str(3*int(name[-1]))+'h',*([100] if name.endswith('1') else [100,-100])*3)
print('PASS: four baseline corrupt headers; eight clean rejections preserve existing/absent destinations; eleven valid PCM/IMA/retail controls byte-identical.',flush=True)
(root/'results.json').write_text(json.dumps(records,indent=2)+'\n')
