//! Ticker: treść węzła (`ticker_content`), rotacja wiadomości w slotach (`ticker_slots`) i segmenty paska (`segment`).

use super::*;

impl Player {
    pub(super) fn ticker_content(&mut self, i: usize, clk: Clock, res: &Resources, epoch: u64, _dt: f64) {
        let raw = self.txt(i, "text");
        let label = self.txt(i, "label");
        if self.states[i].tk_raw.as_ref() != Some(&(raw.clone(), label.clone())) {
            let items = ticker::parse_items(&strip_markup(&raw));
            self.states[i].tk_page = ticker::page_cat_colors(&items, &label);
            self.states[i].tk_items = items;
            self.states[i].tk_raw = Some((raw, label.clone()));
        }
        let mode = self.txt(i, "tickerMode");
        if !matches!(mode.as_str(), "crawl" | "roll" | "wipe" | "whip") {
            let msg = format!("ticker w trybie {mode}");
            self.issue(i, &msg);
            return;
        }
        self.live = true;
        let font = self.txt(i, "font");
        if !self.font_ok(i, &font, res) && crate::font::is_font_file(&font) {
            return;
        }
        let items = self.states[i].tk_items.clone();
        let up = truthy(&self.prop(i, "upper"));
        let w = self.states[i].w;
        let h = self.states[i].h;
        let mut colors = ticker::parse_cat_colors(&self.txt(i, "catColors"));
        colors.extend(self.states[i].tk_page.clone());
        let cat_bg = self.txt(i, "catBg");
        let cat_fg = self.txt(i, "catFg");
        let lab_w = if label.is_empty() { 0.0 } else { self.num(i, "catW").max(0.0) };
        let size = self.num(i, "size");
        let style = self.style(i, size, false);
        let fg = color(&self.txt(i, "color")).unwrap_or([1.0; 4]);
        let sh = self.num(i, "shadow");
        let shadow = (sh > 0.0).then(|| ((sh / 4.0).max(1.0), sh / 2.0, [0.0, 0.0, 0.0, 0.65f32]));
        let sep = self.txt(i, "sep");
        let sep_style = self.txt(i, "sepStyle");
        let sep_color = self.txt(i, "sepColor");
        let lab_size = {
            let ls = self.num(i, "labelSize");
            if ls > 0.0 {
                ls
            } else {
                size * 0.8
            }
        };
        let lab_weight = {
            let lw = self.txt(i, "labelWeight");
            if lw.is_empty() {
                style.weight
            } else {
                weight_of(&Value::from(lw))
            }
        };
        if mode != "crawl" {
            // roll, wipe, whip (`tickerSlots`): dwie przegródki (bieżąca i następna wiadomość) i wycieraczka
            self.states[i].crawl = None;
            let cat_w = if !label.is_empty() || items.iter().any(|x| !x.cat.is_empty()) { self.num(i, "catW").max(0.0) } else { 0.0 };
            let o = SlotOpts {
                res,
                mode: &mode,
                w,
                h,
                cat_w,
                up,
                label: &label,
                colors: &colors,
                cat_bg: &cat_bg,
                cat_fg: color(&cat_fg).unwrap_or([1.0; 4]),
                style: &style,
                lab_style: Style { size: lab_size, weight: lab_weight, ..style.clone() },
                fg,
                shadow,
                fit: self.txt(i, "textFit"),
                min: text::squeeze_floor(self.num(i, "squeezeMin")),
                hold: f64::from(self.num(i, "hold")).max(1.0),
                gleam: self.num(i, "gleam"),
                now: clk.now,
            };
            let slots = self.states[i].slots.take();
            let (extra, slots) = ticker_slots(&o, &items, slots);
            self.states[i].slots = slots;
            self.states[i].extra = extra;
            return;
        }
        self.states[i].slots = None;
        let style_key = format!(
            "{up}|{sep}|{sep_style}|{sep_color}|{cat_bg}|{cat_fg}|{:?}|{}|{style:?}|{fg:?}|{shadow:?}|{h}|{epoch}",
            self.txt(i, "catColors"),
            items.iter().map(|x| x.color.as_str()).collect::<Vec<_>>().join(",")
        );
        let vw = (w - lab_w).max(1.0) as f64;
        let speed = f64::from(self.num(i, "speed"));
        let crawl_from = self.txt(i, "crawlFrom");
        let now = clk.now;
        let seg_ctx = SegCtx {
            res,
            style: &style,
            up,
            sep: &sep,
            sep_style: &sep_style,
            sep_color: &sep_color,
            colors: &colors,
            cat_bg: &cat_bg,
            cat_fg: &cat_fg,
            h,
            fg,
            shadow,
        };
        let st = &mut self.states[i];
        let tk = st.crawl.get_or_insert_with(|| Crawl {
            segs: VecDeque::new(),
            d: 0.0,
            cursor: 0.0,
            next_idx: 0,
            items: items.clone(),
            style_key: String::new(),
            last: None,
            fresh: true,
        });
        if tk.style_key != style_key {
            tk.style_key = style_key;
            let mut pos = tk.segs.front().map(|s| s.pos).unwrap_or(tk.cursor);
            for sg in tk.segs.iter_mut() {
                if let Some(found) = items.iter().find(|x| x.text == sg.item.text && x.cat == sg.item.cat) {
                    sg.item = found.clone();
                }
                let (node, sw) = segment(&seg_ctx, &sg.item);
                sg.node = Arc::new(node);
                sg.pos = pos;
                sg.w = sw;
                pos += sw;
            }
            tk.cursor = pos;
        }
        if tk.items != items {
            tk.next_idx = match tk.segs.back() {
                Some(last) if !tk.items.is_empty() => ticker::continuation(&tk.items, last.idx, &items),
                _ => 0,
            };
            tk.items = items.clone();
        }
        let dtk = tk.last.map(|l| ((now - l) / 1000.0).clamp(0.0, 0.25)).unwrap_or(0.0);
        tk.last = Some(now);
        tk.d += speed * dtk;
        if tk.segs.is_empty() {
            tk.cursor = tk.cursor.max(tk.d);
        }
        if tk.fresh {
            tk.fresh = false;
            if crawl_from == "left" {
                tk.cursor = tk.d - vw;
            }
        }
        // crawlFill: doklejanie odcinków, aż wypełnią okno (z zapasem 40 px)
        let n = items.len();
        let mut guard = 0;
        while n > 0 && tk.cursor - tk.d < vw + 40.0 && guard < 200 {
            guard += 1;
            let idx = tk.next_idx % n;
            let item = items[idx].clone();
            let (node, sw) = segment(&seg_ctx, &item);
            tk.segs.push_back(Seg { item, idx, pos: tk.cursor, w: sw, node: Arc::new(node) });
            tk.cursor += sw;
            tk.next_idx = (idx + 1) % n;
        }
        while tk.segs.front().is_some_and(|s| s.pos + s.w < tk.d - vw) {
            tk.segs.pop_front();
        }
        if tk.d > 50000.0 {
            let off = tk.segs.front().map(|s| s.pos).unwrap_or(tk.d);
            for sg in tk.segs.iter_mut() {
                sg.pos -= off;
            }
            tk.d -= off;
            tk.cursor -= off;
        }
        // okno taśmy: przycięte, za etykietą
        let mut window = DNode::new(Transform::from_translate(lab_w, 0.0), vw as f32, h);
        window.overflow = Some(0.0);
        let shift = vw - tk.d;
        for sg in &tk.segs {
            let mut s = (*sg.node).clone();
            s.m = Transform::from_translate((shift + sg.pos) as f32, 0.0);
            window.children.push(s);
        }
        let mut extra = vec![window];
        if lab_w > 0.0 {
            let lab_style = Style { size: lab_size, weight: lab_weight, ..style.clone() };
            let bg = ticker::cat_color_of(&Item { cat: label.clone(), color: String::new(), text: String::new() }, &colors, &cat_bg, "");
            let mut lab = DNode::new(Transform::identity(), lab_w, h);
            lab.overflow = Some(0.0);
            if let Some(c) = color(&bg) {
                lab.paints.push(Paint::Box { radius: Radius::Px(0.0), fill: Fill::Solid(c) });
            }
            let shown = if up { label.to_uppercase() } else { label.clone() };
            if let Some(fgc) = color(&cat_fg) {
                if let Some((p, _w)) = label_text(res, &lab_style, &shown, lab_w, h, fgc) {
                    lab.paints.push(p);
                }
            }
            let gp = self.num(i, "gleam");
            if let Some(g) = gleam(gp, now, lab_w, h) {
                lab.children.push(g);
            }
            extra.push(lab);
        }
        self.states[i].extra = extra;
    }
}

