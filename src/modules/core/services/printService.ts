/**
 * `printService`: the TS-side entry point for native thermal printing
 * (Phase 14, docs/v2/12-documents-pdf-excel.md §5). Builds a receipt
 * `DocumentPayload` (same shape `pdfService.ts`'s `buildInvoicePayload`
 * builds — reused here rather than duplicated), fires the Rust
 * `print_thermal_receipt` command **without awaiting the print itself**
 * (the command returns immediately after spawning the job; the real
 * success/failure arrives later as a `print://receipt-result` event), and
 * drives the toast + reprint + PDF-fallback UX per §5: "printing is
 * asynchronous and never blocks the next sale... On failure: a toast +
 * إعادة الطباعة (reprint) + the PDF fallback."
 *
 * Browser dev-mode fallback: mirrors `pdfService.ts` — outside Tauri,
 * `printReceipt` returns `{ ok: false }` immediately so the caller (the POS
 * checkout's receipt button) falls back to the existing `/print/invoices/:id`
 * browser thermal-HTML route.
 */
import { isTauri } from '@tauri-apps/api/core';
import { encode as uqrEncode } from 'uqr';
import { formatDateTime, formatMoney, formatNumber } from '@/modules/core/helpers/format';
import { tafqit } from '@/modules/core/helpers/tafqit';
import { useToast } from '@/modules/core/controllers/useToast';
import { getInvoicePrintData } from '@/modules/invoices/services/invoiceService';
import { round2 } from '@/modules/invoices/helpers/totals';
import { zatcaQrBase64 } from '@/modules/invoices/helpers/zatcaQr';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import type { ThermalPrinterSettings, ThermalWidth } from '@/modules/settings/types';
import type { DocumentPayload } from './pdfService';
import * as pdfService from './pdfService';

import { wrap } from '@/modules/diagnostics/services/defineService';

/** Mirrors `src-tauri/src/print/payload.rs`'s `ThermalPrinterConfig`. */
interface ThermalPrinterConfigWire {
  printer_name: string | null;
  connection: 'windows' | 'network';
  host: string | null;
  width: '58mm' | '80mm';
  dpi: number;
  cut: boolean;
  open_drawer: boolean;
  copies: number;
}

interface PrintReceiptRequestWire {
  payload: DocumentPayload;
  printer: ThermalPrinterConfigWire;
  options: Record<string, unknown>;
}

export interface PrinterInfo {
  name: string;
  is_default: boolean;
}

/**
 * Same QR SVG builder as `pdfService.ts`'s private `qrSvg` (duplicated
 * intentionally rather than exporting a second symbol from that file, to
 * keep this phase's diff scoped to files it owns).
 */
function qrSvg(value: string, sizeMm = 28): string {
  const qr = uqrEncode(value, { ecc: 'M', border: 2 });
  let d = '';
  qr.data.forEach((row, y) =>
    row.forEach((on, x) => {
      if (on) d += `M${x},${y}h1v1h-1z`;
    }),
  );
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${qr.size} ${qr.size}" width="${sizeMm}mm" height="${sizeMm}mm" shape-rendering="crispEdges"><rect width="${qr.size}" height="${qr.size}" fill="#fff"/><path d="${d}" fill="#000"/></svg>`;
}

