//! Tekst: kształtowanie (rustybuzz – kerning i ligatury jak HarfBuzz w Chromium), pomiar szerokości jak
//! `offsetWidth`, łamanie wierszy jak `white-space: pre-wrap` i dopasowanie do ramki – odpowiedniki `fitText`
//! i `fitLines` z `renderer.js` (te same progi, ta sama kolejność prób).

use crate::font::FontDb;
use crate::richtext;

/// Styl tekstu węzła.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub font: String,
    pub weight: u16,
    pub italic: bool,
    pub size: f32,
    pub spacing: f32,
    /// `font-variant-numeric: tabular-nums` (teksty i zegary).
    pub tnum: bool,
    /// Tło i tekst `[[wyróżnienia]]` (`hlColor`, `hlText`).
    pub hl: Option<(Color, Color)>,
}

pub type Color = [f32; 4];

/// Kolor CSS jako RGBA 0–1.
pub fn css_color(v: &str) -> Option<Color> {
    crate::model::parse_color(v).map(|c| [(c[0] / 255.0) as f32, (c[1] / 255.0) as f32, (c[2] / 255.0) as f32, c[3] as f32])
}

/// Glif ułożony w wierszu.
#[derive(Clone, Debug)]
pub struct Glyph {
    pub face: usize,
    pub id: u16,
    /// Położenie początku glifu (px od początku wiersza) i przesunięcie w pionie.
    pub x: f32,
    pub dy: f32,
    /// Rozmiar fontu glifu (`{sz=}` zmienia go w przebiegu).
    pub size: f32,
    /// Kolor własny (`{c=}`, tekst wyróżnienia); brak = kolor tekstu.
    pub color: Option<Color>,
    pub fake_bold: bool,
    pub fake_italic: bool,
}

/// Ozdoby wiersza: tło wyróżnienia (pod tekstem) i podkreślenie (`{u}`).
#[derive(Clone, Debug)]
pub enum Decor {
    /// Prostokąt x0..x1 od `top` do `bottom` względem linii bazowej, zaokrąglenie, kolor.
    Mark { x0: f32, x1: f32, top: f32, bottom: f32, radius: f32, color: Color },
    /// Linia x0..x1 w `y` (względem linii bazowej) o grubości `th`; kolor brak = kolor tekstu.
    Underline { x0: f32, x1: f32, y: f32, th: f32, color: Option<Color> },
}

/// Wiersz po kształtowaniu.
#[derive(Clone, Debug, Default)]
pub struct Run {
    pub glyphs: Vec<Glyph>,
    pub decor: Vec<Decor>,
    /// Szerokość z odstępami liter (jak układ przeglądarki, przed zaokrągleniem).
    pub width: f32,
    pub size: f32,
    /// Wysokość nad i pod linią bazową (zaokrąglone jak w Blink), px.
    pub ascent: f32,
    pub descent: f32,
}

impl Run {
    /// `offsetWidth` elementu z tym tekstem.
    pub fn offset_width(&self) -> f32 {
        self.width.round()
    }
}

/// `font-weight: bolder` względem grubości rodzica.
fn bolder(w: u16) -> u16 {
    match w {
        0..=349 => 400,
        350..=549 => 700,
        _ => 900,
    }
}

/// Kształtuje `text` stylem `style` od położenia `x` i dokłada glify do `run`; zwraca nowe `x`.
fn shape_into(db: &FontDb, style: &Style, text: &str, mut x: f32, run: &mut Run, color: Option<Color>) -> f32 {
    let chain = db.chain(&style.font, style.weight, style.italic);
    let Some(&primary) = chain.first() else { return x };
    // przebiegi o tej samej czcionce (znak bez glifu w pierwszym foncie bierze następny w łańcuchu)
    let mut segs: Vec<(usize, usize, usize)> = Vec::new();
    for (i, c) in text.char_indices() {
        let f =
            if c == ' ' || c == '\u{a0}' { segs.last().map(|s| s.0).unwrap_or(primary) } else { db.face_for(&chain, c).unwrap_or(primary) };
        match segs.last_mut() {
            Some(last) if last.0 == f => last.2 = i + c.len_utf8(),
            _ => segs.push((f, i, i + c.len_utf8())),
        }
    }
    let features: Vec<rustybuzz::Feature> =
        if style.tnum { vec![rustybuzz::Feature::new(rustybuzz::ttf_parser::Tag::from_bytes(b"tnum"), 1, ..)] } else { Vec::new() };
    for (f, a, b) in segs {
        let face = db.face(f);
        let k = style.size / face.upem;
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(&text[a..b]);
        buf.guess_segment_properties();
        let out = rustybuzz::shape(&face.hb, &features, buf);
        let infos = out.glyph_infos();
        let pos = out.glyph_positions();
        let fake_bold = style.weight >= 600 && face.weight < 600;
        let fake_italic = style.italic && !face.italic;
        for (i, (info, p)) in infos.iter().zip(pos).enumerate() {
            run.glyphs.push(Glyph {
                face: f,
                id: info.glyph_id as u16,
                x: x + p.x_offset as f32 * k,
                dy: -(p.y_offset as f32) * k,
                size: style.size,
                color,
                fake_bold,
                fake_italic,
            });
            x += p.x_advance as f32 * k;
            // odstęp liter po każdym znaku (klastrze), także po ostatnim – jak Blink
            let next_cluster = infos.get(i + 1).map(|n| n.cluster);
            if next_cluster != Some(info.cluster) {
                x += style.spacing;
            }
        }
    }
    x
}

