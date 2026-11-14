import json, subprocess, pathlib
root=pathlib.Path.cwd()
out=root/'.tmp/proof-payload'
out.mkdir(exist_ok=True)
cases=[('9223372036854.999999','(default)'),('-9223372036854.999999','(default)'),('9223372036854.999999%','(default)'),('-9223372036854.999999%','(default)'),('9223372036854.775807','9223372036854.775807 (2361183241434822 lp)'),('-9223372036854.775808','-9223372036854.775808 (-2361183241434822 lp)'),('5.75','5.75 (1472 lp)'),('-1.4','-1.4 (-358 lp)'),('75.125%','0.75125 (192 lp)')]
results=[]
for n,(value,expected) in enumerate(cases):
    fixture=out/f'case-{n}'
    fixture.mkdir(exist_ok=True)
    (fixture/'rulesmd.ini').write_text(f'[VehicleTypes]\n0=TEST\n[TEST]\nPrimary=WEAPON\n[WEAPON]\nRange={value}\n')
    (fixture/'artmd.ini').write_text('[TEST]\nVoxel=yes\n')
    row={'value':value,'expected':expected}
    for phase,repo in [('baseline',pathlib.Path('/workspace/openyr-decimal-baseline-20261114')),('final',root)]:
        cmd=['python3',str(root/'scripts/proof_build.py'),'--repo',str(repo),'run','--receipt',str(root/f'.tmp/{phase}-cli/receipt.json'),'--','rules',str(fixture),'TEST']
        r=subprocess.run(cmd,text=True,capture_output=True,timeout=20)
        row[phase]={'command':cmd,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
        line=next((s for s in r.stdout.splitlines() if 'WEAPON: damage' in s),'NO WEAPON OUTPUT')
        print(phase,repr(value),'exit',r.returncode,line,flush=True)
        assert r.returncode==0 and not r.stderr,(phase,value,r.stderr)
        if phase=='final': assert f'range {expected} burst' in r.stdout,(value,r.stdout)
    if n<4: assert 'range (default) burst' not in row['baseline']['stdout']
    if n>=6: assert row['baseline']['stdout'].split('\n',1)[1]==row['final']['stdout'].split('\n',1)[1]
    results.append(row)
(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
print('PASS: four release overflow regressions, two exact signed endpoints, three byte-identical ordinary controls.',flush=True)
