//! Minimal 24-bit BMP writer for F5 (CPU-Z saves tabs as .bmp).

use eframe::egui::ColorImage;

pub fn encode(img: &ColorImage) -> Vec<u8> {
    let [w, h] = img.size;
    let row = (w * 3).div_ceil(4) * 4;
    let data_len = row * h;
    let mut out = Vec::with_capacity(54 + data_len);
    let le32 = |v: usize| (v as u32).to_le_bytes();
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&le32(54 + data_len));
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&le32(54));
    out.extend_from_slice(&le32(40)); // BITMAPINFOHEADER
    out.extend_from_slice(&le32(w));
    out.extend_from_slice(&le32(h)); // positive = bottom-up rows
    out.extend_from_slice(&[1, 0, 24, 0]);
    out.extend_from_slice(&[0; 4]); // BI_RGB
    out.extend_from_slice(&le32(data_len));
    out.extend_from_slice(&le32(2835)); // 72 dpi
    out.extend_from_slice(&le32(2835));
    out.extend_from_slice(&[0; 8]);
    for y in (0..h).rev() {
        let start = out.len();
        for p in &img.pixels[y * w..(y + 1) * w] {
            out.extend_from_slice(&[p.b(), p.g(), p.r()]);
        }
        out.resize(start + row, 0);
    }
    out
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Color32, ColorImage};
    #[test]
    fn header_and_padding() {
        let img = ColorImage::new([3, 2], vec![Color32::RED; 6]);
        let b = super::encode(&img);
        assert_eq!(&b[..2], b"BM");
        assert_eq!(b.len(), 54 + 12 * 2); // 9 bytes/row padded to 12
        assert_eq!(&b[54..57], &[0, 0, 255]);
    }
}