/// Wysokość nad i pod linią bazową (px, zaokrąglone jak w Blink) dla stylu.
fn metrics<'a>(db: &'a FontDb, style: &Style) -> Option<(f32, f32, &'a crate::font::Face)> {
    let chain = db.chain(&style.font, style.weight, style.italic);
    let pf = db.face(*chain.first()?);
    Some(((pf.ascent / pf.upem * style.size).round(), (pf.descent / pf.upem * style.size).round(), pf))
}

/// Kształtuje jeden wiersz (bez `\n`); znaczniki tekstu sformatowanego jak `setRich` w `renderer.js`.
pub fn shape(db: &FontDb, style: &Style, text: &str) -> Run {
    let mut run = Run { size: style.size, ..Run::default() };
    let Some((ascent, descent, pf)) = metrics(db, style) else { return run };
    run.ascent = ascent;
    run.descent = descent;
    if text.is_empty() {
        return run;
    }
    if !crate::richtext::has_markup(text) {
        run.width = shape_into(db, style, text, 0.0, &mut run, None).max(0.0);
        return run;
    }
    let (spans, _) = crate::richtext::parse(text, None);
    let em = style.size;
    let (ul_th, ul_pos) = (pf.underline.0 / pf.upem, pf.underline.1 / pf.upem);
    let (hl_bg, hl_fg) = style.hl.unwrap_or(([0.89, 0.02, 0.07, 1.0], [1.0; 4]));
    let mut x = 0.0f32;
    let mut mark_start: Option<f32> = None;
    let close_mark = |run: &mut Run, start: f32, x: &mut f32| {
        // <mark>: padding 0 .16em, margin 0 .04em, zaokrąglenie .06em; tło pod polem treści fontu
        *x += 0.16 * em;
        run.decor.push(Decor::Mark { x0: start + 0.04 * em, x1: *x, top: -ascent, bottom: descent, radius: 0.06 * em, color: hl_bg });
        *x += 0.04 * em;
    };
    for sp in &spans {
        if sp.hl && mark_start.is_none() {
            mark_start = Some(x);
            x += 0.2 * em;
        } else if !sp.hl {
            if let Some(start) = mark_start.take() {
                close_mark(&mut run, start, &mut x);
            }
        }
        let st = Style {
            weight: if sp.b { bolder(style.weight) } else { style.weight },
            italic: style.italic || sp.i,
            size: if sp.sz > 0.0 { style.size * sp.sz / 100.0 } else { style.size },
            ..style.clone()
        };
        let color = css_color(&sp.c).filter(|_| !sp.c.is_empty()).or(sp.hl.then_some(hl_fg));
        let x0 = x;
        x = shape_into(db, &st, &sp.t, x, &mut run, color);
        if sp.u {
            run.decor.push(Decor::Underline { x0, x1: x, y: -ul_pos * st.size, th: (ul_th * st.size).max(1.0), color });
        }
    }
    if let Some(start) = mark_start.take() {
        close_mark(&mut run, start, &mut x);
    }
    run.width = x.max(0.0);
    run
}

/// Szerokość tekstu jak `offsetWidth` elementu z `white-space: pre`.
pub fn measure(db: &FontDb, style: &Style, text: &str) -> f32 {
    shape(db, style, text).offset_width()
}

