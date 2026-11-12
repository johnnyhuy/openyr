import hashlib,json,subprocess,urllib.request
from pathlib import Path
from PIL import Image
root=Path('.tmp');media=root/'media';media.mkdir(exist_ok=True);proof=root/'proof';proof.mkdir(exist_ok=True)
sha='de23c15a4b438357fb6db4a1291efaf8d74bf102';prefix='proof/retail-regressions/'
remote=json.loads(subprocess.check_output(['gh','api',f'repos/johnnyhuy/openyr/git/trees/{sha}?recursive=1']));assert not remote.get('truncated')
entries={e['path'][len(prefix):]:e for e in remote['tree'] if e['type']=='blob' and e['path'].startswith(prefix)}
manifest=json.loads(subprocess.check_output(['git','show',sha+':'+prefix+'manifest.json']));assert len(entries)==51 and len(manifest)==50
rows=[]
for name,e in entries.items():
 data=subprocess.check_output(['git','show',sha+':'+prefix+name]);digest=hashlib.sha256(data).hexdigest()
 assert len(data)==e['size'],name
 assert hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()==e['sha'],name
 if name in manifest:assert len(data)==manifest[name]['bytes'] and digest==manifest[name]['sha256'],name
 rows.append({'name':name,'bytes':len(data),'git_blob':e['sha'],'sha256':digest})
print('Verified all 51 remote proof blobs and all 50 manifest payloads.',flush=True)
public=[]
for name in ['placement.gif','assault.gif','full-session.mp4']:
 url='https://raw.githubusercontent.com/johnnyhuy/openyr/'+sha+'/'+prefix+name
 with urllib.request.urlopen(url,timeout=90) as response:
  data=response.read();mime=response.headers.get('Content-Type','').split(';',1)[0].strip().lower();status=response.status
 digest=hashlib.sha256(data).hexdigest();expected=manifest[name]
 assert status==200 and len(data)==expected['bytes'] and digest==expected['sha256'],name
 path=media/name;path.write_bytes(data);item={'name':name,'url':url,'status':status,'mime':mime,'bytes':len(data),'sha256':digest}
 if name.endswith('.gif'):
  print('GIF response MIME:',mime,flush=True)
  assert mime in {'image/gif','application/octet-stream'},mime
  assert data[:6] in {b'GIF89a',b'GIF87a'}
  with Image.open(path) as im:
   item.update(width=im.width,height=im.height,frames=im.n_frames,loop=im.info.get('loop'))
   hashes=set();duration=0
   for frame in range(im.n_frames):
    im.seek(frame);hashes.add(hashlib.sha256(im.convert('RGBA').tobytes()).hexdigest());duration+=im.info.get('duration',0)
   item.update(distinct_rgba_frames=len(hashes),duration_ms=duration)
   assert im.size==(640,400) and im.n_frames==({'placement.gif':120,'assault.gif':180}[name]) and len(hashes)>1
 else:
  assert mime in {'video/mp4','application/octet-stream'},mime
  meta=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-show_format','-of','json',str(path)]));item['ffprobe']=meta
  video=next(s for s in meta['streams'] if s['codec_type']=='video');audio=next(s for s in meta['streams'] if s['codec_type']=='audio')
  assert video['width']==1280 and video['height']==800 and video['nb_frames']=='2999' and video['r_frame_rate']=='15/1'
  assert audio['channels']==2 and audio['sample_rate']=='44100' and abs(float(video['duration'])-float(audio['duration']))<0.01
 public.append(item);print(name,status,mime,len(data),flush=True)
(proof/'source-media.json').write_text(json.dumps({'source_proof_commit':sha,'remote_blobs':rows,'manifest_payloads_verified':len(manifest),'public_media':public},indent=2)+'\n')
for name in ['build-receipt.json','final-build-receipt.json','unchanged-runtime-frames.json']:
 (proof/name).write_bytes(subprocess.check_output(['git','show',sha+':'+prefix+name]))
print('Verified GIF frames, MP4 video/audio metadata and public content hashes.',flush=True)
