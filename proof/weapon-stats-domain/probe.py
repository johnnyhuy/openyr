import pathlib,subprocess,json,hashlib,tarfile,io
root=pathlib.Path.cwd();out=root/'.tmp/proof-payload';out.mkdir(exist_ok=True)
cases=[('4294967296','65','100%',False),('4294967297','65','100%',False),('9223372036854775807','65','100%',False),('2','2147483648','100%',False),('2','-2147483648','100%',False),('2','65','100000.0001%',False),('2','65','-100000.0001%',False),('absent','65','100%',True),('-1','65','100%',True),('0','65','100%',True),('2','65','90%',True),('4294967295','65','100%',True),('2','2147483647','100%',True),('2','-2147483647','100%',True),('2','1','100000%',True),('2','1','-100000%',True)]
audit={};results=[]
for phase in ['baseline','final']:
 repo=root/f'.tmp/{phase}-probe-src';source=json.loads((repo/'PROBE_SOURCE.json').read_text());files={};allowed={}
 assert (repo/'tools/weapon-stats-probe/src/main.rs').read_bytes()==(root/'.tmp/weapon-stats-probe.rs').read_bytes()
 with tarfile.open(fileobj=io.BytesIO(subprocess.check_output(['git','archive',source['commit']]))) as tar:
  for f in tar:
   if f.isfile() and f.name not in ['Cargo.toml','Cargo.lock']:
    b=tar.extractfile(f).read();assert (repo/f.name).read_bytes()==b;files[f.name]=hashlib.sha256(b).hexdigest()
 for name in ['Cargo.toml','Cargo.lock']:
  original=subprocess.check_output(['git','show',source['commit']+':'+name])
  if name=='Cargo.toml': expected=original.replace(b'\nmembers = [',b'\nmembers = ["tools/weapon-stats-probe",',1)
  else: expected=original+b'\n[[package]]\nname = "weapon-stats-probe"\nversion = "0.0.0"\ndependencies = ["yr-formats", "yr-rules", "yr-sim"]\n'
  assert (repo/name).read_bytes()==expected;allowed[name]=hashlib.sha256(expected).hexdigest()
 audit[phase]={'allowed_probe_manifest_changes':allowed,'original':source,'probe_commit':subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip(),'exact_original_files':files}
 print(phase,'exact source files audited:',len(files),flush=True)
for n,(rof,damage,verse,valid) in enumerate(cases):
 row={'rof':rof,'damage':damage,'verse':verse,'valid':valid}
 for phase in ['baseline','final']:
  repo=root/f'.tmp/{phase}-probe-src';receipt=root/f'.tmp/{phase}-probe-built/receipt.json';save=out/f'{phase}-{n}.saves'
  cmd=['python3',str(root/'scripts/proof_build.py'),'--repo',str(repo),'run','--receipt',str(receipt),'--',rof,damage,verse,str(save)]
  r=subprocess.run(cmd,text=True,capture_output=True,timeout=20)
  assert r.returncode==0 and not r.stderr,(phase,rof,damage,verse,r.stderr)
  row[phase]={'command':cmd,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
  print(phase,r.stdout.split('\n',1)[1].strip().replace('\n','; '),flush=True)
  if phase=='final': assert ('weapon=some' if valid else 'weapon=none') in r.stdout
  if phase=='baseline': assert 'weapon=some' in r.stdout
 if valid:
  a=(out/f'baseline-{n}.saves').read_bytes();b=(out/f'final-{n}.saves').read_bytes();assert a==b
  assert row['baseline']['stdout'].split('\n',1)[1]==row['final']['stdout'].split('\n',1)[1]
  row['save_bytes']=len(a);row['save_sha256']=hashlib.sha256(a).hexdigest()
 results.append(row)
(out/'source-audit.json').write_text(json.dumps(audit,indent=2)+'\n');(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
print('PASS: three overflowing reloads and four unsupported stat fields rejected; nine supported/default/boundary controls preserve complete saved bytes and continued state through sixteen ticks and restore.',flush=True)