/// Wiersz gotowy do narysowania: glify, położenie w ramce tekstu i ściśnięcie poziome.
#[derive(Clone, Debug)]
pub struct Line {
    pub run: Run,
    /// Lewa krawędź wiersza i linia bazowa (px w układzie elementu tekstu, przed ściśnięciem).
    pub x: f32,
    pub baseline: f32,
    /// Ściśnięcie poziome `scaleX` i jego oś (x).
    pub sx: f32,
    pub ox: f32,
}

/// Wynik dopasowania (jak `{ w, h, sq, over }` w rendererze) i ułożone wiersze.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub lines: Vec<Line>,
    /// Rozmiar elementu tekstu (`span`) przed ściśnięciem: szerokość i wysokość układu.
    pub span_w: f32,
    pub span_h: f32,
    /// Rzeczywisty rozmiar treści dla autofollow (`mw`, `mh`).
    pub w: f32,
    pub h: f32,
    pub sq: Vec<f32>,
    pub over: bool,
}

/// Parametry dopasowania (jak obiekt `fo` w rendererze).
pub struct Fit<'a> {
    pub avail: f32,
    pub size: f32,
    pub max_lines: f32,
    pub mode: &'a str,
    pub align: &'a str,
    pub line_height: f32,
    pub min_squeeze: f32,
    pub line_break: &'a str,
    pub fill_squeeze: f32,
}

/// `squeezeFloor`: granica ściśnięcia w 0,3…1; brak = 0,6.
pub fn squeeze_floor(v: f32) -> f32 {
    let v = if v.is_nan() || v == 0.0 { 0.6 } else { v };
    v.clamp(0.3, 1.0)
}

/// Miejsca podziału wiersza jak w Chromium (ICU, uproszczone): po spacji, po łączniku między znakami i po półpauzie/pauzie.
fn break_after(prev: char, next: Option<char>) -> bool {
    match prev {
        ' ' | '\t' => next.is_some_and(|n| n != ' '),
        '-' => next.is_some_and(|n| n.is_alphanumeric()),
        '–' | '—' => next.is_some_and(|n| n != ' '),
        _ => false,
    }
}

/// `white-space: pre-wrap`: dzieli tekst na wiersze mieszczące się w `width` (słowo dłuższe wystaje).
/// Spacje na końcu wiersza „wiszą” (nie liczą się do szerokości).
fn wrap(db: &FontDb, style: &Style, text: &str, width: f32) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        // kawałki między miejscami podziału; znaczniki to nie znaki, `[[wyróżnienie]]` się nie dzieli (`nowrap`)
        let mut items: Vec<(String, bool)> = Vec::new(); // (tekst, znacznik)
        let mut i = 0;
        while i < para.len() {
            if let Some(len) = richtext::token_len(&para[i..]) {
                items.push((para[i..i + len].to_string(), true));
                i += len;
            } else {
                let c = para[i..].chars().next().expect("in bounds");
                items.push((c.to_string(), false));
                i += c.len_utf8();
            }
        }
        let mut pieces: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut hl = 0u32;
        for (k, (t, tag)) in items.iter().enumerate() {
            cur.push_str(t);
            if *tag {
                if t == "[[" {
                    hl += 1;
                } else if t == "]]" {
                    hl = hl.saturating_sub(1);
                }
                continue;
            }
            let c = t.chars().next().expect("one char");
            let next = items[k + 1..].iter().find(|(_, tag)| !tag).and_then(|(t, _)| t.chars().next());
            if hl == 0 && break_after(c, next) {
                pieces.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() || pieces.is_empty() {
            pieces.push(cur);
        }
        let mut line = String::new();
        for piece in pieces {
            let candidate = format!("{line}{piece}");
            if !line.is_empty() && shape(db, style, candidate.trim_end_matches(' ')).width > width + 0.01 {
                out.push(std::mem::take(&mut line));
                line = piece;
            } else {
                line = candidate;
            }
        }
        out.push(line);
    }
    out
}

/// Wiersze `pre` albo `pre-wrap` (maxWidth = `width`), każdy już ukształtowany.
fn block_lines(db: &FontDb, style: &Style, text: &str, width: Option<f32>) -> Vec<Run> {
    let mut lines: Vec<String> = match width {
        Some(w) => wrap(db, style, text, w),
        None => text.split('\n').map(str::to_string).collect(),
    };
    if richtext::has_markup(text) {
        lines = richtext::carry_lines(&lines);
    }
    lines.iter().map(|l| shape(db, style, if width.is_some() { l.trim_end_matches(' ') } else { l })).collect()
}

