/**
 * Template designer types (Phase 11a, docs/v2/12-documents-pdf-excel.md §3).
 * `TemplateOptions` mirrors the shape the Rust side expects as `opts.json`
 * (src-tauri/src/pdf/payload.rs's `TemplateOptionsPeek` reads a subset;
 * `src-tauri/templates/lib.typ` reads the rest by key with `.at(..., default: ...)`,
 * so unknown/future keys are safe to add on either side without breaking the other).
 *
 * Only the invoice document kind is wired up in Phase 11a. `kind` is kept as
 * a field (not narrowed to `'invoice'`) so the designer shell doesn't need
 * rework when Phase 11b adds the other 9 kinds.
 */

export type DocumentKind =
  | 'invoice'
  | 'quotation'
  | 'creditNote'
  | 'debitNote'
  | 'purchaseOrder'
  | 'voucher'
  | 'statement'
  | 'zReport'
  | 'transferNote'
  | 'report';

export type LogoPosition = 'start' | 'center' | 'end';
export type LogoSize = 's' | 'm' | 'l';
export type PaperSize = 'a4' | 'a5' | 'letter' | '80mm' | '58mm';
export type FontFamilyOption = 'Cairo' | 'Noto Naskh Arabic' | 'IBM Plex Sans Arabic' | 'Tajawal';

export interface TemplateColumn {
  key: 'index' | 'sku' | 'barcode' | 'name' | 'unit' | 'qty' | 'price' | 'discount' | 'net' | 'vatRate' | 'vat' | 'total';
  label: string;
  visible: boolean;
}

export const DEFAULT_COLUMNS: TemplateColumn[] = [
  { key: 'index', label: '#', visible: true },
  { key: 'name', label: 'الصنف', visible: true },
  { key: 'qty', label: 'الكمية', visible: true },
  { key: 'price', label: 'السعر', visible: true },
  { key: 'discount', label: 'الخصم', visible: false },
  { key: 'net', label: 'صافي', visible: false },
  { key: 'vatRate', label: 'الضريبة %', visible: true },
  { key: 'vat', label: 'الضريبة', visible: true },
  { key: 'total', label: 'الإجمالي', visible: true },
];

/**
 * The subset of docs/v2/12-documents-pdf-excel.md §3's `TemplateOptions`
 * implemented in Phase 11a. See the designer page's top-of-file comment for
 * which groups were skipped and why (stamp/signature uploads, watermark,
 * party-box field toggles, column reorder-by-drag, per-branch defaults).
 */
export interface TemplateOptions {
  accentColor: string;
  logoPosition: LogoPosition;
  logoSize: LogoSize;
  fontFamily: FontFamilyOption;
  fontSize: number;
  header: {
    showCompanyName: boolean;
    showAddress: boolean;
    showVatNumber: boolean;
    showCommercialRegister: boolean;
    showPhone: boolean;
    showEmail: boolean;
    showWebsite: boolean;
    title: string;
    titleEn: string;
  };
  columns: TemplateColumn[];
  totals: {
    showAmountInWords: boolean;
    showBalance: boolean;
  };
  footer: {
    terms: string;
    bankDetails: string;
    showSignatureLines: boolean;
    thankYouLine: string;
    showPageNumbers: boolean;
  };
  qr: {
    position: 'start' | 'center' | 'end';
    size: string;
  };
  paper: PaperSize;
}

export function defaultTemplateOptions(): TemplateOptions {
  return {
    accentColor: '#4f46e5',
    logoPosition: 'start',
    logoSize: 'm',
    fontFamily: 'Cairo',
    fontSize: 10,
    header: {
      showCompanyName: true,
      showAddress: true,
      showVatNumber: true,
      showCommercialRegister: true,
      showPhone: true,
      showEmail: false,
      showWebsite: false,
      title: 'فاتورة ضريبية',
      titleEn: 'TAX INVOICE',
    },
    columns: DEFAULT_COLUMNS.map((c) => ({ ...c })),
    totals: {
      showAmountInWords: true,
      showBalance: false,
    },
    footer: {
      terms: '',
      bankDetails: '',
      showSignatureLines: false,
      thankYouLine: 'شكراً لتعاملكم معنا',
      showPageNumbers: true,
    },
    qr: {
      position: 'center',
      size: '3cm',
    },
    paper: 'a4',
  };
}

/**
 * Which built-in `.typ` file (src-tauri/src/pdf/render.rs's `builtin_template_source`) this
 * template starts from. Phase 11a shipped the two invoice variants; Phase 11b adds one per new
 * document kind (docs/v2/12-documents-pdf-excel.md §3's document-kinds table) plus the two label
 * layouts (sheet grid / one-per-page thermal — §4).
 */
