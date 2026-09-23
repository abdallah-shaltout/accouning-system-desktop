/**
 * ZATCA Phase-1 (simplified tax invoice) QR payload: five TLV fields, base64-encoded.
 * Ported from references/vat-invoice-app/js/zatca-qr.js.
 *
 *   1 seller name · 2 VAT registration number · 3 timestamp (ISO 8601)
 *   4 invoice total incl. VAT · 5 VAT total
 *
 * This is the cosmetic Phase-1 QR only — no Phase-2 XML/UBL, signing, or reporting.
 */
export interface ZatcaQrInput {
  sellerName: string;
  vatNumber: string;
  timestamp: string | Date;
  invoiceTotal: number | string;
  vatTotal: number | string;
}

const encoder = new TextEncoder();

function tlv(tag: number, value: string): Uint8Array {
  const bytes = encoder.encode(value);
  if (bytes.length > 255) console.warn(`ZATCA TLV value too long for tag ${tag}`);
  return Uint8Array.from([tag, bytes.length, ...bytes]);
}

function money(v: number | string): string {
  const n = Number(v);
  return Number.isFinite(n) ? n.toFixed(2) : '0.00';
}

function timestamp(ts: string | Date): string {
  const d = typeof ts === 'string' ? new Date(ts) : ts;
  return Number.isNaN(d.getTime()) ? new Date().toISOString() : d.toISOString();
}

export function buildZatcaTlv(input: ZatcaQrInput): Uint8Array {
  const parts = [
    tlv(1, input.sellerName.trim()),
    tlv(2, input.vatNumber ?? ''),
    tlv(3, timestamp(input.timestamp)),
    tlv(4, money(input.invoiceTotal)),
    tlv(5, money(input.vatTotal)),
  ];
  const out = new Uint8Array(parts.reduce((s, p) => s + p.length, 0));
  let offset = 0;
  for (const p of parts) {
    out.set(p, offset);
    offset += p.length;
  }
  return out;
}

export function zatcaQrBase64(input: ZatcaQrInput): string {
  let binary = '';
  buildZatcaTlv(input).forEach((b) => (binary += String.fromCharCode(b)));
  return btoa(binary);
}

/** Decode a payload back into its fields (used by tests / the settings preview). */
export function decodeZatcaQr(base64: string): Record<number, string> {
  const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
  const decoder = new TextDecoder();
  const fields: Record<number, string> = {};
  for (let i = 0; i < bytes.length; ) {
    const tag = bytes[i];
    const len = bytes[i + 1];
    fields[tag] = decoder.decode(bytes.slice(i + 2, i + 2 + len));
    i += 2 + len;
  }
  return fields;
}