/// Stan rotacji wiadomości (`it.tk` w trybie `slots`).
pub(super) struct Slots {
    items: Vec<Item>,
    cur: Option<Item>,
    cur_idx: Option<usize>,
    prev: Option<Item>,
    next_idx: usize,
    t0: f64,
}

pub(super) struct SlotOpts<'a> {
    res: &'a Resources,
    mode: &'a str,
    w: f32,
    h: f32,
    cat_w: f32,
    up: bool,
    label: &'a str,
    colors: &'a HashMap<String, String>,
    cat_bg: &'a str,
    cat_fg: Color,
    style: &'a Style,
    lab_style: Style,
    fg: Color,
    shadow: Option<(f32, f32, Color)>,
    fit: String,
    min: f32,
    hold: f64,
    gleam: f32,
    now: f64,
}

pub(super) fn ease_io(u: f64) -> f64 {
    if u < 0.5 {
        4.0 * u * u * u
    } else {
        1.0 - (-2.0 * u + 2.0).powi(3) / 2.0
    }
}

/// Przegródka: pole tekstu (`box`) i etykieta (`cat`) dla wiadomości `x`.
pub(super) fn slot_nodes(o: &SlotOpts, x: &Item) -> (DNode, DNode) {
    let (w, h, cat_w) = (o.w, o.h, o.cat_w);
    // etykieta: kategoria albo etykieta domyślna, tło w kolorze kategorii
    let lab = if x.cat.is_empty() { o.label.to_string() } else { x.cat.clone() };
    let mut cat = DNode::new(Transform::identity(), cat_w, h);
    cat.overflow = Some(0.0);
    if !lab.is_empty() {
        if let Some(c) = color(&ticker::cat_color_of(x, o.colors, o.cat_bg, o.label)) {
            cat.paints.push(Paint::Box { radius: Radius::Px(0.0), fill: Fill::Solid(c) });
        }
        let shown = if o.up { lab.to_uppercase() } else { lab };
        if let Some((p, _)) = label_text(o.res, &o.lab_style, &shown, cat_w, h, o.cat_fg) {
            cat.paints.push(p);
        }
    }
    // treść: od catW, margines 18 px, dopasowanie jak `fill` w `tickerSlots`
    let bw = (w - cat_w).max(0.0);
    let mut bx = DNode::new(Transform::from_translate(cat_w, 0.0), bw, h);
    bx.overflow = Some(0.0);
    let body = if o.up { x.text.to_uppercase() } else { x.text.clone() };
    let avail = (w - cat_w - 36.0).max(1.0);
    let (mut block, sw, _) = line_block(o.res, o.style, &body, h);
    let mut sx = 1.0f32;
    if sw > avail && o.fit != "none" {
        let s = avail / sw;
        if o.fit == "shrink" {
            let st = Style { size: o.style.size * s, ..o.style.clone() };
            let (b2, sw2, _) = line_block(o.res, &st, &body, h);
            block = b2;
            let s2 = (avail / sw2.max(1.0)).min(1.0);
            if s2 < 0.999 {
                sx = s2;
            }
        } else if s < o.min && o.fit == "squeeze-shrink" {
            let st = Style { size: o.style.size * s / o.min, ..o.style.clone() };
            let (b2, sw2, _) = line_block(o.res, &st, &body, h);
            block = b2;
            sx = o.min.max((avail / sw2.max(1.0)).min(1.0));
        } else {
            sx = o.min.max(s);
        }
    }
    if sx < 0.999 {
        let mut lines = block.lines.clone();
        for l in &mut lines {
            l.sx = sx;
            l.ox = 0.0;
        }
        block = Arc::new(TextBlock { key: block.key ^ u64::from(sx.to_bits()).rotate_left(17), lines, span: [0.0; 4] });
    }
    bx.paints.push(Paint::Text { block, color: o.fg, shadow: o.shadow, x: 18.0, y: 0.0 });
    (bx, cat)
}

