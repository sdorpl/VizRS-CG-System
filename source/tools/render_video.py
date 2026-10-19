# Render pokazówki VizRS klatka po klatce (25 kl./s, 1080p).
# Wymaga: pip install playwright && playwright install chromium, oraz ffmpeg.
# Użycie: uruchom vizrs, potem:  python3 render_video.py nazwa steps_info24.py
# (tło: plik studio.jpg w katalogu assets serwera; bez niego tło będzie czarne)
import asyncio, urllib.request, sys, os, subprocess, shutil
from playwright.async_api import async_playwright
B='http://localhost:8088'
def api(p): urllib.request.urlopen(urllib.request.Request(B+p, method='POST')).read()
FONT="""@font-face{font-family:'Barlow Condensed';src:local('DejaVu Sans Condensed Bold');font-weight:600 900}
@font-face{font-family:'Barlow Condensed';src:local('DejaVu Sans Condensed Bold Oblique');font-weight:600 900;font-style:italic}
@font-face{font-family:'Barlow';src:local('DejaVu Sans Bold');font-weight:600 900}
#cap{position:fixed;left:24px;top:22px;z-index:99999;font:600 22px/1.2 'DejaVu Sans',sans-serif;color:#fff;background:rgba(0,0,0,.55);padding:8px 14px;border-radius:6px;border-left:4px solid #f0a020}
#cap small{display:block;font-weight:400;font-size:15px;opacity:.75}"""
CLOCK="""(() => {
  let t = 0; const cbs = new Map(); let id = 0;
  const RD = Date, start = RD.now();
  performance.now = () => t;
  class FD extends RD { constructor(...a) { if (a.length) super(...a); else super(start + t); } static now() { return start + t; } }
  window.Date = FD;
  window.requestAnimationFrame = (cb) => { cbs.set(++id, cb); return id; };
  window.cancelAnimationFrame = (i) => cbs.delete(i);
  window.__step = (ms) => { t += ms; const l = [...cbs.values()]; cbs.clear(); for (const cb of l) cb(t); };
})();"""
name, script_file = sys.argv[1], sys.argv[2]
steps = eval(open(script_file).read())
FPS=25; out=f'frames_{name}'
shutil.rmtree(out, ignore_errors=True); os.makedirs(out)
async def main():
    async with async_playwright() as p:
        br = await p.chromium.launch()
        pg = await br.new_page(viewport={'width':1920,'height':1080})
        await pg.add_init_script(CLOCK)
        await pg.goto(B+'/output?bg='+urllib.parse.quote('#000 url(/assets/studio.jpg) center/cover'))
        await pg.add_style_tag(content=FONT)
        await pg.evaluate("document.body.insertAdjacentHTML('beforeend','<div id=cap></div>')")
        await pg.wait_for_timeout(800)
        api('/api/clear'); await pg.wait_for_timeout(200)
        n=0
        for st in steps:
            cmds, secs, cap = st[:3]; ms = st[3] if len(st) > 3 else 40
            for c in cmds: api(c)
            if cmds: await pg.wait_for_timeout(180)
            await pg.evaluate("c=>{document.getElementById('cap').innerHTML=c; document.getElementById('cap').style.display=c?'block':'none'}", cap)
            for _ in range(int(secs*FPS)):
                await pg.evaluate(f"__step({ms})")
                await pg.screenshot(path=f'{out}/{n:05d}.jpg', type='jpeg', quality=92)
                n+=1
        await br.close()
    subprocess.run(['ffmpeg','-y','-loglevel','error','-framerate',str(FPS),'-i',f'{out}/%05d.jpg','-c:v','libx264','-pix_fmt','yuv420p','-crf','18','-preset','medium','-movflags','+faststart',f'{name}.mp4'],check=True)
    print(name, n, 'frames')
import urllib.parse
asyncio.run(main())
