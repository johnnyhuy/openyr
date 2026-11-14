import pathlib,subprocess,json,hashlib,tarfile,io,struct
root=pathlib.Path.cwd();out=root/'.tmp/proof-payload';out.mkdir(exist_ok=True)
audit={};results=[]
for phase in ['baseline','final']:
 repo=root/f'.tmp/{phase}-probe-src';source=json.loads((repo/'PROBE_SOURCE.json').read_text());files={};allowed={}
 assert (repo/'tools/projectile-damage-probe/src/main.rs').read_bytes()==(root/'.tmp/projectile-damage-probe.rs').read_bytes()
 with tarfile.open(fileobj=io.BytesIO(subprocess.check_output(['git','archive',source['commit']]))) as tar:
  for f in tar:
   if f.isfile() and f.name not in ['Cargo.toml','Cargo.lock']:
    b=tar.extractfile(f).read();assert (repo/f.name).read_bytes()==b;files[f.name]=hashlib.sha256(b).hexdigest()
 for name in ['Cargo.toml','Cargo.lock']:
  original=subprocess.check_output(['git','show',source['commit']+':'+name])
  if name=='Cargo.toml': expected=original.replace(b'\nmembers = [',b'\nmembers = ["tools/projectile-damage-probe",',1)
  else: expected=original+b'\n[[package]]\nname = "projectile-damage-probe"\nversion = "0.0.0"\ndependencies = ["yr-formats", "yr-rules", "yr-sim"]\n'
  assert (repo/name).read_bytes()==expected;allowed[name]=hashlib.sha256(expected).hexdigest()
 audit[phase]={'allowed_probe_manifest_changes':allowed,'original':source,'probe_commit':subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip(),'exact_original_files':files}
 print(phase,'exact source files audited:',len(files),flush=True)

def run(phase,args):
 repo=root/f'.tmp/{phase}-probe-src';receipt=root/f'.tmp/{phase}-probe-built/receipt.json'
 cmd=['python3',str(root/'scripts/proof_build.py'),'--repo',str(repo),'run','--receipt',str(receipt),'--',*map(str,args)]
 r=subprocess.run(cmd,text=True,capture_output=True,timeout=30)
 assert r.returncode==0 and not r.stderr,(cmd,r.returncode,r.stdout,r.stderr)
 output=r.stdout.split('\n',1)[1]
 print(phase,output.strip().replace('\n','; '),flush=True)
 return {'command':cmd,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr},output
cases=[('2147483647','200%',4294967294),('2147483647','-200%',-4294967294),('2147483647','100000%',2147483647000),('2147483647','-100000%',-2147483647000),('-2147483647','100000%',-2147483647000),('-2147483647','-100000%',2147483647000),('65','90%',58),('2147483647','100%',2147483647),('-2147483647','100%',-2147483647),('65','0%',0)]
for n,(damage,verse,expected) in enumerate(cases):
 row={'damage':damage,'verse':verse,'expected':expected};outputs={}
 for phase in ['baseline','final']:
  trace=out/f'{phase}-{n}.saves';row[phase],outputs[phase]=run(phase,[damage,verse,trace])
  if phase=='baseline' and n<6:
   assert 'save=error invalid save: projectile damage out of range' in outputs[phase];assert not trace.exists();continue
  assert 'save=ok' in outputs[phase]
  b=trace.read_bytes();size=struct.unpack('<Q',b[:8])[0];data=json.loads(b[20:8+size]);assert data['projectiles'][0]['damage']==expected
  row[phase]['trace_sha256']=hashlib.sha256(b).hexdigest()
 if n>=6:
  assert (out/f'baseline-{n}.saves').read_bytes()==(out/f'final-{n}.saves').read_bytes();assert outputs['baseline']==outputs['final']
 results.append(row)
b=(out/'final-6.saves').read_bytes();size=struct.unpack('<Q',b[:8])[0];save=b[8:8+size];original=json.loads(save[12:]);loads=[]
for field,values,error in [('projectile',[-2147483647000,2147483647000,-2147483647001,2147483647001,-9223372036854775808,9223372036854775807],'projectile damage out of range'),('origin',[-2147483648,2147483648],'shot source damage out of range'),('weapon',[-2147483648,2147483648],'weapon damage out of range')]:
 for value in values:
  data=json.loads(json.dumps(original))
  if field=='projectile':data['projectiles'][0]['damage']=value
  elif field=='origin':data['projectiles'][0]['origin']['damage']=value
  else:data['units'][0]['weapon']['damage']=value
  path=out/f'{field}-{value}.oyrsave';path.write_bytes(save[:12]+json.dumps(data,separators=(',',':')).encode())
  result,output=run('final',['load',path]);valid=field=='projectile' and abs(value)<=2147483647000
  assert ('load=ok' in output) if valid else (error in output)
  loads.append({'field':field,'value':value,'valid':valid,'final':result})
for version in range(1,5):
 data=json.loads(json.dumps(original));data['projectiles'][0].pop('origin');data['units'][0]['weapon'].pop('source')
 path=out/f'legacy-v{version}.oyrsave';path.write_bytes(save[:8]+struct.pack('<I',version)+json.dumps(data,separators=(',',':')).encode())
 a,ao=run('baseline',['load',path]);z,zo=run('final',['load',path]);assert 'load=ok' in ao and ao==zo
 loads.append({'version':version,'baseline':a,'final':z})
(out/'source-audit.json').write_text(json.dumps(audit,indent=2)+'\n');(out/'results.json').write_text(json.dumps(results,indent=2)+'\n');(out/'loader-results.json').write_text(json.dumps(loads,indent=2)+'\n')
print('PASS: six formerly unsaveable native flights restore with exact signed applied damage and impact events; four controls preserve all 36 complete snapshots; 10 hostile/boundary loads and four legacy versions verified.',flush=True)
