/**
 * `pdfService`: builds a `DocumentPayload` (docs/v2/12-documents-pdf-excel.md
 * §2) from a real document and drives the Rust `render_pdf` / `render_preview`
 * commands. Phase 11a wires up `kind: 'invoice'` only — the other 9 kinds are
 * Phase 11b (they'd each need their own `buildXPayload`, but reuse this same
 * `render`/`renderPreview`/`open` machinery).
 *
 * Browser dev-mode fallback: `@tauri-apps/api/core`'s `isTauri()` is already
 * how this codebase detects Tauri (see `src/modules/reports/helpers/export.ts`).
 * When not in Tauri, `render()` shows a toast and returns a signal telling the
 * caller to fall back to the v1 print route instead of calling `invoke`.
 */
import { isTauri } from '@tauri-apps/api/core';
import { encode as uqrEncode } from 'uqr';
import { formatDate, formatDateTime, formatMoney, formatNumber } from '@/modules/core/helpers/format';
import { tafqit } from '@/modules/core/helpers/tafqit';
import { useToast } from '@/modules/core/controllers/useToast';
import { getInvoicePrintData } from '@/modules/invoices/services/invoiceService';
import { round2 } from '@/modules/invoices/helpers/totals';
import { zatcaQrBase64 } from '@/modules/invoices/helpers/zatcaQr';
import { getDefaultTemplate, getTemplate } from '@/modules/templates/services/templateService';
import { defaultTemplateOptions, type DocumentKind, type PdfTemplate, type TemplateOptions } from '@/modules/templates/types';

export type PdfDocumentKind = 'invoice';

/** Mirrors src-tauri/src/pdf/payload.rs's `DocumentPayload` shape. */
export interface DocumentPayload {
  document: { kind: string; number: string; date: string; titleAr: string; titleEn: string };
  company: Record<string, unknown>;
  party: Record<string, unknown> | null;
  lines: Record<string, unknown>[];
  totals: Record<string, unknown>;
  qr: string | null;
  logo: string | null;
}

/** Builds the QR as inline SVG markup (same rendering approach as `QrCode.vue`, extracted so both the web print route and the Rust template can use an equivalent SVG). */
function qrSvg(value: string, sizeMm = 30): string {
  const qr = uqrEncode(value, { ecc: 'M', border: 2 });
  let d = '';
  qr.data.forEach((row, y) =>
    row.forEach((on, x) => {
      if (on) d += `M${x},${y}h1v1h-1z`;
    }),
  );
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${qr.size} ${qr.size}" width="${sizeMm}mm" height="${sizeMm}mm" shape-rendering="crispEdges"><rect width="${qr.size}" height="${qr.size}" fill="#fff"/><path d="${d}" fill="#000"/></svg>`;
}