/// Szerokość bloku tekstu (`scrollWidth` przy `pre`): najszerszy wiersz.
fn block_width(runs: &[Run]) -> f32 {
    runs.iter().map(|r| r.width).fold(0.0, f32::max)
}

/// `fitText`: dopasowanie całego tekstu (`squeeze`, `squeeze-shrink`, `shrink`, `none`).
pub fn fit_text(db: &FontDb, base: &Style, text: &str, o: &Fit) -> Layout {
    let l = o.max_lines.round().max(1.0);
    let avail = o.avail.max(1.0);
    let lh = if o.line_height > 0.0 { o.line_height } else { 1.15 };
    let min = squeeze_floor(o.min_squeeze);
    let multi = l > 1.0;
    let style_at = |fs: f32| Style { size: fs, ..base.clone() };
    // jak `fits(fs, width)`: wiersze po złamaniu (multi) albo szerokość (jeden wiersz)
    let fits = |fs: f32, width: f32| -> bool {
        let st = style_at(fs);
        if multi {
            let n = block_lines(db, &st, text, Some(width)).len() as f32;
            let height = (n * fs * lh).round();
            height / (fs * lh) <= l + 0.35
        } else {
            block_width(&block_lines(db, &st, text, None)).round() <= width + 0.5
        }
    };
    let search_font = |width: f32| -> f32 {
        let (mut lo, mut hi) = (4.0f32, o.size);
        for _ in 0..9 {
            let mid = (lo + hi) / 2.0;
            if fits(mid, width) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    };
    let mut sx = 1.0f32;
    let mut fs = o.size;
    let mut over = false;
    let mut wrap_w = if multi { Some(avail) } else { None };
    let squeeze = o.mode == "squeeze" || o.mode == "squeeze-shrink";
    if o.mode == "none" {
        // bez zmian
    } else if fits(o.size, avail) {
        // mieści się
    } else if squeeze {
        let shrink = o.mode == "squeeze-shrink";
        if !multi {
            let sw = block_width(&block_lines(db, &style_at(o.size), text, None)).round().max(1.0);
            let need = avail / sw;
            sx = min.max(need);
            if need < min {
                if shrink {
                    fs = search_font(avail / min);
                } else {
                    over = true;
                }
            }
        } else if fits(o.size, avail / min) {
            let (mut lo, mut hi) = (min, 1.0f32);
            for _ in 0..7 {
                let mid = (lo + hi) / 2.0;
                if fits(o.size, avail / mid) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            sx = lo;
            wrap_w = Some(avail / lo);
        } else {
            sx = min;
            wrap_w = Some(avail / min);
            if shrink {
                fs = search_font(avail / min);
            } else {
                over = true;
            }
        }
    } else {
        fs = search_font(avail);
    }
    let st = style_at(fs);
    let runs = block_lines(db, &st, text, wrap_w);
    let line_h = fs * lh;
    let content_w = block_width(&runs);
    // szerokość elementu: treść, przy wielu wierszach najwyżej maxWidth
    let span_w = match wrap_w {
        Some(w) => content_w.min(w).round().max(if content_w > w { w.round() } else { 0.0 }),
        None => content_w.round(),
    };
    let span_h = (runs.len() as f32 * line_h).round();
    let ox = match o.align {
        "right" => span_w,
        "center" => span_w / 2.0,
        _ => 0.0,
    };
    let lines = runs
        .into_iter()
        .enumerate()
        .map(|(i, run)| {
            let x = match o.align {
                "right" => span_w - run.width,
                "center" => (span_w - run.width) / 2.0,
                _ => 0.0,
            };
            let baseline = i as f32 * line_h + half_leading(line_h, &run) + run.ascent;
            Line { run, x, baseline, sx, ox }
        })
        .collect();
    let sq = if squeeze { vec![(sx * fs / o.size * 100.0).round() / 100.0] } else { Vec::new() };
    let w = (span_w * sx).min(if multi && !over { avail } else { f32::INFINITY });
    Layout { lines, span_w, span_h, w, h: span_h, sq, over }
}

/// Połowa interlinii nad wierszem (jak LayoutNG: (wysokość linii − (A + D)) / 2).
fn half_leading(line_h: f32, run: &Run) -> f32 {
    (line_h - (run.ascent + run.descent)) / 2.0
}

/// Słowa do łamania: `[[wyróżnienia]]` niepodzielne, twarda spacja nie dzieli (`breakTokens`).
fn break_tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while !rest.is_empty() {
        let t = rest.trim_start_matches([' ', '\t', '\r', '\n']);
        if t.is_empty() {
            break;
        }
        if t.starts_with("[[") {
            if let Some(end) = t.find("]]") {
                out.push(t[..end + 2].to_string());
                rest = &t[end + 2..];
                continue;
            }
        }
        let end = t.find([' ', '\t', '\r', '\n']).unwrap_or(t.len());
        let end = t[..end].find("[[").filter(|&i| i > 0).unwrap_or(end);
        out.push(t[..end].to_string());
        rest = &t[end..];
    }
    if out.is_empty() {
        out.push(line.to_string());
    }
    out
}

/// `balanceLines`: k wierszy o możliwie równej szerokości.
#[allow(clippy::needless_range_loop)]
fn balance_lines(line: &str, k: usize, w: &dyn Fn(&str) -> f32) -> Vec<String> {
    let toks = break_tokens(line);
    let n = toks.len();
    let k = k.min(n);
    if k <= 1 {
        return vec![line.to_string()];
    }
    let sw = w(" ");
    let tw: Vec<f32> = toks.iter().map(|t| w(t)).collect();
    let mut pre = vec![0.0f32];
    for i in 0..n {
        pre.push(pre[i] + tw[i]);
    }
    let width = |i: usize, j: usize| pre[j] - pre[i] + (j - i - 1) as f32 * sw;
    let mut best = vec![vec![f32::INFINITY; n + 1]; k + 1];
    let mut cut = vec![vec![0usize; n + 1]; k + 1];
    for m in 1..=k {
        for j in m..=n {
            if m == 1 {
                best[1][j] = width(0, j);
                cut[1][j] = 0;
                continue;
            }
            let (mut b, mut c) = (f32::INFINITY, m - 1);
            for i in (m - 1)..j {
                let v = best[m - 1][i].max(width(i, j));
                if v < b - 0.01 {
                    b = v;
                    c = i;
                }
            }
            best[m][j] = b;
            cut[m][j] = c;
        }
    }
    let mut out = Vec::new();
    let mut j = n;
    for m in (1..=k).rev() {
        let i = cut[m][j];
        out.insert(0, toks[i..j].join(" "));
        j = i;
    }
    out
}

/// `fillLines`: wypełnianie kolejnych wierszy ściśniętych najwyżej do `tol` (próg obniżany do `min`).
fn fill_lines(line: &str, k: usize, w: &dyn Fn(&str) -> f32, avail: f32, tol: f32, min: f32) -> Vec<String> {
    let toks = break_tokens(line);
    let sw = w(" ");
    let tw: Vec<f32> = toks.iter().map(|t| w(t)).collect();
    let greedy = |lim: f32| -> Vec<(usize, usize, f32)> {
        let mut out = Vec::new();
        let (mut a, mut cw) = (0usize, tw[0]);
        for (i, &t) in tw.iter().enumerate().skip(1) {
            if cw + sw + t > lim && out.len() < k - 1 {
                out.push((a, i, cw));
                a = i;
                cw = t;
            } else {
                cw += sw + t;
            }
        }
        out.push((a, toks.len(), cw));
        out
    };
    let mut t = min.max(tol);
    let mut ls = greedy(avail / t);
    while t > min + 1e-6 && ls.last().is_some_and(|l| l.2 > avail / t) {
        t = min.max(t - 0.01);
        ls = greedy(avail / t);
    }
    ls.into_iter().map(|(a, b, _)| toks[a..b].join(" ")).collect()
}

/// `fitLines`: tryby `squeeze-lines` i `squeeze-each` – każdy wiersz ściskany osobno.
pub fn fit_lines(db: &FontDb, base: &Style, text: &str, o: &Fit) -> Layout {
    let l = o.max_lines.round().max(1.0) as usize;
    let avail = o.avail.max(1.0);
    let min = squeeze_floor(o.min_squeeze);
    let each = o.mode == "squeeze-each";
    let fill = o.line_break == "fill";
    let room = if fill {
        avail
    } else if each {
        avail / min
    } else {
        avail
    };
    let lh = if o.line_height > 0.0 { o.line_height } else { 1.15 };
    let st = Style { size: o.size, ..base.clone() };
    let w = |s: &str| measure(db, &st, s);
    let mut lines: Vec<String> = text.split('\n').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if lines.is_empty() {
        lines.push(String::new());
    }
    if lines.len() > l {
        let tail = lines[l - 1..].join(" ");
        lines.truncate(l - 1);
        lines.push(tail);
    }
    let mut changed = true;
    while changed && lines.len() < l {
        changed = false;
        let ws: Vec<f32> = lines.iter().map(|s| w(s)).collect();
        let max = ws.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let i = ws.iter().position(|&x| x == max).unwrap_or(0);
        if ws[i] > room {
            let parts = if fill {
                fill_lines(
                    &lines[i],
                    l - lines.len() + 1,
                    &w,
                    avail,
                    squeeze_floor(if o.fill_squeeze > 0.0 { o.fill_squeeze } else { 0.9 }),
                    min,
                )
            } else {
                balance_lines(&lines[i], (l - lines.len() + 1).min((ws[i] / room).ceil() as usize), &w)
            };
            if parts.len() > 1 {
                lines.splice(i..=i, parts);
                changed = true;
            }
        }
    }
    // każdy wiersz samodzielnie poprawny (znaczniki przeniesione przez podział)
    let lines = richtext::carry_lines(&lines);
    let mut out = Layout::default();
    let mut max_w = 0.0f32;
    let mut top = 0.0f32;
    let mut placed: Vec<(Run, f32, f32, f32)> = Vec::new(); // run, top, line_h, sx
    for line in &lines {
        let mut run = shape(db, &st, line);
        let mut width = run.offset_width();
        let mut s = if width > avail { avail / width } else { 1.0 };
        let mut f = 1.0;
        if s < min && each {
            s = min;
            out.over = true;
        } else if s < min {
            f = s / min;
            run = shape(db, &Style { size: o.size * f, ..st.clone() }, line);
            width = run.offset_width();
            s = if width > avail { min.max(avail / width) } else { 1.0 };
        }
        let s = if s < 0.999 { s } else { 1.0 };
        out.sq.push((s * f * 100.0).round() / 100.0);
        max_w = max_w.max(width * s);
        let line_h = o.size * f * lh;
        placed.push((run, top, line_h, s));
        top += line_h;
    }
    // element `sp` (flex, kolumna): szerokość = najszerszy wiersz przed ściśnięciem, wiersze wyrównane w nim
    let span_w = placed.iter().map(|(r, ..)| r.offset_width()).fold(0.0, f32::max);
    for (run, top, line_h, sx) in placed {
        let rw = run.offset_width();
        let x = match o.align {
            "right" => span_w - rw,
            "center" => (span_w - rw) / 2.0,
            _ => 0.0,
        };
        let ox = x + match o.align {
            "right" => rw,
            "center" => rw / 2.0,
            _ => 0.0,
        };
        let baseline = top + half_leading(line_h, &run) + run.ascent;
        out.lines.push(Line { run, x, baseline, sx, ox });
    }
    out.span_w = span_w;
    out.span_h = top.round();
    out.w = max_w.min(avail);
    out.h = out.span_h;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_keep_highlights_whole() {
        assert_eq!(break_tokens("a [[b c]] d"), vec!["a", "[[b c]]", "d"]);
        assert_eq!(break_tokens("  "), vec!["  "]);
    }

    #[test]
    fn balance_splits_evenly_by_width() {
        let w = |s: &str| s.chars().count() as f32 * 10.0;
        assert_eq!(balance_lines("aaa bbb ccc ddd", 2, &w), vec!["aaa bbb", "ccc ddd"]);
        assert_eq!(balance_lines("one", 3, &w), vec!["one"]);
    }

    #[test]
    fn fill_takes_words_greedily() {
        let w = |s: &str| s.chars().count() as f32 * 10.0;
        // 100 px przy ściśnięciu 0,9 → limit ~111 px: „aaaa bbbb” (90) mieści się, „cccc” przechodzi dalej
        assert_eq!(fill_lines("aaaa bbbb cccc", 2, &w, 100.0, 0.9, 0.6), vec!["aaaa bbbb", "cccc"]);
    }

    #[test]
    fn squeeze_floor_limits() {
        assert_eq!(squeeze_floor(0.0), 0.6);
        assert_eq!(squeeze_floor(0.1), 0.3);
        assert_eq!(squeeze_floor(2.0), 1.0);
    }

    #[test]
    fn breaks_like_the_browser() {
        assert!(break_after(' ', Some('a')));
        assert!(!break_after(' ', Some(' ')));
        assert!(break_after('-', Some('b')));
        assert!(!break_after('-', Some(' ')));
        assert!(!break_after('a', Some('b')));
    }
}
