const fs=require('fs');const {chromium}=require('playwright');
(async()=>{
 const rows=JSON.parse(fs.readFileSync('.tmp/media-urls.json','utf8'));
 const rendered=fs.readFileSync('.tmp/readme-render.html','utf8');
 const html=`<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width"><style>body{margin:0;color:#1f2328;background:white;font:16px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}.markdown-body{overflow-wrap:anywhere;max-width:1012px;margin:24px auto;padding:32px}h1,h2{border-bottom:1px solid #d1d9e0;padding-bottom:.3em}h1{font-size:32px}h2{font-size:24px;margin-top:24px}a{color:#0969da}img{max-width:100%;box-sizing:border-box}table{display:block;width:max-content;max-width:100%;overflow:auto;border-collapse:collapse}td,th{border:1px solid #d1d9e0;padding:6px 13px;text-align:left}tr:nth-child(2n){background:#f6f8fa}td{vertical-align:top}pre{overflow:auto;background:#f6f8fa;padding:16px}code{font-family:monospace} @media(max-width:600px){.markdown-body{margin:0;padding:16px}}</style><article class="markdown-body">${rendered}</article>`;
 fs.writeFileSync('.tmp/readme-preview.html',html);
 const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
 const page=await browser.newPage({viewport:{width:1280,height:1400}});
 let requested=[];
 await page.route('**/*',async route=>{
  const row=rows.find(r=>r.url===route.request().url());
  if(row){requested.push(row.url);return route.fulfill({status:200,contentType:row.format==='GIF'?'image/gif':'image/png',body:fs.readFileSync(row.file)});}
  // The CI badge is unrelated to gameplay rendering.
  return route.fulfill({status:200,contentType:'image/svg+xml',body:'<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20"><rect width="80" height="20" fill="#238636"/><text x="10" y="14" fill="white" font-size="12">CI passing</text></svg>'});
 });
 await page.setContent(html,{waitUntil:'networkidle'});
 await page.waitForFunction(()=>[...document.querySelectorAll('table img')].every(i=>i.complete&&i.naturalWidth>0));
 const data=await page.evaluate(()=>({headings:[...document.querySelectorAll('h2')].map(h=>h.textContent.trim()),images:[...document.querySelectorAll('table img')].map(i=>({src:i.src,alt:i.alt,width:i.naturalWidth,height:i.naturalHeight})),legal:document.body.textContent.includes('not endorsed')&&document.body.textContent.includes('GNU GPLv3'),mp4Links:[...document.querySelectorAll('a')].filter(a=>a.href.endsWith('.mp4')).length}));
 if(data.headings[0]!=='Gameplay'||data.images.length!==7||!data.legal||data.mp4Links!==0)throw Error(JSON.stringify(data));
 await page.screenshot({path:'.tmp/readme-desktop.png'});
 await page.setViewportSize({width:390,height:1000});
 const mobile=await page.evaluate(()=>({bodyOverflow:document.documentElement.scrollWidth>innerWidth,tables:[...document.querySelectorAll('table')].map(t=>({width:t.clientWidth,scrollWidth:t.scrollWidth}))}));
 if(mobile.bodyOverflow)throw Error('Mobile page overflows beyond its scrollable tables');
 await page.locator('table').first().evaluate(t=>t.scrollLeft=t.scrollWidth);
 await page.screenshot({path:'.tmp/readme-mobile.png'});
 fs.writeFileSync('.tmp/browser.json',JSON.stringify({render:'GitHub GFM API HTML, local preview CSS; image requests fulfilled with freshly downloaded and decoded public bytes',requested,desktop:data,mobile},null,2)+'\n');
 console.log('PASS: Gameplay first; seven decoded public images; no MP4 links; legal text retained; mobile table overflow contained.');
 await browser.close();
})().catch(e=>{console.error(e);process.exit(1)});
