/**
 * `pdfService`: builds a `DocumentPayload` (docs/v2/12-documents-pdf-excel.md
 * §2) from a real document and drives the Rust `render_pdf` / `render_preview`
 * commands. Phase 11a wired up `kind: 'invoice'` only. Phase 11b (this file's
 * additions below `buildInvoicePayload`) adds the other 9 document kinds —
 * quotation, credit/debit note, purchase order, voucher, party statement,
 * Z-report, transfer note and the generic report template — each with its
 * own `buildXPayload`, reusing this same `render`/`renderPreview`/`open`
 * machinery. Labels are a separate entry point (`renderLabels` below) since
 * they don't map to one existing document id the way the other kinds do.
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
import { saveFile } from '@/modules/core/services/saveFile';
import { getInvoicePrintData, getQuotation, getRefund, getShift } from '@/modules/invoices/services/invoiceService';
import { round2 } from '@/modules/invoices/helpers/totals';
import { zatcaQrBase64 } from '@/modules/invoices/helpers/zatcaQr';
import { getPurchaseOrder, getPurchaseReturn } from '@/modules/purchases/services/purchaseService';
import { getVoucher } from '@/modules/vouchers/services/voucherService';
import type { Voucher } from '@/modules/vouchers/types';
import { getAccounts } from '@/modules/accounting/services/accountingService';
import { getPaymentMethods } from '@/modules/settings/services/settingsService';
import { getCustomer, getCustomerStatement, getSupplier, getSupplierStatement } from '@/modules/parties/services/partyService';
import { getTransfer } from '@/modules/products/services/transferService';
import { getProducts } from '@/modules/products/services/productService';
import { getBranches } from '@/modules/settings/services/branchesService';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { getDefaultTemplate, getTemplate } from '@/modules/templates/services/templateService';
import { defaultTemplateOptions, type BaseTemplateId, type DocumentKind, type LabelOptions, type PdfTemplate, type TemplateOptions } from '@/modules/templates/types';
import { barcodeSvg, looksLikeEan13 } from '@/modules/products/helpers/labelBarcode';
import type { ReportDocument } from '@/modules/reports/print/types';

export type PdfDocumentKind =
  | 'invoice'
  | 'quotation'
  | 'creditNote'
  | 'debitNote'
  | 'purchaseOrder'
  | 'voucher'
  | 'statement'
  | 'zReport'
  | 'transferNote';

/** Maps a document kind to the built-in `.typ` template id `render.rs` knows (src-tauri/src/pdf/render.rs's `builtin_template_source`). */
const BUILTIN_TEMPLATE_ID: Record<PdfDocumentKind, BaseTemplateId> = {
  invoice: 'invoice_standard',
  quotation: 'quotation',
  creditNote: 'credit_note',
  debitNote: 'debit_note',
  purchaseOrder: 'purchase_order',
  voucher: 'voucher',
  statement: 'statement',
  zReport: 'z_report',
  transferNote: 'transfer_note',
};

/**
 * Mirrors src-tauri/src/pdf/payload.rs's `DocumentPayload` shape. `document` carries a handful of
 * required fields plus an open-ended bag of per-kind extras (e.g. a credit note's `refNumber`, a
 * voucher's `accountsLine`, a Z-report's `openedAt`) — Rust's `DocumentMeta` only reads the fixed
 * fields itself and the Typst templates read the rest by key with `.at(..., default: ...)`, same
 * "unknown fields pass through opaquely" contract `lib.typ`'s top-of-file comment documents for
 * `opts`.
 */
