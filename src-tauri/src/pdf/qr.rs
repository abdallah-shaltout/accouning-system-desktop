//! Placeholder QR code generation for the Typst PDF spike.
//!
//! The spike only needs *an* embedded QR image on the page — it doesn't need
//! to encode a real ZATCA TLV payload (that's `zatcaQr.ts` + `uqr` in the TS
//! layer per docs/v2/12-documents-pdf-excel.md §2). This renders a real QR
//! (not a static placeholder image) from whatever string is given, as PNG
//! bytes, using the `qrcode` + `image` crates.

use image::{ImageBuffer, ImageEncoder, Luma};
use qrcode::QrCode;

pub fn placeholder_qr_png(data: &str) -> Result<Vec<u8>, String> {
    let code = QrCode::new(data.as_bytes()).map_err(|e| format!("qr encode failed: {e}"))?;

    let scale = 6u32; // pixels per module
    let quiet_zone = 2u32; // modules of white border
    let width = code.width() as u32;
    let size = width + quiet_zone * 2;

    let mut img = ImageBuffer::from_pixel(size * scale, size * scale, Luma([255u8]));

    for y in 0..width {
        for x in 0..width {
            let dark = code[(x as usize, y as usize)] == qrcode::Color::Dark;
            if dark {
                let px = (x + quiet_zone) * scale;
                let py = (y + quiet_zone) * scale;
                for dy in 0..scale {
                    for dx in 0..scale {
                        img.put_pixel(px + dx, py + dy, Luma([0u8]));
                    }
                }
            }
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::L8,
        )
        .map_err(|e| format!("qr png encode failed: {e}"))?;

    Ok(bytes)
}
