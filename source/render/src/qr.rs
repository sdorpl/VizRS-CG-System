//! Kod QR – odpowiednik `web/qr.js` (tryb bajtowy, poziom korekcji M, wersje 1–10, wybór maski wg kar),
//! więc moduły są te same co w Chromium.

const ECC_M: [(usize, &[(usize, usize)]); 10] = [
    (10, &[(1, 16)]),
    (16, &[(1, 28)]),
    (26, &[(1, 44)]),
    (18, &[(2, 32)]),
    (24, &[(2, 43)]),
    (16, &[(4, 27)]),
    (18, &[(4, 31)]),
    (22, &[(2, 38), (2, 39)]),
    (22, &[(3, 36), (2, 37)]),
    (26, &[(4, 43), (1, 44)]),
];
const ALIGN: [&[usize]; 10] =
    [&[], &[6, 18], &[6, 22], &[6, 26], &[6, 30], &[6, 34], &[6, 22, 38], &[6, 24, 42], &[6, 26, 46], &[6, 28, 50]];

fn gf_mul(x: u8, y: u8) -> u8 {
    let mut z: u32 = 0;
    for i in (0..8).rev() {
        z = (z << 1) ^ ((z >> 7) * 0x11d);
        z ^= ((u32::from(y) >> i) & 1) * u32::from(x);
    }
    (z & 0xff) as u8
}

fn rs_divisor(degree: usize) -> Vec<u8> {
    let mut r = vec![0u8; degree];
    r[degree - 1] = 1;
    let mut root = 1u8;
    for _ in 0..degree {
        for j in 0..degree {
            r[j] = gf_mul(r[j], root);
            if j + 1 < degree {
                r[j] ^= r[j + 1];
            }
        }
        root = gf_mul(root, 2);
    }
    r
}

fn rs_remainder(data: &[u8], div: &[u8]) -> Vec<u8> {
    let mut r = vec![0u8; div.len()];
    for &b in data {
        let f = b ^ r.remove(0);
        r.push(0);
        for (i, d) in div.iter().enumerate() {
            r[i] ^= gf_mul(*d, f);
        }
    }
    r
}