export type BaseTemplateId =
  | 'invoice_standard'
  | 'invoice_simplified'
  | 'quotation'
  | 'credit_note'
  | 'debit_note'
  | 'purchase_order'
  | 'voucher'
  | 'statement'
  | 'z_report'
  | 'transfer_note'
  | 'generic_report'
  | 'label_sheet'
  | 'label_thermal';

export interface PdfTemplate {
  id: string;
  name: string;
  kind: DocumentKind;
  baseTemplateId: BaseTemplateId;
  options: TemplateOptions;
  /** Present once the user opens the advanced tab and edits the raw source; null = use the built-in template + options. */
  customSource: string | null;
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface TemplateExport {
  schema: 'pdf-template-v1';
  template: Omit<PdfTemplate, 'id' | 'isDefault' | 'createdAt' | 'updatedAt'>;
}

// =================================================================================================
// Labels (Phase 11b, docs/v2/07-products-and-inventory.md §6 / docs/v2/12-documents-pdf-excel.md
// §3-4). Mirrors `lib.typ`'s `opts.label` dict read by `label_sheet.typ`/`label_thermal.typ`
// (src-tauri/templates/label_sheet.typ's top-of-file comment documents the exact keys).
// =================================================================================================

export type LabelLayoutKind = 'sheet' | 'thermal';

export interface LabelOptions {
  layout: LabelLayoutKind;
  widthMm: number;
  heightMm: number;
  /** Sheet layout only — ignored (both stay 1) for a thermal one-per-page layout. */
  cols: number;
  rows: number;
  marginTopMm: number;
  marginLeftMm: number;
  gutterXMm: number;
  gutterYMm: number;
  /** 1-based, row-major — lets a half-used A4 sheet be reused by skipping already-printed cells. */
  startCell: number;
  showStoreName: boolean;
  showPrice: boolean;
  showSku: boolean;
  showBatch: boolean;
  showBarcode: boolean;
  showQr: boolean;
  fontFamily: FontFamilyOption;
}

/** A named starting point in the builder's "label template" picker (docs/v2/07 §6 "Label templates: Sheets ... Thermal label printers"). */
export interface LabelPreset {
  id: string;
  name: string;
  options: LabelOptions;
}

export const LABEL_PRESETS: LabelPreset[] = [
  {
    id: 'a4_3x8_70x37',
    name: 'A4 — 3×8 (70×37 مم)',
    options: { layout: 'sheet', widthMm: 70, heightMm: 37, cols: 3, rows: 8, marginTopMm: 10, marginLeftMm: 8, gutterXMm: 2, gutterYMm: 0, startCell: 1, showStoreName: true, showPrice: true, showSku: false, showBatch: false, showBarcode: true, showQr: false, fontFamily: 'Cairo' },
  },
  {
    id: 'a4_4x10_48x25',
    name: 'A4 — 4×10 (48.5×25.4 مم)',
    options: { layout: 'sheet', widthMm: 48.5, heightMm: 25.4, cols: 4, rows: 10, marginTopMm: 8, marginLeftMm: 6, gutterXMm: 2, gutterYMm: 0, startCell: 1, showStoreName: false, showPrice: true, showSku: false, showBatch: false, showBarcode: true, showQr: false, fontFamily: 'Cairo' },
  },
  {
    id: 'thermal_40x25',
    name: 'حراري — 40×25 مم',
    options: { layout: 'thermal', widthMm: 40, heightMm: 25, cols: 1, rows: 1, marginTopMm: 2, marginLeftMm: 2, gutterXMm: 0, gutterYMm: 0, startCell: 1, showStoreName: false, showPrice: true, showSku: false, showBatch: false, showBarcode: true, showQr: false, fontFamily: 'Cairo' },
  },
  {
    id: 'thermal_50x30',
    name: 'حراري — 50×30 مم',
    options: { layout: 'thermal', widthMm: 50, heightMm: 30, cols: 1, rows: 1, marginTopMm: 2, marginLeftMm: 2, gutterXMm: 0, gutterYMm: 0, startCell: 1, showStoreName: true, showPrice: true, showSku: false, showBatch: false, showBarcode: true, showQr: false, fontFamily: 'Cairo' },
  },
  {
    id: 'thermal_58x40',
    name: 'حراري — 58×40 مم',
    options: { layout: 'thermal', widthMm: 58, heightMm: 40, cols: 1, rows: 1, marginTopMm: 2, marginLeftMm: 2, gutterXMm: 0, gutterYMm: 0, startCell: 1, showStoreName: true, showPrice: true, showSku: true, showBatch: true, showBarcode: true, showQr: false, fontFamily: 'Cairo' },
  },
];

export function defaultLabelOptions(): LabelOptions {
  return { ...LABEL_PRESETS[0].options };
}
