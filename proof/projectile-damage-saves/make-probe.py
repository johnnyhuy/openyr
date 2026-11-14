import sys,pathlib,subprocess,tarfile,io,json
root=pathlib.Path.cwd();commit=sys.argv[1];dest=root/'.tmp'/sys.argv[2]
dest.mkdir()
b=subprocess.check_output(['git','archive',commit])
with tarfile.open(fileobj=io.BytesIO(b)) as t:t.extractall(dest,filter='data')
source={'commit':subprocess.check_output(['git','rev-parse',commit],text=True).strip(),'tree':subprocess.check_output(['git','rev-parse',commit+'^{tree}'],text=True).strip()}
probe=dest/'tools/projectile-damage-probe/src';probe.mkdir(parents=True)
(probe/'main.rs').write_bytes((root/'.tmp/projectile-damage-probe.rs').read_bytes())
(probe.parent/'Cargo.toml').write_text('[package]\nname="projectile-damage-probe"\nversion="0.0.0"\nedition="2021"\n[dependencies]\nyr-sim={path="../../crates/yr-sim"}\nyr-rules={path="../../crates/yr-rules"}\nyr-formats={path="../../crates/yr-formats"}\n')
p=dest/'Cargo.toml';p.write_text(p.read_text().replace('\nmembers = [','\nmembers = ["tools/projectile-damage-probe",',1))
p=dest/'Cargo.lock';p.write_text(p.read_text()+'\n[[package]]\nname = "projectile-damage-probe"\nversion = "0.0.0"\ndependencies = ["yr-formats", "yr-rules", "yr-sim"]\n')
(dest/'PROBE_SOURCE.json').write_text(json.dumps(source,indent=2)+'\n')
subprocess.run(['git','init','-q',str(dest)],check=True)
subprocess.run(['git','-C',str(dest),'add','.'],check=True)
subprocess.run(['git','-C',str(dest),'-c','user.name=Johnny Huynh','-c','user.email=27847622+johnnyhuy@users.noreply.github.com','commit','-qm','Add authored public API range probe to exact source snapshot'],check=True)
print(dest,source)
