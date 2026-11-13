import hashlib,json,subprocess,wave
from pathlib import Path
repo=Path.cwd();base=Path('/workspace/openyr');out=repo/'.tmp/runtime-proof';out.mkdir(exist_ok=False)
receipts={'before':Path('/workspace/openyr-core-review-20261113-6/.tmp/tmp-rebased-runtime/receipt.json'),'after':repo/'.tmp/final-runtime/receipt.json'}
records=[]
def run(label,receipt,worktree,args):
 cmd=['python3','scripts/proof_build.py','--repo',str(worktree),'run','--receipt',str(receipt),'--','/workspace/game-data/RA2','EB1.MMX']+list(map(str,args));print('$ '+' '.join(cmd),flush=True)
 p=subprocess.run(cmd,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=90)
 print(p.stdout,end='',flush=True);assert p.returncode==0,(p.returncode,p.stdout)
 records.append({'label':label,'command':cmd,'exit':p.returncode,'output':p.stdout})
for phase in ['before','after']:
 run(phase+' mixer capture',receipts[phase],base if phase=='before' else repo,['--frames',out/phase,300])
for name in ['audio.wav','audio-cues.tsv','frame-00300.png']:
 a=out/'before'/name;b=out/'after'/name
 assert a.read_bytes()==b.read_bytes(),name
 print('byte-identical',name,len(b.read_bytes()),hashlib.sha256(b.read_bytes()).hexdigest(),flush=True)
with wave.open(str(out/'after/audio.wav'),'rb') as wav:
 assert (wav.getnchannels(),wav.getsampwidth(),wav.getframerate(),wav.getnframes())==(2,2,44100,299*2940)
 print('Verified 299-tick 44100 Hz stereo 16-bit capture',flush=True)
run('final tick 6000 save',receipts['after'],repo,['--ticks',6000,'--save',out/'end.oyrsave'])
run('final restore',receipts['after'],repo,['--load',out/'end.oyrsave','--ticks',6000,'--save',out/'reloaded.oyrsave'])
a=(out/'end.oyrsave').read_bytes();assert a==(out/'reloaded.oyrsave').read_bytes()==Path('/workspace/openyr-core-review-20261113-6/.tmp/tmp-rebased-end.oyrsave').read_bytes()
print('PASS: native mixer/frame/cue bytes unchanged; tick-6000 fresh/restored saves match baseline SHA256',hashlib.sha256(a).hexdigest(),flush=True)
(out/'results.json').write_text(json.dumps(records,indent=2)+'\n')