/** Builds the receipt `DocumentPayload` for a sale, in the same shape `pdfService`'s `render()` builds for the A4 route. */
async function buildReceiptPayload(saleId: string): Promise<DocumentPayload> {
  const data = await getInvoicePrintData(saleId);
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

  const lines = inv.lines.map((l) => ({
    name: l.name,
    qty: formatNumber(l.qty),
    price: formatMoney(l.price),
    net: formatMoney(round2(l.qty * l.price - l.discount)),
  }));

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
    party: data.customer ? { name: data.customer.name, vatNumber: data.customer.vatNumber ?? null, address: data.customer.address ?? null, phone: data.customer.phone ?? null } : null,
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

function defaultThermalConfig(): ThermalPrinterSettings {
  return { connection: 'windows', dpi: 203, cut: true, openDrawer: false, copies: 1 };
}

function buildRequest(payload: DocumentPayload, width: ThermalWidth, thermal: ThermalPrinterSettings, isCashSale: boolean, receiptFooter?: string, sample = false): PrintReceiptRequestWire {
  return {
    payload: sample ? { ...payload } : payload,
    printer: {
      printer_name: thermal.connection === 'windows' ? thermal.printerName ?? null : null,
      connection: thermal.connection,
      host: thermal.connection === 'network' ? thermal.host ?? null : null,
      width: width === 58 ? '58mm' : '80mm',
      dpi: thermal.dpi,
      cut: thermal.cut,
      // Only kick the drawer on cash sales (§5: "optional cash-drawer kick
      // (on cash sales)") — never for card/credit/etc, and never on a
      // settings-page test print.
      open_drawer: thermal.openDrawer && isCashSale && !sample,
      copies: thermal.copies,
    },
    options: receiptFooter ? { receiptFooter } : {},
  };
}

let jobSeq = 0;
function nextJobId(): string {
  return `print-${Date.now()}-${++jobSeq}`;
}

export interface PrintReceiptOutcome {
  /** false when running outside Tauri (browser dev mode) — caller should fall back to the `/print/invoices/:id` route. */
  ok: boolean;
  jobId?: string;
}

/**
 * Fires a native thermal print for a completed sale. Never awaits the actual
 * print — `invoke('print_thermal_receipt', ...)` resolves as soon as the job
 * is *spawned* on the Rust side (see `print::commands::print_thermal_receipt`),
 * so this returns almost immediately regardless of printer speed/health, and
 * the POS can move on to the next sale right away.
 *
 * Success/failure is reported later via the `print://receipt-result` Tauri
 * event (wired once by `initPrintResultListener`, below) which shows a toast
 * and, on failure, an "إعادة الطباعة" action that calls this again with the
 * same `saleId`, plus a PDF fallback offer.
 */
export const printReceipt = wrap('core.printReceipt', async function printReceipt(saleId: string, isCashSale: boolean): Promise<PrintReceiptOutcome> {
  if (!isTauri()) return { ok: false };

  const settings = useSettingsStore();
  const printer = settings.settings?.printer;
  const width: ThermalWidth = printer?.thermalWidthMm ?? 80;
  const thermal = printer?.thermal ?? defaultThermalConfig();

  const payload = await buildReceiptPayload(saleId);
  const req = buildRequest(payload, width, thermal, isCashSale, settings.settings?.receiptFooter);
  const jobId = nextJobId();

  const { invoke } = await import('@tauri-apps/api/core');
  // The command itself resolves once the job is *spawned*, not once it's
  // done printing — see the doc comment above.
  await invoke('print_thermal_receipt', { req, jobId });

  pendingSaleForJob.set(jobId, saleId);
  return { ok: true, jobId };
});

/** Synchronous-style test print for the settings page's "اختبار الطباعة" button — awaits the actual print result rather than going through the async event. */
export const testPrint = wrap('core.testPrint', async function testPrint(thermal: ThermalPrinterSettings, width: ThermalWidth): Promise<{ ok: boolean; error?: string }> {
  if (!isTauri()) return { ok: false, error: 'غير متاح خارج تطبيق سطح المكتب' };
  const sample = await getInvoicePrintData('sample');
  const payload = await buildReceiptPayloadFromData(sample);
  const req = buildRequest(payload, width, thermal, false, sample.settings.receiptFooter, true);
  const { invoke } = await import('@tauri-apps/api/core');
  try {
    await invoke('print_test_receipt', { req });
    return { ok: true };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
});

async function buildReceiptPayloadFromData(data: Awaited<ReturnType<typeof getInvoicePrintData>>): Promise<DocumentPayload> {
  const inv = data.invoice;
  const s = data.settings;
  const qrValue = zatcaQrBase64({ sellerName: s.storeName, vatNumber: s.vatNumber ?? '', timestamp: inv.date, invoiceTotal: inv.grandTotal, vatTotal: inv.taxAmount });
  const lines = inv.lines.map((l) => ({ name: l.name, qty: formatNumber(l.qty), price: formatMoney(l.price), net: formatMoney(round2(l.qty * l.price - l.discount)) }));
  return {
    document: { kind: 'invoice', number: inv.number, date: formatDateTime(inv.date), titleAr: 'فاتورة ضريبية مبسطة', titleEn: 'SIMPLIFIED TAX INVOICE' },
    company: { name: s.storeName, address: s.address ?? null, phone: s.phone ?? null, email: null, website: null, vatNumber: s.vatNumber ?? null, commercialRegister: s.commercialRegister ?? null, logo: s.logo ?? null },
    party: null,
    lines,
    totals: { subtotal: formatMoney(inv.subTotal), discount: null, vat: formatMoney(inv.taxAmount), grand: formatMoney(inv.grandTotal), paid: null, remaining: null, amountInWords: tafqit(inv.grandTotal), previousBalance: null, currentBalance: null },
    qr: qrSvg(qrValue),
    logo: s.logo ?? null,
  };
}

const pendingSaleForJob = new Map<string, string>();
let listenerInitialized = false;

/**
 * Wires the `print://receipt-result` listener once (call from app startup,
 * e.g. `App.vue`'s `onMounted`, same place other one-time Tauri listeners are
 * set up). Shows a success toast, or on failure a sticky error toast with
 * "إعادة الطباعة" (reprint, retries the same sale) and "طباعة PDF بدلاً منها"
 * (PDF fallback via `pdfService`) actions — per §5's asynchronous
 * printing + reprint-on-failure + PDF-fallback requirement.
 */
export const initPrintResultListener = wrap('core.initPrintResultListener', async function initPrintResultListener(): Promise<void> {
  if (listenerInitialized || !isTauri()) return;
  listenerInitialized = true;
  const { listen } = await import('@tauri-apps/api/event');
  await listen<{ job_id: string; ok: boolean; error: string | null }>('print://receipt-result', (event) => {
    const { job_id, ok, error } = event.payload;
    const saleId = pendingSaleForJob.get(job_id);
    pendingSaleForJob.delete(job_id);
    const toast = useToast();
    if (ok) {
      toast.success('تمت طباعة الإيصال');
      return;
    }
    const actions = saleId
      ? [
          { label: 'إعادة الطباعة', onClick: () => void printReceipt(saleId, false) },
          {
            label: 'طباعة PDF بدلاً منها',
            onClick: () => void pdfService.renderAndSave('invoice', saleId, `${saleId}.pdf`),
          },
        ]
      : [];
    toast.errorWithActions('تعذرت طباعة الإيصال', error ?? undefined, actions);
  });
});