/// `tickerSlots`: tryby `roll` (góra–dół), `wipe` (wycieraczka) i `whip` (etykieta przejeżdża przez pasek).
pub(super) fn ticker_slots(o: &SlotOpts, items: &[Item], state: Option<Slots>) -> (Vec<DNode>, Option<Slots>) {
    let n = items.len();
    let now = o.now;
    let mut tk = state.unwrap_or_else(|| Slots {
        items: items.to_vec(),
        cur: items.first().cloned(),
        cur_idx: (n > 0).then_some(0),
        prev: None,
        next_idx: usize::from(n > 1),
        t0: now,
    });
    if tk.items != items {
        if tk.cur.is_none() && n > 0 {
            tk.cur = Some(items[0].clone());
            tk.cur_idx = Some(0);
            tk.next_idx = usize::from(n > 1);
            tk.t0 = now;
        } else if let Some(ci) = tk.cur_idx {
            let next = ticker::continuation(&tk.items, ci, items);
            tk.cur_idx = tk.cur.as_ref().and_then(|c| items.iter().position(|x| x.text == c.text && x.cat == c.cat));
            tk.next_idx = next;
        } else {
            let at = tk.items.get(tk.next_idx).and_then(|c| items.iter().position(|x| x.text == c.text && x.cat == c.cat));
            tk.next_idx = at.unwrap_or_else(|| ticker::continuation(&tk.items, tk.next_idx, items));
        }
        tk.items = items.to_vec();
    }
    let mut out = Vec::new();
    let gleam_on = |cat: &mut DNode| {
        if let Some(g) = gleam(o.gleam, now, o.cat_w, o.h) {
            cat.children.push(g);
        }
    };
    let Some(cur0) = tk.cur.clone().filter(|_| n > 0) else { return (out, Some(tk)) };
    let _ = cur0;
    let single = n == 1 && tk.cur_idx == Some(0);
    let hold = o.hold;
    if single || now - tk.t0 > hold * 50000.0 {
        tk.t0 = now;
    }
    while (now - tk.t0) / 1000.0 >= hold {
        tk.prev = tk.cur.clone();
        let ci = tk.next_idx % n;
        tk.cur_idx = Some(ci);
        tk.cur = Some(items[ci].clone());
        tk.next_idx = (ci + 1) % n;
        tk.t0 += hold * 1000.0;
    }
    let cur = tk.cur.clone().expect("set above");
    let next = items[tk.next_idx % n].clone();
    let ph = (now - tk.t0) / 1000.0;
    let (h, w, cat_w) = (o.h, o.w, o.cat_w);
    let lab_key = |x: &Item| {
        let l = if x.cat.is_empty() { o.label } else { &x.cat };
        format!("{l}#{}", ticker::cat_color_of(x, o.colors, o.cat_bg, o.label))
    };
    let (mut a_box, mut a_cat) = slot_nodes(o, &cur);
    let mut b: Option<(DNode, DNode)> = None;
    let mut a_cat_on = cat_w > 0.0;
    let mut wiper: Option<(f32, String)> = None;
    if !single {
        match o.mode {
            "roll" => {
                let t = (0.55f64).min(hold * 0.3);
                if ph > hold - t {
                    let u = ease_io((ph - (hold - t)) / t) as f32;
                    let (mut b_box, mut b_cat) = slot_nodes(o, &next);
                    a_box.m = Transform::from_translate(cat_w, -u * h);
                    b_box.m = Transform::from_translate(cat_w, (1.0 - u) * h);
                    let mut b_cat_on = cat_w > 0.0;
                    if lab_key(&cur) != lab_key(&next) {
                        a_cat.m = Transform::from_translate(0.0, -u * h);
                        b_cat.m = Transform::from_translate(0.0, (1.0 - u) * h);
                    } else {
                        b_cat_on = false;
                    }
                    if !b_cat_on {
                        b_cat.opacity = 0.0;
                    }
                    b = Some((b_box, b_cat));
                }
            }
            "whip" => {
                let t_in = (0.25f64).min(hold * 0.1);
                let t_out = (0.35f64).min(hold * 0.1);
                let t_w = if cat_w > 0.0 { (1.5f64).min(hold * 0.35) } else { 0.0 };
                let out_at = hold - t_w - t_out;
                a_box.opacity = (if ph < t_in {
                    ph / t_in
                } else if ph < out_at {
                    1.0
                } else {
                    (1.0 - (ph - out_at) / t_out).max(0.0)
                }) as f32;
                if t_w > 0.0 && ph >= hold - t_w {
                    let u = (ph - (hold - t_w)) / t_w;
                    let travel = f64::from((w - cat_w).max(0.0));
                    let x = if u < 0.65 { ease_io(u / 0.65) * travel } else { (1.0 - ease_io((u - 0.65) / 0.35)) * travel };
                    if u < 0.325 {
                        a_cat.m = Transform::from_translate(x as f32, 0.0);
                    } else {
                        // etykieta następnej wiadomości (przegródka B bez treści)
                        a_cat_on = false;
                        let (mut b_box, mut b_cat) = slot_nodes(o, &next);
                        b_box.opacity = 0.0;
                        b_cat.m = Transform::from_translate(x as f32, 0.0);
                        b = Some((b_box, b_cat));
                    }
                }
            }
            _ => {
                // wipe: odsłonięcie od lewej, na końcu zmazanie; krawędź prowadzi wycieraczka w kolorze kategorii
                let t = (0.45f64).min(hold * 0.25);
                let bw = w - cat_w;
                let wc = ticker::cat_color_of(&cur, o.colors, o.cat_bg, o.label);
                if ph < t && tk.prev.is_some() {
                    let u = ease_io(ph / t) as f32;
                    a_box.clip = Some([0.0, (1.0 - u) * 100.0, 0.0, 0.0]);
                    if tk.prev.as_ref().map(lab_key) != Some(lab_key(&cur)) {
                        a_cat.clip = Some([(1.0 - u) * 100.0, 0.0, 0.0, 0.0]);
                    }
                    if u < 0.999 {
                        wiper = Some((cat_w + u * bw - 3.0, wc));
                    }
                } else if ph > hold - t {
                    let u = ease_io((ph - (hold - t)) / t) as f32;
                    a_box.clip = Some([0.0, 0.0, 0.0, u * 100.0]);
                    if lab_key(&cur) != lab_key(&next) {
                        a_cat.clip = Some([0.0, 0.0, u * 100.0, 0.0]);
                    }
                    if u > 0.001 {
                        wiper = Some((cat_w + u * bw - 3.0, wc));
                    }
                }
            }
        }
    }
    // kolejność jak w DOM: pola treści, potem etykiety (z-index 2), na wierzchu wycieraczka (z-index 3)
    out.push(a_box);
    let mut b_cat_node = None;
    if let Some((b_box, b_cat)) = b {
        out.push(b_box);
        b_cat_node = Some(b_cat);
    }
    if a_cat_on {
        gleam_on(&mut a_cat);
        out.push(a_cat);
    }
    if let Some(mut c) = b_cat_node.filter(|_| cat_w > 0.0) {
        if c.opacity > 0.0 {
            gleam_on(&mut c);
        }
        out.push(c);
    }
    if let Some((left, c)) = wiper {
        let mut wp = DNode::new(Transform::from_translate(left, 0.0), 6.0, h);
        if let Some(col) = color(&c) {
            wp.paints.push(Paint::Box { radius: Radius::Px(0.0), fill: Fill::Solid(col) });
        }
        out.push(wp);
    }
    (out, Some(tk))
}

