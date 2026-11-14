import json,subprocess,pathlib,hashlib,struct
root=pathlib.Path.cwd();out=root/'.tmp/proof-payload';out.mkdir(exist_ok=True)
base=pathlib.Path('/workspace/openyr-core-slice-20261114-04')
cases=[(9223372036854775807,'100%',9223372036854775807),(-9223372036854775808,'100%',-9223372036854775808),(-9223372036854775808,'-100%',9223372036854775807),(9223372036854775807,'200%',9223372036854775807),(-9223372036854775808,'200%',-9223372036854775808),(65,'90%',58),(-65,'90%',-58)]
results=[]
for n,(damage,verse,expected) in enumerate(cases):
 d=out/f'case-{n}';d.mkdir(exist_ok=True)
 (d/'rulesmd.ini').write_text(f'[VehicleTypes]\n0=TEST\n[TEST]\nPrimary=GUN\n[GUN]\nDamage={damage}\nWarhead=WH\n[WH]\nVerses='+','.join([verse]*11)+'\n')
 (d/'artmd.ini').write_text('[TEST]\nVoxel=yes\n')
 row={'damage':damage,'verse':verse,'expected':expected}
 for phase,repo,receipt in [('baseline',base,base/'.tmp/final-cli/receipt.json'),('final',root,root/'.tmp/final-cli/receipt.json')]:
  cmd=['python3',str(root/'scripts/proof_build.py'),'--repo',str(repo),'run','--receipt',str(receipt),'--','rules',str(d),'TEST']
  r=subprocess.run(cmd,text=True,capture_output=True,timeout=20)
  assert r.returncode==0 and not r.stderr,(phase,r.returncode,r.stderr)
  row[phase]={'command':cmd,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
  line=next(x for x in r.stdout.splitlines() if 'damage/shot:' in x)
  print(phase,damage,verse,line.strip(),flush=True)
  if phase=='final': assert f'damage/shot: none {expected}, flak {expected}' in line
 if n<5: assert row['baseline']['stdout'].split('\n',1)[1]!=row['final']['stdout'].split('\n',1)[1]
 else: assert row['baseline']['stdout'].split('\n',1)[1]==row['final']['stdout'].split('\n',1)[1]
 results.append(row)
a=(out/'baseline-saves.bin').read_bytes();b=(out/'final-saves.bin').read_bytes();assert a==b
print('PASS: five corrected extreme damage tables; two unchanged ordinary controls.',flush=True)
print('PASS: 64 baseline/final save snapshots identical:',len(a),'bytes SHA256',hashlib.sha256(a).hexdigest(),flush=True)
(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
