export interface Tax {
  id: string;
  name: string;
  rate: number;
  type: 'OUTPUT' | 'INPUT';
  isDefault: boolean;
  active: boolean;
}

export type PrinterMode = 'a4' | 'thermal';
export type ThermalWidth = 58 | 80;

export interface StoreSettings {
  storeName: string;
  logo?: string;
  currency: string;
  vatNumber?: string;
  defaultTaxId?: string;
  invoiceNumberPrefix: string;
  printer: {
    mode: PrinterMode;
    thermalWidthMm: ThermalWidth;
  };
  theme: 'light' | 'dark';
  /** Extensions shown on printed documents. */
  address?: string;
  phone?: string;
  commercialRegister?: string;
  receiptFooter?: string;
  /**
   * v2 accounting settings block (docs/v2/02-accounting-review.md B2). `lockDate`: no posting is
   * allowed on/before this date without the `postToClosedPeriod` override, regardless of the
   * fiscal year's own open/closed status. The full closing wizard is Phase 2 — this phase only
   * needs the field + the posting-time check.
   */
  accounting?: {
    lockDate?: string;
    /** v2 (E2): fallback purchase (expense) account id when neither the product nor its category has one. */
    defaultPurchaseAccountId?: string;
  };
  /** Phase 13a — docs/v2/14-platform.md §4. Absent until the backup settings page is opened once. */
  backup?: import('./backup').BackupSettings;
}