async function buildInvoicePayload(id: string): Promise<DocumentPayload> {
  const data = await getInvoicePrintData(id);
  const inv = data.invoice;
  const s = data.settings;
  const isB2B = !!data.customer?.vatNumber;

  const qrValue = zatcaQrBase64({
    sellerName: s.storeName,
    vatNumber: s.vatNumber ?? '',
    timestamp: inv.date,
    invoiceTotal: inv.grandTotal,
    vatTotal: inv.taxAmount,
  });

  const lines = inv.lines.map((l) => {
    const net = round2(l.qty * l.price - l.discount);
    const vat = round2((net * (1 - inv.discountRate / 100) * inv.taxRate) / 100);
    const total = round2(net * (1 - inv.discountRate / 100) * (1 + inv.taxRate / 100));
    return {
      name: l.name,
      qty: formatNumber(l.qty),
      price: formatMoney(l.price),
      discount: formatMoney(l.discount),
      net: formatMoney(net),
      vatRate: inv.taxRate,
      vat: formatMoney(vat),
      total: formatMoney(total),
    };
  });

  const outstanding = Math.max(0, round2(inv.grandTotal - inv.refundedAmount - inv.paidAmount));

  return {
    document: {
      kind: 'invoice',
      number: inv.number,
      date: formatDateTime(inv.date),
      titleAr: isB2B ? 'فاتورة ضريبية' : 'فاتورة ضريبية مبسطة',
      titleEn: isB2B ? 'TAX INVOICE' : 'SIMPLIFIED TAX INVOICE',
    },
    company: {
      name: s.storeName,
      address: s.address ?? null,
      phone: s.phone ?? null,
      email: null,
      website: null,
      vatNumber: s.vatNumber ?? null,
      commercialRegister: s.commercialRegister ?? null,
      logo: s.logo ?? null,
    },
    party: data.customer
      ? { name: data.customer.name, vatNumber: data.customer.vatNumber ?? null, address: data.customer.address ?? null, phone: data.customer.phone ?? null }
      : null,
    lines,
    totals: {
      subtotal: formatMoney(inv.subTotal),
      discount: inv.discountAmount > 0 ? formatMoney(inv.discountAmount) : null,
      vat: formatMoney(inv.taxAmount),
      grand: formatMoney(inv.grandTotal),
      paid: inv.paidAmount > 0 ? formatMoney(inv.paidAmount) : null,
      remaining: inv.paidAmount > 0 ? formatMoney(outstanding) : null,
      amountInWords: tafqit(inv.grandTotal),
      previousBalance: null,
      currentBalance: null,
    },
    qr: qrSvg(qrValue),
    logo: s.logo ?? null,
  };
}

async function buildPayload(kind: PdfDocumentKind, id: string): Promise<DocumentPayload> {
  if (kind === 'invoice') return buildInvoicePayload(id);
  throw new Error(`pdfService: unsupported document kind '${kind}' (Phase 11b)`);
}

