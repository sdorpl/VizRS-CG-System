// Pomiar szerokości tekstu dla tools/native-widths.js: stdin JSON [[font, weight, italic, size, spacing, text], …] → szerokości.
use std::io::Read;
fn main() {
    let mut db = vizrs_render::font::FontDb::new();
    db.add_dir(std::path::Path::new("../web/fonts"));
    vizrs_render::add_system_fonts(&mut db);
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let cases: Vec<serde_json::Value> = serde_json::from_str(&s).unwrap();
    let out: Vec<f32> = cases
        .iter()
        .map(|c| {
            let st = vizrs_render::text::Style {
                font: c[0].as_str().unwrap().into(),
                weight: c[1].as_u64().unwrap() as u16,
                italic: c[2].as_bool().unwrap(),
                size: c[3].as_f64().unwrap() as f32,
                spacing: c[4].as_f64().unwrap() as f32,
                tnum: false,
                hl: None,
            };
            vizrs_render::text::shape(&db, &st, c[5].as_str().unwrap()).width
        })
        .collect();
    println!("{}", serde_json::to_string(&out).unwrap());
}