export interface DocumentPayload {
  document: { kind: string; number: string; date: string; titleAr: string; titleEn: string } & Record<string, unknown>;
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

/** Shared company block — every non-invoice builder below reads it the same way `buildInvoicePayload` does. */
function companyBlock() {
  const s = useSettingsStore().settings;
  return {
    name: s?.storeName ?? '',
    address: s?.address ?? null,
    phone: s?.phone ?? null,
    email: null,
    website: null,
    vatNumber: s?.vatNumber ?? null,
    commercialRegister: s?.commercialRegister ?? null,
    logo: s?.logo ?? null,
  };
}

async function buildQuotationPayload(id: string): Promise<DocumentPayload> {
  const q = await getQuotation(id);
  const customer = q.customerId ? await getCustomer(q.customerId).catch(() => undefined) : undefined;

  const lines = q.lines.map((l) => {
    const net = round2(l.qty * l.price - l.discount);
    const vatRate = l.taxRate ?? 0;
    const vat = round2((net * (1 - q.discountRate / 100) * vatRate) / 100);
    const total = round2(net * (1 - q.discountRate / 100) * (1 + vatRate / 100));
    return { name: l.name, qty: formatNumber(l.qty), price: formatMoney(l.price), discount: formatMoney(l.discount), net: formatMoney(net), vatRate, vat: formatMoney(vat), total: formatMoney(total) };
  });

  return {
    document: { kind: 'quotation', number: q.number, date: formatDate(q.date), titleAr: 'عرض سعر', titleEn: 'QUOTATION', validUntil: q.expiryDate ? formatDate(q.expiryDate) : null },
    company: companyBlock(),
    party: customer ? { name: customer.name, vatNumber: customer.vatNumber ?? null, address: customer.address ?? null, phone: customer.phone ?? null } : null,
    lines,
    totals: { subtotal: formatMoney(q.subTotal), discount: q.discountAmount > 0 ? formatMoney(q.discountAmount) : null, vat: formatMoney(q.taxAmount), grand: formatMoney(q.grandTotal), paid: null, remaining: null, amountInWords: tafqit(q.grandTotal), previousBalance: null, currentBalance: null },
    qr: null,
    logo: companyBlock().logo,
  };
}

async function buildCreditNotePayload(id: string): Promise<DocumentPayload> {
  const refund = await getRefund(id);
  const data = await getInvoicePrintData(refund.invoiceId);
  const inv = data.invoice;
  const customer = data.customer;

  const refundLines = refund.lines
    .map((rl) => {
      const src = inv.lines.find((l) => l.id === rl.invoiceLineId);
      if (!src) return null;
      const net = round2(rl.qty * src.price - src.discount * (rl.qty / Math.max(1, src.qty)));
      const vatRate = src.taxRate ?? inv.taxRate;
      const vat = round2((net * vatRate) / 100);
      return { name: src.name, qty: formatNumber(rl.qty), price: formatMoney(src.price), discount: '0.00', net: formatMoney(net), vatRate, vat: formatMoney(vat), total: formatMoney(net + vat) };
    })
    .filter((l): l is NonNullable<typeof l> => l !== null);

  return {
    document: {
      kind: 'creditNote',
      number: refund.number,
      date: formatDate(refund.date),
      titleAr: 'إشعار دائن',
      titleEn: 'CREDIT NOTE',
      refNumber: inv.number,
      refDate: formatDate(inv.date),
      reason: refund.reason ?? null,
    },
    company: companyBlock(),
    party: customer ? { name: customer.name, vatNumber: customer.vatNumber ?? null, address: customer.address ?? null, phone: customer.phone ?? null } : null,
    lines: refundLines,
    totals: { subtotal: formatMoney(refund.subTotal), discount: null, vat: formatMoney(refund.taxAmount), grand: formatMoney(refund.grandTotal), paid: null, remaining: null, amountInWords: tafqit(refund.grandTotal), previousBalance: null, currentBalance: null },
    qr: null,
    logo: companyBlock().logo,
  };
}

async function buildDebitNotePayload(id: string): Promise<DocumentPayload> {
  const ret = await getPurchaseReturn(id);
  const po = await getPurchaseOrder(ret.purchaseOrderId);
  const supplier = po.supplier;

  const lines = ret.lines.map((rl) => {
    const product = po.products[rl.productId];
    const net = round2(rl.qty * rl.costPrice);
    return { name: product?.name ?? rl.productId, sku: product?.sku ?? '', qty: formatNumber(rl.qty), price: formatMoney(rl.costPrice), discount: '0.00', net: formatMoney(net), vatRate: 0, vat: '0.00', total: formatMoney(net) };
  });

  return {
    document: {
      kind: 'debitNote',
      number: ret.number,
      date: formatDate(ret.date),
      titleAr: 'إشعار مدين',
      titleEn: 'DEBIT NOTE',
      refNumber: po.number,
      refDate: formatDate(po.date),
      reason: ret.reason ?? null,
    },
    company: companyBlock(),
    party: supplier ? { name: supplier.name, vatNumber: supplier.vatNumber ?? null, address: supplier.address ?? null, phone: supplier.phone ?? null } : null,
    lines,
    totals: { subtotal: formatMoney(ret.subTotal), discount: null, vat: formatMoney(ret.taxAmount), grand: formatMoney(ret.grandTotal), paid: null, remaining: null, amountInWords: tafqit(ret.grandTotal), previousBalance: null, currentBalance: null },
    qr: null,
    logo: companyBlock().logo,
  };
}

async function buildPurchaseOrderPayload(id: string): Promise<DocumentPayload> {
  const po = await getPurchaseOrder(id);
  const supplier = po.supplier;

  const lines = po.lines.map((l) => {
    const product = po.products[l.productId];
    const factor = l.unitFactor ?? 1;
    const net = round2(l.qty * l.costPrice);
    return { name: product?.name ?? l.productId, sku: product?.sku ?? '', unit: '', qty: formatNumber(l.qty * factor === l.qty ? l.qty : l.qty), price: formatMoney(l.costPrice), discount: '0.00', net: formatMoney(net), vatRate: 0, vat: '0.00', total: formatMoney(net) };
  });

  return {
    document: { kind: 'purchaseOrder', number: po.number, date: formatDate(po.date), titleAr: 'أمر شراء', titleEn: 'PURCHASE ORDER' },
    company: companyBlock(),
    party: supplier ? { name: supplier.name, vatNumber: supplier.vatNumber ?? null, address: supplier.address ?? null, phone: supplier.phone ?? null } : null,
    lines,
    totals: { subtotal: formatMoney(po.subTotal), discount: null, vat: formatMoney(po.taxAmount), grand: formatMoney(po.grandTotal), paid: null, remaining: null, amountInWords: tafqit(po.grandTotal), previousBalance: null, currentBalance: null },
    qr: null,
    logo: companyBlock().logo,
  };
}

const VOUCHER_KIND_LABEL: Record<Voucher['kind'], { ar: string; en: string }> = {
  RECEIPT: { ar: 'سند قبض', en: 'RECEIPT VOUCHER' },
  PAYMENT: { ar: 'سند صرف', en: 'PAYMENT VOUCHER' },
  TRANSFER: { ar: 'سند تحويل', en: 'TRANSFER VOUCHER' },
  OWNER: { ar: 'سند مالك', en: 'OWNER VOUCHER' },
};

async function buildVoucherPayload(id: string): Promise<DocumentPayload> {
  const voucher = await getVoucher(id);
  const [accounts, methods] = await Promise.all([getAccounts(), getPaymentMethods()]);
  const accountName = (accountId?: string) => accounts.find((a) => a.id === accountId)?.name ?? accountId ?? '';
  const methodName = (methodId?: string) => methods.find((m) => m.id === methodId)?.name ?? methodId ?? '';

  let accountsLine = '';
  if (voucher.kind === 'RECEIPT') accountsLine = `من: ${methodName(voucher.paymentMethodId)} — إلى: ${accountName(voucher.creditAccountId)}`;
  else if (voucher.kind === 'PAYMENT') accountsLine = `من: ${accountName(voucher.debitAccountId)} — إلى: ${methodName(voucher.paymentMethodId)}`;
  else if (voucher.kind === 'TRANSFER') accountsLine = `من: ${accountName(voucher.sourceAccountId)} — إلى: ${accountName(voucher.destinationAccountId)}`;
  else accountsLine = `${voucher.direction === 'drawings' ? 'مسحوبات من' : 'إضافة إلى'}: ${accountName(voucher.cashAccountId)}`;

  const label = VOUCHER_KIND_LABEL[voucher.kind];

  return {
    document: {
      kind: 'voucher',
      number: voucher.number,
      date: formatDate(voucher.date),
      titleAr: label.ar,
      titleEn: label.en,
      description: voucher.description,
      accountsLine,
      note: voucher.note ?? null,
    },
    company: companyBlock(),
    party: null,
    lines: [],
    totals: { subtotal: null, discount: null, vat: null, grand: formatMoney(voucher.amount), paid: null, remaining: null, amountInWords: tafqit(voucher.amount), previousBalance: null, currentBalance: null },
    qr: null,
    logo: companyBlock().logo,
  };
}

async function buildStatementPayload(id: string): Promise<DocumentPayload> {
  // `id` is "customer:<id>" or "supplier:<id>" — the print button on each party's page knows
  // which kind it is (see PartyDetailPage's call site below).
  const [kind, partyId] = id.split(':');
  const isCustomer = kind === 'customer';
  const party = isCustomer ? await getCustomer(partyId) : await getSupplier(partyId);
  const rows = isCustomer ? await getCustomerStatement(partyId) : await getSupplierStatement(partyId);

  const lines = rows.map((r) => ({
    date: formatDate(r.date),
    description: r.description,
    number: r.number,
    debit: r.debit > 0 ? formatMoney(r.debit) : '',
    credit: r.credit > 0 ? formatMoney(r.credit) : '',
    balance: formatMoney(r.balance),
  }));
  const closing = rows.length ? rows[rows.length - 1].balance : 0;

  return {
    document: { kind: 'statement', number: '', date: formatDate(new Date().toISOString()), titleAr: 'كشف حساب', titleEn: 'STATEMENT OF ACCOUNT' },
    company: companyBlock(),
    party: { name: party.name, code: party.code ?? null, vatNumber: party.vatNumber ?? null, address: party.address ?? null, phone: party.phone ?? null },
    lines,
    totals: { subtotal: null, discount: null, vat: null, grand: formatMoney(closing), paid: null, remaining: null, amountInWords: null, previousBalance: null, currentBalance: null },
    qr: null,
    logo: companyBlock().logo,
  };
}

async function buildZReportPayload(id: string): Promise<DocumentPayload> {
  const shift = await getShift(id);

  const movements = shift.movements.map((m) => ({
    time: formatDateTime(m.at),
    kind: m.kind,
    ref: m.refNumber ?? '',
    amount: formatMoney(m.amount),
  }));
  const variance = round2((shift.countedCash ?? shift.expectedCash) - shift.expectedCash);

  return {
    document: { kind: 'zReport', number: shift.number, date: formatDateTime(shift.closedAt ?? shift.openedAt), titleAr: 'تقرير إغلاق الوردية (Z)', titleEn: 'Z-REPORT', cashierName: shift.openedByName, openedAt: formatDateTime(shift.openedAt), closedAt: shift.closedAt ? formatDateTime(shift.closedAt) : null },
    company: companyBlock(),
    party: null,
    lines: movements,
    totals: {
      openingFloat: formatMoney(shift.openingFloat),
      cashSales: formatMoney(shift.cashSales),
      cashRefunds: formatMoney(shift.cashRefunds),
      payIns: formatMoney(shift.payIns),
      payOuts: formatMoney(shift.payOuts),
      bankDrops: formatMoney(shift.bankDrops),
      expectedCash: formatMoney(shift.expectedCash),
      countedCash: formatMoney(shift.countedCash ?? shift.expectedCash),
      grand: formatMoney(variance),
    },
    qr: null,
    logo: companyBlock().logo,
  };
}

async function buildTransferNotePayload(id: string): Promise<DocumentPayload> {
  const transfer = await getTransfer(id);
  const [branches, products] = await Promise.all([getBranches(), getProducts({ includeInactive: true })]);
  const branchName = (branchId: string) => branches.find((b) => b.id === branchId)?.name ?? branchId;
  const productName = (productId: string) => products.find((p) => p.id === productId)?.name ?? productId;

  const lines = transfer.lines.map((l) => ({
    name: productName(l.productId),
    qty: formatNumber(l.qty),
    price: '',
    discount: '',
    net: '',
    vatRate: 0,
    vat: '',
    total: formatNumber(l.receivedQty ?? l.qty),
  }));

  return {
    document: {
      kind: 'transferNote',
      number: transfer.number,
      date: formatDate(transfer.date),
      titleAr: 'إذن تحويل مخزون',
      titleEn: 'STOCK TRANSFER NOTE',
      fromBranch: branchName(transfer.fromBranchId),
      toBranch: branchName(transfer.toBranchId),
      note: transfer.note ?? null,
    },
    company: companyBlock(),
    party: null,
    lines,
    totals: { subtotal: null, discount: null, vat: null, grand: null, paid: null, remaining: null, amountInWords: null, previousBalance: null, currentBalance: null },
    qr: null,
    logo: companyBlock().logo,
  };
}

async function buildPayload(kind: PdfDocumentKind, id: string): Promise<DocumentPayload> {
  switch (kind) {
    case 'invoice':
      return buildInvoicePayload(id);
    case 'quotation':
      return buildQuotationPayload(id);
    case 'creditNote':
      return buildCreditNotePayload(id);
    case 'debitNote':
      return buildDebitNotePayload(id);
    case 'purchaseOrder':
      return buildPurchaseOrderPayload(id);
    case 'voucher':
      return buildVoucherPayload(id);
    case 'statement':
      return buildStatementPayload(id);
    case 'zReport':
      return buildZReportPayload(id);
    case 'transferNote':
      return buildTransferNotePayload(id);
    default:
      throw new Error(`pdfService: unsupported document kind '${kind}'`);
  }
}

function resolveTemplate(kind: PdfDocumentKind, templateId?: string): PdfTemplate {
  const documentKind: DocumentKind = kind;
  const template = (templateId ? getTemplate(templateId) : undefined) ?? getDefaultTemplate(documentKind);
  if (template) return template;
  return {
    id: 'fallback',
    name: 'افتراضي',
    kind: documentKind,
    baseTemplateId: BUILTIN_TEMPLATE_ID[kind],
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

/**
 * Native "Save as" → write → open in the default PDF viewer. Returns false when the dialog is
 * cancelled. Opening is best-effort: the file is already on disk at that point, so a viewer or
 * permission problem (`opener:allow-open-path` is scoped to `*.pdf` in capabilities/default.json)
 * only shows where it was saved — it never turns a successful save into an error. `silent: true`
 * since opening the PDF viewer is the success signal here, not the shared "تم الحفظ" toast.
 */
async function savePdfBytes(bytes: Uint8Array, filename: string): Promise<boolean> {
  const path = await saveFile(bytes, { suggestedName: filename, kind: 'pdf', silent: true });
  if (!path) return false;
  try {
    const { openPath } = await import('@tauri-apps/plugin-opener');
    await openPath(path as string);
  } catch {
    useToast().info('تم حفظ الملف', path as string);
  }
  return true;
}

/** Saves the rendered PDF via the native save dialog and opens it. No-op (returns false) outside Tauri. */
export async function renderAndSave(kind: PdfDocumentKind, id: string, filename: string, templateId?: string): Promise<boolean> {
  const outcome = await render(kind, id, templateId);
  if (!outcome.ok || !outcome.pdfBytes) return false;
  return savePdfBytes(outcome.pdfBytes, filename);
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

// =================================================================================================
// Labels (Phase 11b) — docs/v2/07-products-and-inventory.md §6, docs/v2/12-documents-pdf-excel.md
// §3/§4. A separate small entry point rather than another `PdfDocumentKind`/`buildXPayload`: a
// label batch isn't "one document with one id" the way an invoice/voucher/etc. is — it's an
// arbitrary set of product×qty picks the label builder page assembles, so the caller builds the
// item list itself and hands it to `renderLabels`/`renderLabelsPreview` directly.
// =================================================================================================

/** One picked line in the label builder: a product, the unit it should be priced/labeled in, and how many copies. */
export interface LabelPick {
  productId: string;
  name: string;
  sku?: string;
  barcode?: string;
  priceText: string;
  unit?: string;
  batchNo?: string;
  expiryText?: string;
  copies: number;
}

/** One rendered label item, ready to embed in the label `DocumentPayload` (`data.labels[i]`). */
export interface LabelDataItem {
  name: string;
  priceText: string;
  sku?: string;
  unit?: string;
  batchNo?: string;
  expiryText?: string;
  barcodeSvg?: string | null;
  qrSvg?: string | null;
}

/**
 * Expands each pick into `copies` individual label items and generates its barcode (EAN-13 when
 * the barcode looks like one, Code-128 otherwise) and optional QR (encoding the barcode itself,
 * per docs/v2/07-products-and-inventory.md §6 "QR (encodes the barcode ...)"). Barcode/QR
 * generation happens once per distinct pick, not once per copy, since bwip-js/uqr output is
 * identical across copies of the same product — the copies are only expanded afterward.
 */
export async function buildLabelItems(picks: LabelPick[], opts: { includeQr: boolean }): Promise<LabelDataItem[]> {
  const rendered = await Promise.all(
    picks.map(async (p) => {
      const symbology = looksLikeEan13(p.barcode) ? 'ean13' : 'code128';
      const barcode = p.barcode ? await barcodeSvg(p.barcode, symbology) : null;
      const qr = opts.includeQr && p.barcode ? qrSvg(p.barcode, 20) : null;
      const item: LabelDataItem = { name: p.name, priceText: p.priceText, sku: p.sku, unit: p.unit, batchNo: p.batchNo, expiryText: p.expiryText, barcodeSvg: barcode, qrSvg: qr };
      return { item, copies: Math.max(1, Math.floor(p.copies) || 1) };
    }),
  );
  const items: LabelDataItem[] = [];
  for (const { item, copies } of rendered) {
    for (let i = 0; i < copies; i++) items.push(item);
  }
  return items;
}

/** Builds the label `DocumentPayload` — `data.labels` (not `data.lines`) is the array the label templates read. */
function buildLabelPayload(items: LabelDataItem[]): DocumentPayload & { labels: LabelDataItem[] } {
  const s = useSettingsStore().settings;
  return {
    document: { kind: 'label', number: '', date: formatDate(new Date().toISOString()), titleAr: 'ملصقات', titleEn: 'LABELS' },
    company: { name: s?.storeName ?? '', logo: s?.logo ?? null },
    party: null,
    lines: [],
    totals: {},
    qr: null,
    logo: s?.logo ?? null,
    labels: items,
  };
}

function labelTemplateOptions(label: LabelOptions): TemplateOptions {
  const base = defaultTemplateOptions();
  return { ...base, fontFamily: label.fontFamily, paper: 'a4', label } as TemplateOptions & { label: LabelOptions };
}

export interface RenderLabelsOutcome {
  ok: boolean;
  pdfBytes?: Uint8Array;
}

/** Renders a label batch to PDF and returns its bytes. Mirrors `render()`'s browser-dev-mode fallback. */
export async function renderLabels(picks: LabelPick[], label: LabelOptions): Promise<RenderLabelsOutcome> {
  if (!isTauri()) {
    useToast().info(DESKTOP_ONLY_MESSAGE);
    return { ok: false };
  }
  const items = await buildLabelItems(picks, { includeQr: label.showQr });
  const payload = buildLabelPayload(items);
  const templateId = label.layout === 'thermal' ? 'label_thermal' : 'label_sheet';
  const { invoke } = await import('@tauri-apps/api/core');
  const result = await invoke<{ pdf_base64: string; achieved_standard: string; warnings: unknown[] }>('render_pdf', {
    req: { template_source: null, template_id: templateId, payload, options: labelTemplateOptions(label) },
  });
  const binary = atob(result.pdf_base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return { ok: true, pdfBytes: bytes };
}

/** Saves the rendered label PDF via the native save dialog and opens it. No-op (returns false) outside Tauri. */
export async function renderLabelsAndSave(picks: LabelPick[], label: LabelOptions, filename = 'labels.pdf'): Promise<boolean> {
  const outcome = await renderLabels(picks, label);
  if (!outcome.ok || !outcome.pdfBytes) return false;
  return savePdfBytes(outcome.pdfBytes, filename);
}

// =================================================================================================
// Generic report (Phase 11b) — docs/v2/12-documents-pdf-excel.md §3 "Reports: a generic report
// template (title, filter line, table with a repeating header, totals, page x of y)". The reports
// module and every other "print a list/statement" screen now use the official layout instead
// (`renderReportPdf`/`saveReportPdf` below + `src/modules/reports/print/`); this plain table
// template stays available for ad-hoc tabular exports.
// =================================================================================================

export interface GenericReportColumn {
  key: string;
  label: string;
}

export interface GenericReportRequest {
  titleAr: string;
  titleEn?: string;
  filterLine?: string;
  columns: GenericReportColumn[];
  rows: Record<string, string>[];
  totalText?: string;
}

function buildGenericReportPayload(req: GenericReportRequest): DocumentPayload {
  return {
    document: { kind: 'report', number: '', date: formatDate(new Date().toISOString()), titleAr: req.titleAr, titleEn: req.titleEn ?? '', filterLine: req.filterLine ?? null },
    company: companyBlock(),
    party: null,
    lines: req.rows,
    totals: { grand: req.totalText ?? null },
    qr: null,
    logo: companyBlock().logo,
  };
}

function genericReportOptions(columns: GenericReportColumn[]): TemplateOptions {
  const base = defaultTemplateOptions();
  return { ...base, columns: columns.map((c) => ({ key: c.key as TemplateOptions['columns'][number]['key'], label: c.label, visible: true })) };
}

/** Renders a generic report to PDF and returns its bytes, browser-dev-mode fallback included. */
export async function renderGenericReport(req: GenericReportRequest): Promise<RenderPdfOutcome> {
  if (!isTauri()) {
    useToast().info(DESKTOP_ONLY_MESSAGE);
    return { ok: false };
  }
  const { invoke } = await import('@tauri-apps/api/core');
  const result = await invoke<{ pdf_base64: string; achieved_standard: string; warnings: unknown[] }>('render_pdf', {
    req: { template_source: null, template_id: 'generic_report', payload: buildGenericReportPayload(req), options: genericReportOptions(req.columns) },
  });
  const binary = atob(result.pdf_base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return { ok: true, pdfBytes: bytes, achievedStandard: result.achieved_standard };
}

/** Saves the rendered generic-report PDF via the native save dialog and opens it. No-op (returns false) outside Tauri. */
export async function renderGenericReportAndSave(req: GenericReportRequest, filename: string): Promise<boolean> {
  const outcome = await renderGenericReport(req);
  if (!outcome.ok || !outcome.pdfBytes) return false;
  return savePdfBytes(outcome.pdfBytes, filename);
}

// =================================================================================================
// Official reports — `src-tauri/templates/report.typ` renders the same `ReportDocument` model the
// reports module prints/previews as HTML (src/modules/reports/print/), so the saved PDF is a real
// document render (letterhead, meta strip, sectioned tables, signatures, page x of y), never a
// capture of the on-screen page.
// =================================================================================================

/** Renders an official report to PDF bytes (desktop only — returns null in the browser). */
export async function renderReportPdf(report: ReportDocument): Promise<Uint8Array | null> {
  if (!isTauri()) return null;
  const payload = {
    document: { kind: 'report', number: '', date: report.issuedAt, titleAr: report.title, titleEn: '' },
    company: companyBlock(),
    party: null,
    lines: [],
    totals: {},
    qr: null,
    logo: report.company.logo ?? null,
    report,
  };
  const { invoke } = await import('@tauri-apps/api/core');
  const result = await invoke<{ pdf_base64: string; achieved_standard: string; warnings: unknown[] }>('render_pdf', {
    req: { template_source: null, template_id: 'report', payload, options: { paper: 'a4' } },
  });
  const binary = atob(result.pdf_base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

/** Renders the report PDF, asks where to save it, writes and opens it. False when cancelled or outside Tauri. */
export async function saveReportPdf(report: ReportDocument, filename: string): Promise<boolean> {
  const bytes = await renderReportPdf(report);
  if (!bytes) return false;
  return savePdfBytes(bytes, filename);
}

/** Live SVG preview for the label builder — same `render_preview` command the template designer uses, mirroring `renderPreview` above. */
export async function renderLabelsPreview(picks: LabelPick[], label: LabelOptions): Promise<RenderPreviewOutcome> {
  if (!isTauri()) return { pages: [], warnings: [] };
  const items = await buildLabelItems(picks, { includeQr: label.showQr });
  const payload = buildLabelPayload(items);
  const templateId = label.layout === 'thermal' ? 'label_thermal' : 'label_sheet';
  const { invoke } = await import('@tauri-apps/api/core');
  try {
    const result = await invoke<{ pages: string[]; warnings: RenderPreviewOutcome['warnings'] }>('render_preview', {
      req: { template_source: null, template_id: templateId, payload, options: labelTemplateOptions(label) },
    });
    return { pages: result.pages, warnings: result.warnings ?? [] };
  } catch (err) {
    const diagnostics = Array.isArray(err) ? (err as PreviewError['diagnostics']) : [{ line: null, column: null, severity: 'error', message: String(err) }];
    throw { diagnostics } satisfies PreviewError;
  }
}
