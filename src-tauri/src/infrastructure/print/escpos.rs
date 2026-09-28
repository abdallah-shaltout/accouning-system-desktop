//! ESC/POS command encoding (docs/v2/12-documents-pdf-excel.md §5).
//!
//! Reference: the raster bit-image command is `GS v 0` (hex `1D 76 30`),
//! documented across ESC/POS-compatible printer manuals (Epson TM-T series,
//! Xprinter, and the many clones) as:
//!
//! ```text
//! GS v 0 m xL xH yL yH d1...dk
//! 1D  76 30 m  xL xH yL yH  <raster data>
//! ```
//! - `m`: mode byte. `0` = normal. (1/2/3 are double-width/height/both —
//!   unused here, receipts print at native resolution.)
//! - `xL xH`: width **in bytes** (i.e. `bytes_per_row`), little-endian 16-bit:
//!   `xL = bytes_per_row & 0xFF`, `xH = (bytes_per_row >> 8) & 0xFF`.
//! - `yL yH`: height **in dots** (pixel rows), little-endian 16-bit.
//! - `d1..dk`: `bytes_per_row * height` raster bytes, MSB-first, 1 = print
//!   dot, top-to-bottom / left-to-right (this matches `dither`'s bit packing
//!   exactly, so no repacking is needed between the two modules).
//!
//! Paper cut is `GS V` (`1D 56`); the common "full cut, no feed" variant is
//! `GS V 0` / `GS V 65 0` (partial cut) — this module emits `GS V 66 0`
//! (`1D 56 42 00`), the widely-supported "feed to cutting position then
//! partial cut" form used by Epson TM-T88 and most clones, matching what the
//! v1 HTML thermal route's printers are assumed to be (same class of hardware
//! the settings page's printer list will show).
//!
//! Cash-drawer kick is `ESC p m t1 t2` (`1B 70 00 19 FA`): pin 2, ~25ms on,
//! ~250ms off pulse — the standard values every ESC/POS drawer-kick cable
//! expects.

use super::dither::Bitmap1Bit;

const GS: u8 = 0x1D;
const ESC: u8 = 0x1B;

/// Initializes the printer (`ESC @`) — resets any leftover state from a
/// previous job so the raster image starts from a clean line/alignment.
pub fn init() -> Vec<u8> {
    vec![ESC, b'@']
}

/// Encodes one rasterized, dithered bitmap as a `GS v 0` raster command.
/// `bytes_per_row` and `height` are taken from the bitmap itself, so the
/// header always matches the actual data length — this is the piece most
/// prone to off-by-one printer-garbling bugs if done by hand, so it's
/// asserted in the smoke test (`src-tauri/src/bin/thermal_smoke.rs`).
pub fn raster_image(bitmap: &Bitmap1Bit) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + bitmap.bits.len());
    out.push(GS);
    out.push(b'v');
    out.push(b'0');
    out.push(0x00); // m = normal mode
    let x = bitmap.bytes_per_row;
    let y = bitmap.height;
    out.push((x & 0xFF) as u8);
    out.push(((x >> 8) & 0xFF) as u8);
    out.push((y & 0xFF) as u8);
    out.push(((y >> 8) & 0xFF) as u8);
    out.extend_from_slice(&bitmap.bits);
    out
}

/// Feeds a few lines then cuts (`GS V 66 0`): feed-to-cut-position + partial
/// cut, per the module doc comment.
pub fn cut() -> Vec<u8> {
    vec![GS, b'V', 66, 0]
}

/// Cash-drawer kick (`ESC p 0 25 250`), pin 2, standard timing.
pub fn open_drawer() -> Vec<u8> {
    vec![ESC, b'p', 0x00, 25, 250]
}

/// A few blank feed lines (`ESC d n`) so the cut doesn't slice through the
/// last printed line — used before `cut()`.
pub fn feed(lines: u8) -> Vec<u8> {
    vec![ESC, b'd', lines]
}

/// Builds the full byte stream for one receipt: init → raster image → feed →
/// (cut) → (drawer kick). `copies` repeats the raster+feed+cut portion (not
/// `init`/drawer, which only need to run once per job).
pub fn build_receipt_job(bitmap: &Bitmap1Bit, cut_paper: bool, kick_drawer: bool, copies: u32) -> Vec<u8> {
    let mut out = init();
    let copies = copies.max(1);
    for _ in 0..copies {
        out.extend(raster_image(bitmap));
        out.extend(feed(3));
        if cut_paper {
            out.extend(cut());
        }
    }
    if kick_drawer {
        out.extend(open_drawer());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bitmap() -> Bitmap1Bit {
        // 24 px wide (3 bytes/row) x 4 px tall, alternating dot pattern.
        Bitmap1Bit { width: 24, height: 4, bytes_per_row: 3, bits: vec![0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55] }
    }

    #[test]
    fn raster_header_matches_bitmap_dimensions() {
        let bmp = sample_bitmap();
        let cmd = raster_image(&bmp);
        assert_eq!(&cmd[0..4], &[GS, b'v', b'0', 0x00]);
        let x = u16::from_le_bytes([cmd[4], cmd[5]]);
        let y = u16::from_le_bytes([cmd[6], cmd[7]]);
        assert_eq!(x as u32, bmp.bytes_per_row);
        assert_eq!(y as u32, bmp.height);
        assert_eq!(cmd.len(), 8 + bmp.bits.len());
        assert_eq!(&cmd[8..], &bmp.bits[..]);
    }

    #[test]
    fn cut_command_is_gs_v() {
        assert_eq!(cut(), vec![GS, b'V', 66, 0]);
    }

    #[test]
    fn drawer_kick_is_esc_p() {
        assert_eq!(open_drawer(), vec![ESC, b'p', 0x00, 25, 250]);
    }

    #[test]
    fn job_repeats_raster_per_copy_but_not_init_or_drawer() {
        let bmp = sample_bitmap();
        let job = build_receipt_job(&bmp, true, true, 2);
        // init (2 bytes) + 2x(raster header(8) + data(12) + feed(3) + cut(4)) + drawer(5)
        let per_copy = 8 + bmp.bits.len() + 3 + 4;
        assert_eq!(job.len(), 2 + per_copy * 2 + 5);
        assert_eq!(&job[job.len() - 5..], &open_drawer()[..]);
    }
}
