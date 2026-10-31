//! Rozmycie gaussowskie przybliżone trzema rozmyciami pudełkowymi (jak SVG/CSS `feGaussianBlur` w Skia):
//! `filter: blur(σ)`, cienie pól (`box-shadow`, σ = promień / 2) i tekstu (`text-shadow`).

/// Długość pudełka dla σ (Filter Effects §feGaussianBlur: d = ⌊σ · 3√(2π) / 4 + 0,5⌋).
fn box_size(sigma: f32) -> usize {
    ((sigma * 3.0 * (2.0 * std::f32::consts::PI).sqrt() / 4.0) + 0.5).floor().max(1.0) as usize
}

/// Jedno przejście pudełkowe wzdłuż linii (`n` wartości co `step`), okno `d` z przesunięciem `lead`.
fn box_line(src: &[u32], dst: &mut [u32], d: usize, lead: usize) {
    let n = src.len();
    let mut sum: u64 = 0;
    // okno [i - lead, i - lead + d)
    let get = |i: isize| -> u64 {
        if i < 0 || i as usize >= n {
            0
        } else {
            u64::from(src[i as usize])
        }
    };
    for k in 0..d as isize {
        sum += get(k - lead as isize);
    }
    for (i, out) in dst.iter_mut().enumerate() {
        *out = ((sum + d as u64 / 2) / d as u64) as u32;
        let i = i as isize;
        sum += get(i + d as isize - lead as isize);
        sum -= get(i - lead as isize);
    }
}

/// Trzy przejścia pudełkowe (σ ≈ gauss) jednej linii.
fn blur_line(buf: &mut Vec<u32>, tmp: &mut Vec<u32>, d: usize) {
    if d % 2 == 1 {
        let r = d / 2;
        for _ in 0..3 {
            box_line(buf, tmp, d, r);
            std::mem::swap(buf, tmp);
        }
    } else {
        // parzyste d: dwa pudełka przesunięte w przeciwne strony i jedno d+1 wyśrodkowane
        box_line(buf, tmp, d, d / 2);
        std::mem::swap(buf, tmp);
        box_line(buf, tmp, d, d / 2 - 1);
        std::mem::swap(buf, tmp);
        box_line(buf, tmp, d + 1, d / 2);
        std::mem::swap(buf, tmp);
    }
}

/// Rozmywa obraz `w`×`h` o `ch` kanałach (1 – alfa, 4 – RGBA z przemnożoną alfą).
pub fn gaussian(data: &mut [u8], w: usize, h: usize, ch: usize, sigma: f32) {
    if sigma < 0.3 || w == 0 || h == 0 {
        return;
    }
    let d = box_size(sigma);
    let mut buf = Vec::new();
    let mut tmp = Vec::new();
    for c in 0..ch {
        // wiersze
        for y in 0..h {
            buf.clear();
            buf.extend((0..w).map(|x| u32::from(data[(y * w + x) * ch + c])));
            tmp.resize(w, 0);
            blur_line(&mut buf, &mut tmp, d);
            for x in 0..w {
                data[(y * w + x) * ch + c] = buf[x].min(255) as u8;
            }
        }
        // kolumny
        for x in 0..w {
            buf.clear();
            buf.extend((0..h).map(|y| u32::from(data[(y * w + x) * ch + c])));
            tmp.resize(h, 0);
            blur_line(&mut buf, &mut tmp, d);
            for y in 0..h {
                data[(y * w + x) * ch + c] = buf[y].min(255) as u8;
            }
        }
    }
    if ch == 4 {
        // po zaokrągleniach kolor nie może przekroczyć alfy (przemnożenie)
        for p in data.chunks_exact_mut(4) {
            for k in 0..3 {
                p[k] = p[k].min(p[3]);
            }
        }
    }
}

/// Margines, który rozmycie σ dokłada wokół kształtu (px).
pub fn extent(sigma: f32) -> f32 {
    if sigma < 0.3 {
        0.0
    } else {
        (box_size(sigma) as f32 * 1.5).ceil() + 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_keeps_mass_and_spreads() {
        let (w, h) = (21, 21);
        let mut a = vec![0u8; w * h];
        for y in 8..13 {
            for x in 8..13 {
                a[y * w + x] = 255;
            }
        }
        let before: u32 = a.iter().map(|&v| u32::from(v)).sum();
        gaussian(&mut a, w, h, 1, 2.0);
        let after: u32 = a.iter().map(|&v| u32::from(v)).sum();
        assert!((before as i64 - after as i64).abs() < before as i64 / 50, "{before} {after}");
        assert!(a[10 * w + 5] > 0 && a[10 * w + 10] < 255);
        // symetria
        assert_eq!(a[10 * w + 6], a[10 * w + 14]);
    }

    #[test]
    fn tiny_sigma_does_nothing() {
        let mut a = vec![0, 255, 0];
        gaussian(&mut a, 3, 1, 1, 0.1);
        assert_eq!(a, vec![0, 255, 0]);
        assert_eq!(extent(0.0), 0.0);
    }
}
