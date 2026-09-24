/**
 * Barcode SVG generation for the label builder (docs/v2/07-products-and-inventory.md §6,
 * docs/v2/12-documents-pdf-excel.md §4 "Labels"). `bwip-js` is the barcode library named in
 * README.md's library list and docs/v2/README.md decision 11 ("bwip-js for barcodes ... lazy
 * loaded where it's heavy") — this module is the one place that imports it, dynamically, so pages
 * that never open the label builder never pay for it.
 *
 * Typst has no built-in barcode package and this codebase renders "no Typst packages, everything
 * must work offline" (docs/v2/12 §2), so barcodes are generated as SVG markup here in TS and
 * embedded in the `DocumentPayload` exactly like the QR code already is (`data.qr`/`qrSvg()` in
 * pdfService.ts) — the Typst label templates just `image(bytes(item.barcodeSvg), format: "svg")`.
 */

export type BarcodeSymbology = 'ean13' | 'code128';

/**
 * Renders one barcode as inline SVG markup. Returns `null` (rather than throwing) for an empty/
 * invalid value so a product missing a barcode just skips the barcode line on its label instead of
 * failing the whole batch.
 */
export async function barcodeSvg(value: string, symbology: BarcodeSymbology = 'code128'): Promise<string | null> {
  const clean = value.trim();
  if (!clean) return null;
  try {
    // Explicit `/browser` subpath: `bwip-js`'s package.json only exposes a `types` condition
    // under its `browser`/`electron`/`node` export conditions, not at the bare `"."` root, and
    // this Vite/tsc setup (moduleResolution "bundler", no `customConditions`) doesn't pick
    // `browser` automatically — importing the subpath directly resolves both the runtime code
    // (browser-safe, no Node Buffer/fs) and its .d.ts unambiguously.
    const bwipjs = await import('bwip-js/browser');
    return bwipjs.toSVG({
      bcid: symbology === 'ean13' ? 'ean13' : 'code128',
      text: clean,
      includetext: false,
      scale: 2,
      height: 10,
    });
  } catch {
    // Invalid EAN-13 checksum, non-numeric text for ean13, etc. — same "skip, don't fail the
    // batch" behavior as an empty value.
    return null;
  }
}

/** True-ish EAN-13 shape check (13 digits) — used to pick the symbology automatically per product. */
export function looksLikeEan13(barcode: string | undefined): boolean {
  return !!barcode && /^\d{13}$/.test(barcode.trim());
}
