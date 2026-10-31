#!/usr/bin/env python3
"""Odtwarza src/fonts_data.rs z plików web/fonts/<rodzina>/*.woff2.
Pliki pochodzą z pakietu fontów BTB-CG (fonts/pack, @fontsource, OFL-1.1). Uruchom z katalogu source/:
    python3 tools/gen_fonts.py
(Skrypt jednorazowo wygenerował też katalog web/fonts z pakietu BTB-CG; tu odtwarza tylko tabelę Rust.)"""
import os, re
FAMS = {'oswald': 'Oswald', 'barlow-condensed': 'Barlow Condensed', 'inter': 'Inter', 'roboto-condensed': 'Roboto Condensed', 'montserrat': 'Montserrat', 'roboto': 'Roboto',
        'barlow': 'Barlow', 'bebas-neue': 'Bebas Neue', 'roboto-mono': 'Roboto Mono'}
old = open('src/fonts_data.rs').read()
ranges = re.findall(r'pub const RANGE_[A-Z_]+: &str = "[^"]*";', old)
rows = []
for slug, name in FAMS.items():
    for f in sorted(os.listdir(f'web/fonts/{slug}')):
        m = re.match(rf'{slug}-(latin-ext|latin)-(\d+)-(normal|italic)\.woff2$', f)
        if m:
            rows.append((name, slug, f, m.group(1), int(m.group(2)), m.group(3)))
out = ['// Generowane przez tools/gen_fonts.py – nie edytuj ręcznie.', '// Fonty OFL-1.1 (@fontsource), pakiet z BTB-CG; licencje w web/fonts/*/LICENSE.txt.', '',
       'pub struct Face {', "    pub family: &'static str,", "    pub file: &'static str,", "    pub subset: &'static str,", '    pub weight: u16,', "    pub style: &'static str,", "    pub data: &'static [u8],", '}', '', *ranges, '', 'pub const FACES: &[Face] = &[']
for name, slug, f, subset, w, style in rows:
    out.append(f'    Face {{ family: "{name}", file: "{f}", subset: "{subset}", weight: {w}, style: "{style}", data: include_bytes!("../web/fonts/{slug}/{f}") }},')
out.append('];')
open('src/fonts_data.rs', 'w').write('\n'.join(out) + '\n')