/// Wspólne dane odcinków taśmy crawla.
pub(super) struct SegCtx<'a> {
    res: &'a Resources,
    style: &'a Style,
    up: bool,
    sep: &'a str,
    sep_style: &'a str,
    sep_color: &'a str,
    colors: &'a HashMap<String, String>,
    cat_bg: &'a str,
    cat_fg: &'a str,
    h: f32,
    fg: Color,
    shadow: Option<(f32, f32, Color)>,
}

/// Blok jednego wiersza tekstu (bez dopasowania) z linią bazową wyśrodkowaną w pionie w `h`.
pub(super) fn line_block(res: &Resources, style: &Style, text: &str, h: f32) -> (Arc<TextBlock>, f32, f32) {
    let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
    let run = text::shape(&db, style, text);
    drop(db);
    let (lh, base) = normal_metrics(res, style);
    let _ = lh;
    let w = run.offset_width();
    let baseline = h / 2.0 + base;
    let key = hash_str(&[text, &format!("{style:?}|{h}")]);
    (Arc::new(TextBlock { key, lines: vec![text::Line { run, x: 0.0, baseline, sx: 1.0, ox: 0.0 }], span: [0.0; 4] }), w, lh)
}

/// Etykieta tickera: tekst wyśrodkowany w polu `w`×`h`.
pub(super) fn label_text(res: &Resources, style: &Style, text: &str, w: f32, h: f32, fg: Color) -> Option<(Paint, f32)> {
    let (block, tw, _) = line_block(res, style, text, h);
    Some((Paint::Text { block, color: fg, shadow: None, x: (w - tw) / 2.0, y: 0.0 }, tw))
}