function resolveTemplate(kind: PdfDocumentKind, templateId?: string): PdfTemplate {
  const documentKind: DocumentKind = kind;
  const template = (templateId ? getTemplate(templateId) : undefined) ?? getDefaultTemplate(documentKind);
  if (template) return template;
  return {
    id: 'fallback',
    name: 'افتراضي',
    kind: documentKind,
    baseTemplateId: 'invoice_standard',
    options: defaultTemplateOptions(),
    customSource: null,
    isDefault: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
}

interface RenderRequestBody {
  template_source: string | null;
  template_id: string | null;
  payload: DocumentPayload;
  options: TemplateOptions;
}

function buildRequest(payload: DocumentPayload, template: PdfTemplate): RenderRequestBody {
  return {
    template_source: template.customSource,
    template_id: template.customSource ? null : template.baseTemplateId,
    payload,
    options: template.options,
  };
}

export interface RenderPdfOutcome {
  /** false when running outside Tauri — caller should fall back to the v1 print route. */
  ok: boolean;
  pdfBytes?: Uint8Array;
  achievedStandard?: string;
}

const DESKTOP_ONLY_MESSAGE = 'ملف PDF الفعلي متاح في نسخة سطح المكتب';

/** Renders a real PDF for the given document and returns its bytes. In browser dev mode, shows a toast and returns `{ ok: false }` so the caller can fall back to `/print/...`. */
export async function render(kind: PdfDocumentKind, id: string, templateId?: string): Promise<RenderPdfOutcome> {
  if (!isTauri()) {
    useToast().info(DESKTOP_ONLY_MESSAGE);
    return { ok: false };
  }
  const payload = await buildPayload(kind, id);
  const template = resolveTemplate(kind, templateId);
  const { invoke } = await import('@tauri-apps/api/core');
  const result = await invoke<{ pdf_base64: string; achieved_standard: string; warnings: unknown[] }>(
    'render_pdf',
    { req: buildRequest(payload, template) },
  );
  const binary = atob(result.pdf_base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return { ok: true, pdfBytes: bytes, achievedStandard: result.achieved_standard };
}

/** Saves the rendered PDF via the native save dialog and opens it. No-op (returns false) outside Tauri. */
export async function renderAndSave(kind: PdfDocumentKind, id: string, filename: string, templateId?: string): Promise<boolean> {
  const outcome = await render(kind, id, templateId);
  if (!outcome.ok || !outcome.pdfBytes) return false;
  const [{ save }, { writeFile }, { openPath }] = await Promise.all([
    import('@tauri-apps/plugin-dialog'),
    import('@tauri-apps/plugin-fs'),
    import('@tauri-apps/plugin-opener'),
  ]);
  const path = await save({ defaultPath: filename, filters: [{ name: 'PDF', extensions: ['pdf'] }] });
  if (!path) return false;
  await writeFile(path, outcome.pdfBytes);
  await openPath(path);
  return true;
}

export interface RenderPreviewOutcome {
  pages: string[];
  warnings: { line: number | null; column: number | null; severity: string; message: string }[];
}

export interface PreviewError {
  diagnostics: { line: number | null; column: number | null; severity: string; message: string }[];
}

/**
 * Renders live SVG preview pages for the template designer. Unlike `render`,
 * this accepts an in-progress (possibly invalid) template directly — the
 * designer calls this on every debounced option/source change, not just for
 * a saved template — so it takes the request pieces directly rather than a
 * saved template id.
 */
export async function renderPreview(
  payload: DocumentPayload,
  options: TemplateOptions,
  templateSource: string | null,
  baseTemplateId: PdfTemplate['baseTemplateId'],
): Promise<RenderPreviewOutcome> {
  if (!isTauri()) {
    return { pages: [], warnings: [] };
  }
  const { invoke } = await import('@tauri-apps/api/core');
  try {
    const result = await invoke<{ pages: string[]; warnings: RenderPreviewOutcome['warnings'] }>('render_preview', {
      req: {
        template_source: templateSource,
        template_id: templateSource ? null : baseTemplateId,
        payload,
        options,
      },
    });
    return { pages: result.pages, warnings: result.warnings ?? [] };
  } catch (err) {
    const diagnostics = Array.isArray(err) ? (err as PreviewError['diagnostics']) : [{ line: null, column: null, severity: 'error', message: String(err) }];
    throw { diagnostics } satisfies PreviewError;
  }
}

/** A small self-contained sample invoice payload, for the designer's live preview before picking a real document. */
export function sampleInvoicePayload(): DocumentPayload {
  const now = new Date().toISOString();
  return {
    document: { kind: 'invoice', number: 'INV-000123', date: formatDate(now), titleAr: 'فاتورة ضريبية', titleEn: 'TAX INVOICE' },
    company: {
      name: 'مؤسسة النموذج التجارية',
      address: 'الرياض، حي العليا، طريق الملك فهد',
      phone: '0112345678',
      email: 'info@example.com',
      website: 'example.com',
      vatNumber: '311111111100003',
      commercialRegister: '1010123456',
      logo: null,
    },
    party: { name: 'شركة العميل النموذجي', vatNumber: '311222222200003', address: 'جدة، حي الروضة', phone: '0509876543' },
    lines: [
      { name: 'منتج تجريبي أول', qty: '٢', price: '150.00', discount: '0.00', net: '300.00', vatRate: 15, vat: '45.00', total: '345.00' },
      { name: 'منتج تجريبي ثانٍ مع اسم طويل لاختبار التفاف النص داخل الخلية', qty: '١', price: '89.50', discount: '0.00', net: '89.50', vatRate: 15, vat: '13.43', total: '102.93' },
      { name: 'خدمة صيانة', qty: '٣', price: '45.00', discount: '10.00', net: '125.00', vatRate: 15, vat: '18.75', total: '143.75' },
    ],
    totals: { subtotal: '514.50', discount: null, vat: '77.18', grand: '591.68', paid: null, remaining: null, amountInWords: tafqit(591.68), previousBalance: null, currentBalance: null },
    qr: qrSvg('sample-preview-qr-value'),
    logo: null,
  };
}
