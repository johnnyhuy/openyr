import pathlib,subprocess,json,hashlib,tarfile,io
root=pathlib.Path.cwd();out=root/'.tmp/proof-payload';out.mkdir(exist_ok=True)
cases=[('16777216',None),('-16777216',None),('8388608',None),('65536.003907',None),('-0.003907',None),('-0.000001',None),('9223372036854.775807',None),('-9223372036854.775808',None),('0',0),('0.003906',0),('0.003907',1),('1.4',358),('5.75',1472),('65536',16777216),('65536.003906',16777216)]
audit={};results=[]
for phase in ['baseline','final']:
 repo=root/f'.tmp/{phase}-probe-src';source=json.loads((repo/'PROBE_SOURCE.json').read_text())
 files={};allowed={}
 with tarfile.open(fileobj=io.BytesIO(subprocess.check_output(['git','archive',source['commit']]))) as tar:
  for f in tar:
   if f.isfile() and f.name not in ['Cargo.toml','Cargo.lock']:
    b=tar.extractfile(f).read();assert (repo/f.name).read_bytes()==b
    files[f.name]=hashlib.sha256(b).hexdigest()
 for name in ['Cargo.toml','Cargo.lock']:
  original=subprocess.check_output(['git','show',source['commit']+':'+name])
  if name=='Cargo.toml': expected=original.replace(b'\nmembers = [',b'\nmembers = [\"tools/range-probe\",',1)
  else: expected=original+b'\n[[package]]\nname = \"range-probe\"\nversion = \"0.0.0\"\ndependencies = [\"yr-formats\", \"yr-rules\", \"yr-sim\"]\n'
  assert (repo/name).read_bytes()==expected
  allowed[name]=hashlib.sha256(expected).hexdigest()
 audit[phase]={'allowed_probe_manifest_changes':allowed,'original':source,'probe_commit':subprocess.check_output(['git','-C',str(repo),'rev-parse','HEAD'],text=True).strip(),'exact_original_files':files}
 print(phase,'original engine/source files match exact archived commit:',len(files),flush=True)
for n,(text,expected) in enumerate(cases):
 row={'input':text,'expected_range':expected}
 for phase in ['baseline','final']:
  repo=root/f'.tmp/{phase}-probe-src';receipt=root/f'.tmp/{phase}-probe-built/receipt.json';save=out/f'{phase}-{n}.saves'
  cmd=['python3',str(root/'scripts/proof_build.py'),'--repo',str(repo),'run','--receipt',str(receipt),'--',text,str(save)]
  r=subprocess.run(cmd,text=True,capture_output=True,timeout=20)
  assert r.returncode==0 and not r.stderr,(phase,text,r.stderr)
  row[phase]={'command':cmd,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
  print(phase,repr(text),r.stdout.split('\n',1)[1].strip().replace('\n','; '),flush=True)
  if phase=='final': assert f'weapon={expected if expected is not None else "none"}\n' in r.stdout
 if expected is not None:
  a=(out/f'baseline-{n}.saves').read_bytes();b=(out/f'final-{n}.saves').read_bytes();assert a==b
  assert row['baseline']['stdout'].split('\n',1)[1]==row['final']['stdout'].split('\n',1)[1]
  row['save_bytes']=len(a);row['save_sha256']=hashlib.sha256(a).hexdigest()
 results.append(row)
(out/'source-audit.json').write_text(json.dumps(audit,indent=2)+'\n')
(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
print('PASS: eight unsupported ranges rejected; seven ordinary/boundary controls retain complete saved bytes and state hashes across sixteen ticks and restore.',flush=True)