/// Macierz modułów (true = ciemny); `None` – tekst za długi.
pub fn matrix(text: &str) -> Option<Vec<Vec<bool>>> {
    let bytes = text.as_bytes();
    let ver = (1..=10).find(|&v| {
        let cap: usize = ECC_M[v - 1].1.iter().map(|(n, d)| n * d).sum::<usize>() * 8;
        4 + if v < 10 { 8 } else { 16 } + bytes.len() * 8 <= cap
    })?;
    let (ec_len, groups) = ECC_M[ver - 1];
    let data_cw: usize = groups.iter().map(|(n, d)| n * d).sum();
    let mut bits: Vec<u8> = Vec::new();
    let push = |bits: &mut Vec<u8>, val: usize, len: usize| {
        for i in (0..len).rev() {
            bits.push(((val >> i) & 1) as u8);
        }
    };
    push(&mut bits, 4, 4);
    push(&mut bits, bytes.len(), if ver < 10 { 8 } else { 16 });
    for &b in bytes {
        push(&mut bits, usize::from(b), 8);
    }
    let term = (data_cw * 8 - bits.len()).min(4);
    push(&mut bits, 0, term);
    while !bits.len().is_multiple_of(8) {
        bits.push(0);
    }
    let mut cw: Vec<u8> = bits.chunks(8).map(|c| c.iter().fold(0u8, |a, &b| (a << 1) | b)).collect();
    let mut pad = 0xecu8;
    while cw.len() < data_cw {
        cw.push(pad);
        pad ^= 0xec ^ 0x11;
    }
    let div = rs_divisor(ec_len);
    let mut blocks: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut k = 0;
    for &(n, d) in groups {
        for _ in 0..n {
            let dat = cw[k..k + d].to_vec();
            k += d;
            let ec = rs_remainder(&dat, &div);
            blocks.push((dat, ec));
        }
    }
    let mut fin = Vec::new();
    let max_d = blocks.iter().map(|b| b.0.len()).max().unwrap_or(0);
    for i in 0..max_d {
        for b in &blocks {
            if i < b.0.len() {
                fin.push(b.0[i]);
            }
        }
    }
    for i in 0..ec_len {
        for b in &blocks {
            fin.push(b.1[i]);
        }
    }

    let size = ver * 4 + 17;
    let mut m = vec![vec![false; size]; size];
    let mut f = vec![vec![false; size]; size];
    let set = |m: &mut Vec<Vec<bool>>, f: &mut Vec<Vec<bool>>, x: usize, y: usize, v: bool| {
        m[y][x] = v;
        f[y][x] = true;
    };
    for i in 0..size {
        set(&mut m, &mut f, 6, i, i % 2 == 0);
        set(&mut m, &mut f, i, 6, i % 2 == 0);
    }
    for (cx, cy) in [(3i32, 3i32), (size as i32 - 4, 3), (3, size as i32 - 4)] {
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
                    continue;
                }
                let d = dx.abs().max(dy.abs());
                set(&mut m, &mut f, x as usize, y as usize, d != 2 && d != 4);
            }
        }
    }
    let al = ALIGN[ver - 1];
    let na = al.len();
    for i in 0..na {
        for j in 0..na {
            if (i == 0 && j == 0) || (i == 0 && j == na - 1) || (i == na - 1 && j == 0) {
                continue;
            }
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let (x, y) = ((al[i] as i32 + dx) as usize, (al[j] as i32 + dy) as usize);
                    set(&mut m, &mut f, x, y, dx.abs().max(dy.abs()) != 1);
                }
            }
        }
    }
    let bit = |x: u32, i: u32| (x >> i) & 1 != 0;
    let draw_format = |m: &mut Vec<Vec<bool>>, f: &mut Vec<Vec<bool>>, mask: u32| {
        let data = mask; // poziom M = 00
        let mut rem = data;
        for _ in 0..10 {
            rem = (rem << 1) ^ ((rem >> 9) * 0x537);
        }
        let b = ((data << 10) | rem) ^ 0x5412;
        for i in 0..=5 {
            set(m, f, 8, i, bit(b, i as u32));
        }
        set(m, f, 8, 7, bit(b, 6));
        set(m, f, 8, 8, bit(b, 7));
        set(m, f, 7, 8, bit(b, 8));
        for i in 9..15 {
            set(m, f, 14 - i, 8, bit(b, i as u32));
        }
        for i in 0..8 {
            set(m, f, size - 1 - i, 8, bit(b, i as u32));
        }
        for i in 8..15 {
            set(m, f, 8, size - 15 + i, bit(b, i as u32));
        }
        set(m, f, 8, size - 8, true);
    };
    draw_format(&mut m, &mut f, 0);
    if ver >= 7 {
        let mut rem = ver as u32;
        for _ in 0..12 {
            rem = (rem << 1) ^ ((rem >> 11) * 0x1f25);
        }
        let b = ((ver as u32) << 12) | rem;
        for i in 0..18 {
            let (a, c) = (size - 11 + (i % 3), i / 3);
            set(&mut m, &mut f, a, c, bit(b, i as u32));
            set(&mut m, &mut f, c, a, bit(b, i as u32));
        }
    }
    // dane (zygzak)
    let mut bi = 0usize;
    let mut right = size as i32 - 1;
    while right >= 1 {
        if right == 6 {
            right = 5;
        }
        for vert in 0..size {
            for j in 0..2 {
                let x = (right - j) as usize;
                let up = ((right + 1) & 2) == 0;
                let y = if up { size - 1 - vert } else { vert };
                if !f[y][x] && bi < fin.len() * 8 {
                    m[y][x] = (fin[bi >> 3] >> (7 - (bi & 7))) & 1 != 0;
                    bi += 1;
                }
            }
        }
        right -= 2;
    }
    let masks: [fn(usize, usize) -> bool; 8] = [
        |x, y| (x + y) % 2 == 0,
        |_, y| y % 2 == 0,
        |x, _| x % 3 == 0,
        |x, y| (x + y) % 3 == 0,
        |x, y| (x / 3 + y / 2) % 2 == 0,
        |x, y| (x * y) % 2 + (x * y) % 3 == 0,
        |x, y| ((x * y) % 2 + (x * y) % 3) % 2 == 0,
        |x, y| ((x + y) % 2 + (x * y) % 3) % 2 == 0,
    ];
    let apply = |m: &mut Vec<Vec<bool>>, k: usize| {
        for y in 0..size {
            for x in 0..size {
                if !f[y][x] && masks[k](x, y) {
                    m[y][x] = !m[y][x];
                }
            }
        }
    };
    let penalty = |m: &Vec<Vec<bool>>| -> i64 {
        let mut p = 0i64;
        let mut dark = 0i64;
        for y in 0..size {
            let (mut run, mut run_c) = (1, 1);
            for x in 0..size {
                if m[y][x] {
                    dark += 1;
                }
                if x > 0 {
                    if m[y][x] == m[y][x - 1] {
                        run += 1;
                        if run == 5 {
                            p += 3;
                        } else if run > 5 {
                            p += 1;
                        }
                    } else {
                        run = 1;
                    }
                    if m[x][y] == m[x - 1][y] {
                        run_c += 1;
                        if run_c == 5 {
                            p += 3;
                        } else if run_c > 5 {
                            p += 1;
                        }
                    } else {
                        run_c = 1;
                    }
                }
                if x < size - 1 && y < size - 1 {
                    let c = m[y][x];
                    if c == m[y][x + 1] && c == m[y + 1][x] && c == m[y + 1][x + 1] {
                        p += 3;
                    }
                }
            }
        }
        let n = (size * size) as i64;
        p + ((dark * 20 - n * 10).abs() / n) * 10
    };
    let (mut best, mut best_p) = (0, i64::MAX);
    let mut fm = f.clone();
    for k in 0..8 {
        apply(&mut m, k);
        draw_format(&mut m, &mut fm, k as u32);
        let pv = penalty(&m);
        if pv < best_p {
            best_p = pv;
            best = k;
        }
        apply(&mut m, k);
    }
    apply(&mut m, best);
    draw_format(&mut m, &mut fm, best as u32);
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_finders() {
        let m = matrix("https://info24.example/wesprzyj").unwrap();
        assert_eq!(m.len(), 29); // wersja 3
                                 // wzorce wyszukiwania w trzech rogach
        assert!(m[0][0] && m[0][6] && m[6][0] && !m[1][1] && m[3][3]);
        assert!(m[0][28] && m[28][0]);
        assert!(matrix(&"x".repeat(300)).is_none());
        assert_eq!(matrix("").unwrap().len(), 21);
    }
}
