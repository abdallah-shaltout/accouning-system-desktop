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

export interface PdfTemplate {
  id: string;
  name: string;
  kind: DocumentKind;
  /** 'invoice_standard' | 'invoice_simplified' — which built-in .typ this template starts from. */
  baseTemplateId: 'invoice_standard' | 'invoice_simplified';
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
