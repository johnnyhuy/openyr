import hashlib,json,pathlib,shlex,subprocess,sys
root=pathlib.Path.cwd(); fixtures=root/'.tmp/cli-fixtures';fixtures.mkdir()
for folder,names in [('forward',['RULESMD.INI','rulesmd.ini']),('reverse',['rulesmd.ini','RULESMD.INI'])]:
 d=fixtures/folder;d.mkdir()
 for i,name in enumerate(names):
  with (d/name).open('xb') as f:f.write(f'[General]\nvalue={i}\n'.encode())
u=fixtures/'unique';u.mkdir();(u/'rUlEsMd.InI').write_bytes(b'[General]\nvalue=unique\n');(u/'RULESMD.INI').write_bytes(b'');(u/'ignored.txt').write_bytes(b'');(u/'ignored.shp').mkdir()
def hashes():return {str(p.relative_to(fixtures)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(fixtures.rglob('*')) if p.is_file()}
before=hashes(); records=[];errors=[]
for folder in ['forward','reverse','unique']:
 for repeat in range(3):
  command=[sys.executable,'scripts/proof_build.py','run','--receipt','.tmp/loose-open-runtime/receipt.json','--','vfs',str(fixtures/folder),'RuLeSmD.iNi','ignored.txt','ignored.shp']
  print('$ '+shlex.join(command),flush=True)
  result=subprocess.run(command,capture_output=True,text=True)
  print(result.stdout,end='');print(result.stderr,end='');print(f'exit={result.returncode}',flush=True)
  expected=0 if folder=='unique' else 1
  assert result.returncode==expected,(folder,result.returncode)
  if folder=='unique':assert '<loose>' in result.stdout and result.stdout.count('not found')==2
  else:
   assert 'ambiguous loose filename "RULESMD.INI"' in result.stderr
   errors.append(result.stderr[result.stderr.index('ambiguous loose filename'):].strip())
  records.append({'folder':folder,'repeat':repeat,'exit':result.returncode,'stdout':result.stdout,'stderr':result.stderr})
after=hashes();assert before==after;assert len(set(errors))==1
out={'source_tree':(root/'.tmp/source-tree').read_text().strip(),'calls':records,'consistent_collision_diagnostic':errors[0],'source_files_before':before,'source_files_after':after,'all_file_bytes_preserved':True,'case_sensitive_filesystem_exercised':True,'retail_data':False}
(root/'.tmp/proof/cli-results.json').write_text(json.dumps(out,indent=2)+'\n')
print('Nine verified CLI calls passed: both creation orders reject consistently, unique/empty/directory lookup retains behavior, and every source byte/name is preserved.',flush=True)