/// Pas błysku (gleam) etykiety: pozycja z czasu.
pub(super) fn gleam(period: f32, now: f64, w: f32, h: f32) -> Option<DNode> {
    if period <= 0.0 {
        return None;
    }
    let p = f64::from(period);
    let dur = (p * 0.6).min(1.6);
    let u = ((now / 1000.0) % p) / dur;
    if u >= 1.0 {
        return None;
    }
    let gw = w * 0.45;
    let left = w * (-0.6 + u as f32 * 1.7);
    let (ox, oy) = (gw / 2.0, h / 2.0);
    let skew = Transform::from_row(1.0, 0.0, (-20f32).to_radians().tan(), 1.0, 0.0, 0.0);
    let m =
        mul(mul(mul(Transform::from_translate(left, 0.0), Transform::from_translate(ox, oy)), skew), Transform::from_translate(-ox, -oy));
    let mut g = DNode::new(m, gw, h);
    let clear = [1.0, 1.0, 1.0, 0.0];
    g.paints.push(Paint::Box {
        radius: Radius::Px(0.0),
        fill: Fill::Linear { angle: 90.0, stops: vec![(0.0, clear), (0.5, [1.0, 1.0, 1.0, 0.4]), (1.0, clear)] },
    });
    Some(g)
}

/// Odcinek taśmy (`tickerSegment`): [plakietka kategorii] treść [separator]; zwraca węzeł i szerokość (`offsetWidth`).
pub(super) fn segment(c: &SegCtx, x: &Item) -> (DNode, f64) {
    let size = c.style.size;
    let mut node = DNode::new(Transform::identity(), 0.0, c.h);
    let mut cx = 0.0f32;
    if !x.cat.is_empty() {
        let bst = Style { size: size * 0.78, ..c.style.clone() };
        let cat = if c.up { x.cat.to_uppercase() } else { x.cat.clone() };
        let (block, tw, lh) = line_block(c.res, &bst, &cat, c.h);
        let (px, py) = (bst.size * 0.45, bst.size * 0.05);
        let bw = tw + 2.0 * px;
        let bh = lh + 2.0 * py;
        let bg = ticker::cat_color_of(x, c.colors, c.cat_bg, "");
        if let Some(col) = color(&bg) {
            node.paints.push(Paint::Rect {
                x: cx,
                y: (c.h - bh) / 2.0,
                w: bw,
                h: bh,
                radius: Radius::Px(3.0),
                skew: 0.0,
                fill: Fill::Solid(col),
            });
        }
        if let Some(fg) = color(c.cat_fg) {
            node.paints.push(Paint::Text { block, color: fg, shadow: None, x: cx + px, y: 0.0 });
        }
        cx += bw + bst.size * 0.5; // margin-right: .5em plakietki (jej własny rozmiar fontu)
    }
    let body = if c.up { x.text.to_uppercase() } else { x.text.clone() };
    let (block, tw, _) = line_block(c.res, c.style, &body, c.h);
    node.paints.push(Paint::Text { block, color: c.fg, shadow: c.shadow, x: cx, y: 0.0 });
    cx += tw;
    if !c.sep_style.is_empty() && c.sep_style != "text" {
        let (w, h, round, skew) = match c.sep_style {
            "square" => (0.3, 0.3, false, 0.0),
            "pipe" => (0.12, 0.95, false, 0.0),
            "skew" => (0.16, 1.05, false, -20.0),
            _ => (0.32, 0.32, true, 0.0),
        };
        let (w, h) = (w * size, h * size);
        let m = size * 0.9;
        let (col, alpha) = match color(c.sep_color) {
            Some(col) if !c.sep_color.is_empty() => (col, 1.0),
            _ => (c.fg, 0.55),
        };
        let col = [col[0], col[1], col[2], col[3] * alpha];
        let radius = if round { Radius::Ellipse } else { Radius::Px(0.0) };
        node.paints.push(Paint::Rect { x: cx + m, y: (c.h - h) / 2.0, w, h, radius, skew, fill: Fill::Solid(col) });
        cx += w + 2.0 * m;
    } else {
        let sep_fg = color(c.sep_color).filter(|_| !c.sep_color.is_empty()).unwrap_or(c.fg);
        let (block, tw, _) = line_block(c.res, c.style, c.sep, c.h);
        node.paints.push(Paint::Text { block, color: sep_fg, shadow: c.shadow, x: cx, y: 0.0 });
        cx += tw;
    }
    // offsetWidth odcinka (całkowite px), najmniej 1
    let sw = f64::from(cx.round().max(1.0));
    if std::env::var_os("VIZRS_RENDER_LAYOUT").is_some() {
        eprintln!("odcinek „{}”: {cx:.3} → {sw}", x.text.chars().take(20).collect::<String>());
    }
    node.w = sw as f32;
    (node, sw)
}
