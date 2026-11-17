//! Treść węzłów tekstowych i zegara (`text_content`): wartość pola, formatowanie, układ, dopasowanie.

use super::*;

impl Player {
    pub(super) fn text_content(&mut self, i: usize, node: &Node, clk: Clock, res: &Resources, epoch: u64) {
        let mut text = if node.kind == "clock" {
            self.live = true;
            let mode = self.txt(i, "mode");
            let el_s = (clk.now - self.wall_start) / 1000.0;
            let tz = self.txt(i, "tz");
            let secs = match mode.as_str() {
                "up" => el_s,
                "down" => f64::from(self.num(i, "seconds")) - el_s,
                "target" => {
                    let tg = clock::zone_time(&self.txt(i, "target"), &tz);
                    if tg.is_nan() {
                        0.0
                    } else {
                        (tg - clk.wall) / 1000.0
                    }
                }
                _ => 0.0,
            };
            let fmt = self.txt(i, "format");
            clock::fmt_clock(if fmt.is_empty() { "HH:mm:ss" } else { &fmt }, &mode, secs, &tz, clk.wall)
        } else {
            self.txt(i, "text")
        };
        self.states[i].cy_op = 1.0;
        let cyc = if node.kind == "text" { self.num(i, "cycle") } else { 0.0 };
        if cyc > 0.0 && text.contains('\n') {
            self.live = true;
            let lines: Vec<&str> = text.split('\n').filter(|l| !l.trim().is_empty()).collect();
            let el_s = (clk.now - self.wall_start) / 1000.0;
            let cyc = f64::from(cyc);
            let pick = if lines.is_empty() { "" } else { lines[(el_s / cyc).floor() as usize % lines.len()] };
            text = pick.to_string();
            let ph = el_s % cyc;
            let f = (cyc / 4.0).min(0.35);
            self.states[i].cy_op = (ph / f).min((cyc - ph) / f).clamp(0.0, 1.0) as f32;
        }
        if truthy(&self.prop(i, "upper")) {
            text = text.to_uppercase();
        }
        let font = self.txt(i, "font");
        // font z assets jeszcze się wczytuje: tekstu nie ma (`font-display: block`); font spoza wbudowanych –
        // zastępczy z łańcucha (jak Chromium bez Google Fonts), zgłoszony w `issues`
        if !self.font_ok(i, &font, res) && crate::font::is_font_file(&font) {
            return;
        }
        let size = self.num(i, "size");
        let pad_x = self.num(i, "padX");
        let w = self.states[i].w;
        let h = self.states[i].h;
        let align = self.txt(i, "align");
        let valign = self.txt(i, "valign");
        let fit_mode = self.txt(i, "textFit");
        let max_lines = self.num(i, "maxLines");
        let lh = self.num(i, "lineHeight");
        let sq_min = self.num(i, "squeezeMin");
        let line_break = self.txt(i, "lineBreak");
        let fill_sq = self.num(i, "fillSqueeze");
        let mut style = self.style(i, size, true);
        style.hl = Some((
            crate::text::css_color(&self.txt(i, "hlColor")).unwrap_or([0.0; 4]),
            crate::text::css_color(&self.txt(i, "hlText")).unwrap_or([1.0; 4]),
        ));
        let key = format!("{text}|{style:?}|{fit_mode}|{max_lines}|{sq_min}|{line_break}|{fill_sq}|{w}|{pad_x}|{lh}|{align}|{epoch}");
        if self.states[i].key != key || self.states[i].layout.is_none() {
            let fo = Fit {
                avail: w - 2.0 * pad_x,
                size,
                max_lines,
                mode: &fit_mode,
                align: &align,
                line_height: lh,
                min_squeeze: sq_min,
                line_break: &line_break,
                fill_squeeze: fill_sq,
            };
            let db = res.fonts.read().unwrap_or_else(|e| e.into_inner());
            let layout = if fit_mode == "squeeze-lines" || fit_mode == "squeeze-each" {
                text::fit_lines(&db, &style, &text, &fo)
            } else {
                text::fit_text(&db, &style, &text, &fo)
            };
            drop(db);
            // Blink: element z przekształceniem innym niż przesunięcie (tu `scaleX`) ma położenie zaokrąglone do piksela –
            // w `fitLines` każdy ściśnięty wiersz osobno (jego lewa krawędź w układzie pola)
            let x0 = span_x(&layout, pad_x, w, &align);
            let mut lines = layout.lines.clone();
            if layout.per_line {
                for l in lines.iter_mut().filter(|l| l.sx < 0.999) {
                    let d = (x0 + l.x).round() - (x0 + l.x);
                    l.x += d;
                    l.ox += d;
                }
            }
            let st = &mut self.states[i];
            st.mw = Some(layout.w);
            st.mh = Some(layout.h);
            // obrys elementu tekstu: ściśnięcie całego elementu (`scaleX`) wokół osi wg wyrównania, jak w renderer.js
            let sx0 = if layout.per_line { 1.0 } else { layout.lines.first().map_or(1.0, |l| l.sx) };
            let l0 = match align.as_str() {
                "right" => layout.span_w * (1.0 - sx0),
                "center" => layout.span_w * (1.0 - sx0) / 2.0,
                _ => 0.0,
            };
            let span = [l0, 0.0, l0 + layout.span_w * sx0, layout.span_h];
            st.block = Some(Arc::new(TextBlock { key: hash_str(&[&key]), lines, span }));
            if std::env::var_os("VIZRS_RENDER_LAYOUT").is_some() {
                // diagnostyka zgodności z Chromium: rozmiar fontu, ściśnięcie i wymiary elementu tekstu
                let lines: Vec<String> =
                    layout.lines.iter().map(|l| format!("{:.4}px sx {:.6} w {:.0}", l.run.size, l.sx, l.run.offset_width())).collect();
                eprintln!("layout {}: span {}x{} {:?}", self.nodes[i].label(), layout.span_w, layout.span_h, lines);
            }
            st.layout = Some(Arc::new(layout));
            st.key = key;
        }
        let Some(layout) = self.states[i].layout.clone() else { return };
        let Some(block) = self.states[i].block.clone() else { return };
        // flex: wyrównanie elementu tekstu w polu (z marginesem X), w pionie wg `valign`
        let mut x = span_x(&layout, pad_x, w, &align);
        let mut y = match valign.as_str() {
            "top" => 0.0,
            "bottom" => h - layout.span_h,
            _ => (h - layout.span_h) / 2.0,
        };
        if !layout.per_line && layout.lines.first().is_some_and(|l| l.sx < 0.999) {
            // ściśnięty element tekstu (`scaleX`): położenie do pełnego piksela, jak w Blink
            x = x.round();
            y = y.round();
        }
        let Some(col) = color(&self.txt(i, "color")) else { return };
        let sh = self.num(i, "shadow");
        let shadow = (sh > 0.0).then(|| ((sh / 4.0).max(1.0), sh / 2.0, [0.0, 0.0, 0.0, 0.65]));
        self.states[i].content = vec![Paint::Text { block, color: col, shadow, x, y }];
    }
}
