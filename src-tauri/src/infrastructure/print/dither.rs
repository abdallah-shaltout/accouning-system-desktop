//! 1-bit Floyd–Steinberg dithering (docs/v2/12-documents-pdf-excel.md §5:
//! "1-bit dithering (Floyd-Steinberg or ordered dither, your call)").
//!
//! Floyd–Steinberg was chosen over ordered (Bayer) dithering because receipts
//! carry text and thin rule lines almost exclusively — error diffusion keeps
//! edges crisper than a fixed threshold matrix, which tends to fray thin
//! strokes into a visible dot pattern.

/// A 1-bit bitmap: `width`/`height` in pixels, `bits` packed MSB-first, one
/// bit per pixel, rows padded to a whole byte — the exact layout ESC/POS
/// `GS v 0` expects (`bytes_per_row = ceil(width / 8)`).
pub struct Bitmap1Bit {
    pub width: u32,
    pub height: u32,
    pub bytes_per_row: u32,
    pub bits: Vec<u8>,
}

/// Converts RGBA8 pixels to grayscale luminance (ITU-R BT.601), then applies
/// Floyd–Steinberg error diffusion and packs the result into a 1-bit-per-pixel
/// bitmap (0 = white/no dot, 1 = black/print dot — thermal printers burn dots
/// for "on" pixels, matching ESC/POS raster's bit convention).
pub fn dither_floyd_steinberg(rgba: &[u8], width: u32, height: u32) -> Bitmap1Bit {
    let w = width as usize;
    let h = height as usize;
    let mut gray: Vec<f32> = Vec::with_capacity(w * h);
    for px in rgba.chunks_exact(4) {
        let [r, g, b, a] = [px[0] as f32, px[1] as f32, px[2] as f32, px[3] as f32];
        // Composite over white using alpha, then take luminance — the raster
        // step always renders onto an opaque white pixmap (see
        // `pdf::raster::render_svg_to_rgba`), but this stays correct even if
        // a future caller passes a transparent source.
        let alpha = a / 255.0;
        let lum = (0.299 * r + 0.587 * g + 0.114 * b) * alpha + 255.0 * (1.0 - alpha);
        gray.push(lum);
    }

    let bytes_per_row = ((width + 7) / 8) as usize;
    let mut bits = vec![0u8; bytes_per_row * h];

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let old = gray[idx].clamp(0.0, 255.0);
            // Threshold at mid-gray: below → black dot (prints), above → white.
            let new = if old < 128.0 { 0.0 } else { 255.0 };
            let err = old - new;

            if new == 0.0 {
                let byte_idx = y * bytes_per_row + x / 8;
                let bit = 7 - (x % 8);
                bits[byte_idx] |= 1 << bit;
            }

            // Distribute quantization error to neighbors (classic F–S kernel:
            // right 7/16, below-left 3/16, below 5/16, below-right 1/16).
            if x + 1 < w {
                gray[idx + 1] += err * 7.0 / 16.0;
            }
            if y + 1 < h {
                if x > 0 {
                    gray[idx + w - 1] += err * 3.0 / 16.0;
                }
                gray[idx + w] += err * 5.0 / 16.0;
                if x + 1 < w {
                    gray[idx + w + 1] += err * 1.0 / 16.0;
                }
            }
        }
    }

    Bitmap1Bit { width, height, bytes_per_row: bytes_per_row as u32, bits }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dithers_solid_white_to_all_zero_bits() {
        let w = 16u32;
        let h = 8u32;
        let rgba = vec![255u8; (w * h * 4) as usize];
        let bmp = dither_floyd_steinberg(&rgba, w, h);
        assert_eq!(bmp.bytes_per_row, 2);
        assert!(bmp.bits.iter().all(|&b| b == 0), "pure white should dither to no dots");
    }

    #[test]
    fn dithers_solid_black_to_all_one_bits() {
        let w = 16u32;
        let h = 8u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for px in rgba.chunks_exact_mut(4) {
            px[3] = 255;
        }
        let bmp = dither_floyd_steinberg(&rgba, w, h);
        assert!(bmp.bits.iter().all(|&b| b == 0xFF), "pure black should dither to all dots");
    }

    #[test]
    fn row_padding_matches_ceil_div_8() {
        let bmp = dither_floyd_steinberg(&vec![255u8; 9 * 3 * 4], 9, 3);
        assert_eq!(bmp.bytes_per_row, 2); // ceil(9/8) = 2
        assert_eq!(bmp.bits.len(), 2 * 3);
    }
}
