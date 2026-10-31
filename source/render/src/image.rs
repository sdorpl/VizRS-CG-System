//! Obrazy z assets: PNG i JPEG → `Pixmap` (alfa przemnożona, jak w Skia). Inne formaty (WebP, GIF, SVG, wideo)
//! renderer natywny zgłasza jako nieobsługiwane (strona wraca do Chromium).

use tiny_skia::{IntSize, Pixmap};

pub fn decode(data: &[u8]) -> Result<Pixmap, String> {
    if data.starts_with(b"\x89PNG") {
        return decode_png(data);
    }
    if data.starts_with(&[0xff, 0xd8]) {
        return decode_jpeg(data);
    }
    Err("nieobsługiwany format obrazu".into())
}

/// Czy renderer natywny umie narysować plik o tej nazwie.
pub fn supported_name(src: &str) -> bool {
    let l = src.to_ascii_lowercase();
    l.ends_with(".png") || l.ends_with(".jpg") || l.ends_with(".jpeg")
}

fn premultiplied(mut rgba: Vec<u8>, w: u32, h: u32) -> Result<Pixmap, String> {
    for p in rgba.chunks_exact_mut(4) {
        let a = u16::from(p[3]);
        if a < 255 {
            for c in &mut p[..3] {
                *c = ((u16::from(*c) * a + 127) / 255) as u8;
            }
        }
    }
    Pixmap::from_vec(rgba, IntSize::from_wh(w, h).ok_or("pusty obraz")?).ok_or_else(|| "zły rozmiar obrazu".into())
}

fn decode_png(data: &[u8]) -> Result<Pixmap, String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(data));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| format!("PNG: {e}"))?;
    let size = reader.output_buffer_size().ok_or("PNG: za duży")?;
    if size > 256 << 20 {
        return Err("PNG: za duży".into());
    }
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("PNG: {e}"))?;
    buf.truncate(info.buffer_size());
    let (w, h) = (info.width, info.height);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("PNG: paleta bez rozwinięcia".into()),
    };
    premultiplied(rgba, w, h)
}

fn decode_jpeg(data: &[u8]) -> Result<Pixmap, String> {
    let mut dec = jpeg_decoder::Decoder::new(std::io::Cursor::new(data));
    let px = dec.decode().map_err(|e| format!("JPEG: {e}"))?;
    let info = dec.info().ok_or("JPEG: brak nagłówka")?;
    let (w, h) = (u32::from(info.width), u32::from(info.height));
    let rgba: Vec<u8> = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => px.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        jpeg_decoder::PixelFormat::L8 => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        jpeg_decoder::PixelFormat::L16 => px.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], 255]).collect(),
        jpeg_decoder::PixelFormat::CMYK32 => px
            .chunks_exact(4)
            .flat_map(|p| {
                let k = 255 - u16::from(p[3]);
                let ch = |c: u8| ((255 - u16::from(c)) * k / 255) as u8;
                [ch(p[0]), ch(p[1]), ch(p[2]), 255]
            })
            .collect(),
    };
    premultiplied(rgba, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trip_premultiplies() {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, 1, 1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&[255, 0, 0, 128]).unwrap();
        }
        let p = decode(&out).unwrap();
        assert_eq!(&p.data()[..4], &[128, 0, 0, 128]);
        assert!(decode(b"GIF89a").is_err());
        assert!(supported_name("Logo.PNG") && !supported_name("a.webp"));
    }
}
