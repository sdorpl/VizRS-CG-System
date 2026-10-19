"""Generuje animacje OUT dla scen VizRS: teksty wyjeżdżają i gasną, tła zmazują się maską
w kierunku zgodnym z wejściem, kolejno (stagger). Sceny, w których cała grupa odjeżdża (x/y), zostają bez zmian.
Użycie: python3 out_choreo.py plik.show.json [...]"""
import json, sys

TEXTY = {'text', 'clock', 'qr', 'ticker'}
OPP = {'clipR': 'clipL', 'clipL': 'clipR', 'clipT': 'clipB', 'clipB': 'clipT'}

def walk(nodes):
    for n in nodes:
        yield n
        yield from walk(n.get('children', []))

def val_at_stop(n, prop, stop, default=0):
    tr = (n.get('anim') or {}).get(prop) or []
    before = [k for k in tr if k['t'] <= stop + 1e-6]
    if before: return before[-1]['v']
    return (n.get('props') or {}).get(prop, default)

def process(scene):
    stops = sorted(scene.get('stops') or [])
    if not stops: return 'brak stopów – pominięto'
    stop = stops[-1]
    nodes = list(walk(scene['nodes']))
    # cała grupa odjeżdża ruchem? zostaw
    for n in nodes:
        for p in ('x', 'y'):
            tr = (n.get('anim') or {}).get(p, [])
            after = [k['v'] for k in tr if k['t'] > stop + 1e-6]
            if after and n.get('children') and abs(after[-1] - val_at_stop(n, p, stop)) > 60:
                return 'ruch grupy – bez zmian'
    # usuń dotychczasowe klucze po ostatnim stopie
    for n in nodes:
        an = n.get('anim') or {}
        for p in list(an):
            an[p] = [k for k in an[p] if k['t'] <= stop + 1e-6]
            if not an[p]: del an[p]
    leaves = [n for n in nodes if not n.get('children') or n['type'] in ('rect',) and False]
    leaves = [n for n in nodes if not n.get('children')]
    big = lambda n: (n.get('props') or {}).get('w', 0) * (n.get('props') or {}).get('h', 0) > 40000
    texts = [n for n in leaves if n['type'] in TEXTY or (n['type'] == 'image' and not big(n))]
    boxes = [n for n in leaves if n not in texts]
    t0 = stop + 0.02
    end = t0
    # 1) teksty: przenikanie + odjazd 26 px (kolejno od góry drzewa = od wierzchu)
    for i, n in enumerate(reversed(texts)):
        a, b = t0 + i * 0.04, t0 + i * 0.04 + 0.26
        an = n.setdefault('anim', {})
        an.setdefault('opacity', []).extend([{'t': round(a, 3), 'v': 1, 'e': 'in'}, {'t': round(b, 3), 'v': 0}])
        if n['type'] != 'qr':
            x = val_at_stop(n, 'x', stop)
            an.setdefault('x', []).extend([{'t': round(a, 3), 'v': x, 'e': 'in'}, {'t': round(b, 3), 'v': x + 26}])
        else:
            an.setdefault('scale', []).extend([{'t': round(a, 3), 'v': 1, 'e': 'in'}, {'t': round(b, 3), 'v': 0.85}])
        end = max(end, b)
    # 2) tła: maska zgodna z kierunkiem wejścia, od wierzchu do spodu
    tb = t0 + 0.12 + 0.04 * min(len(texts), 4)
    for i, n in enumerate(reversed(boxes)):
        a, b = tb + i * 0.05, tb + i * 0.05 + 0.34
        an = n.setdefault('anim', {})
        pr = n.get('props') or {}
        w, h = pr.get('w', 400), pr.get('h', 100)
        if w < 30 and h < 30 or n['type'] == 'ellipse':
            an.setdefault('opacity', []).extend([{'t': round(a, 3), 'v': 1}, {'t': round(a + 0.2, 3), 'v': 0}])
            end = max(end, a + 0.2); continue
        used = [p for p in ('clipR', 'clipL', 'clipT', 'clipB') if p in an]
        prop = OPP[used[0]] if used else ('clipL' if w >= h else 'clipB')
        an.setdefault(prop, []).extend([{'t': round(a, 3), 'v': 0, 'e': 'in'}, {'t': round(b, 3), 'v': 100}])
        end = max(end, b)
    for n in nodes:
        for p in (n.get('anim') or {}):
            n['anim'][p].sort(key=lambda k: k['t'])
    scene['duration'] = round(end + 0.04, 3)
    return f'OUT {round(scene["duration"] - stop, 2)} s ({len(texts)} tekstów, {len(boxes)} teł)'

for path in sys.argv[1:]:
    d = json.load(open(path))
    print('==', path)
    for s in d['scenes']:
        print(f"  {s['name']:24} {process(s)}")
    json.dump(d, open(path, 'w'), ensure_ascii=False, indent=1)
