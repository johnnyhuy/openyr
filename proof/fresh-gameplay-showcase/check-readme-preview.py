import hashlib,json,urllib.request,subprocess
from pathlib import Path
from html.parser import HTMLParser
from playwright.sync_api import sync_playwright
root=Path('.tmp');proof=root/'proof';body=(root/'rendered-body.html').read_text()
class Images(HTMLParser):
 def __init__(self):super().__init__();self.urls=[]
 def handle_starttag(self,tag,attrs):
  if tag=='img':self.urls.append(dict(attrs)['src'])
images=Images();images.feed(body);assets={};metadata=[]
public=json.loads((proof/'source-media.json').read_text())['public_media'];known={x['url']:x for x in public}
for url in images.urls:
 if url in known:
  m=known[url];data=(root/'media'/m['name']).read_bytes();mime=m['mime']
 else:
  with urllib.request.urlopen(url,timeout=60) as response:
   assert response.status==200
   mime=response.headers.get('Content-Type','').split(';',1)[0];data=response.read()
 assets[url]=(mime,data);metadata.append({'url':url,'mime':mime,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
css_url='https://cdnjs.cloudflare.com/ajax/libs/github-markdown-css/5.8.1/github-markdown-light.min.css'
with urllib.request.urlopen(css_url,timeout=45) as r:assert r.status==200;css=r.read().decode()
(root/'github-markdown-light.css').write_text(css)
html='<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>OpenYR README preview</title><style>'+css+'body{margin:0;background:white}.markdown-body{box-sizing:border-box;min-width:200px;max-width:1012px;margin:0 auto;padding:32px}@media(max-width:767px){.markdown-body{padding:16px}}</style><article class="markdown-body">'+body+'</article>'
(root/'readme-preview.html').write_text(html);results=[]
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path='/usr/bin/chromium',headless=True,args=['--no-sandbox','--disable-dev-shm-usage'])
 for label,width,height in [('desktop',1440,1100),('mobile',390,844)]:
  page=browser.new_page(viewport={'width':width,'height':height},device_scale_factor=1)
  def serve(route):
   item=assets.get(route.request.url)
   if item:route.fulfill(status=200,content_type=item[0],body=item[1],headers={'X-Content-Type-Options':'nosniff'})
   else:route.abort()
  page.route('**/*',serve);page.set_content(html,wait_until='load')
  page.wait_for_function('() => [...document.images].every(i=>i.complete && i.naturalWidth>0)',timeout=15000)
  data=page.evaluate('''() => ({images:[...document.images].map(i=>({src:i.src,alt:i.alt,naturalWidth:i.naturalWidth,naturalHeight:i.naturalHeight,width:i.getBoundingClientRect().width})),videoLinks:[...document.querySelectorAll('a')].filter(a=>a.href.endsWith('full-session.mp4')).map(a=>({text:a.textContent,href:a.href})),tables:[...document.querySelectorAll('table')].map(t=>({width:t.clientWidth,scrollWidth:t.scrollWidth})),documentWidth:document.documentElement.scrollWidth,viewport:innerWidth})''')
  assert len(data['videoLinks'])==1 and len(data['images'])==7
  assert len(data['tables'])>=4 and data['documentWidth']<=width
  assert all(i['width']>=150 for i in data['images'] if 'power plant' in i['alt'] or 'assaulting' in i['alt'])
  assert sum(i['naturalWidth']==640 and ('power plant' in i['alt'] or 'assaulting' in i['alt']) for i in data['images'])==2
  page.screenshot(path=str(proof/f'readme-{label}-full.png'),full_page=True)
  page.screenshot(path=str(proof/f'readme-{label}.png'))
  if label=='mobile':
   page.locator('table').first.evaluate('(t)=>t.scrollLeft=t.scrollWidth');page.screenshot(path=str(proof/'readme-mobile-table-scroll.png'))
  results.append({'viewport':label,'rendering':'GitHub Markdown API HTML with github-markdown-css, local browser preview','tls_verification':'enabled for all HTTP source retrieval; browser network fulfilled only from verified local bytes',**data});page.close()
 browser.close()
(proof/'browser.json').write_text(json.dumps({'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'source_tree':subprocess.check_output(['git','rev-parse','HEAD^{tree}'],text=True).strip(),'views':results,'public_image_responses':metadata,'stylesheet':{'url':css_url,'sha256':hashlib.sha256(css.encode()).hexdigest()},'direct_browser_limit':'Direct GitHub navigation failed ERR_CERT_AUTHORITY_INVALID for the cloud proxy CA; no direct page capture is claimed.'},indent=2)+'\n')
print('Desktop and mobile previews decode all seven images, including both gameplay GIFs; one full-session link; no document overflow.')
