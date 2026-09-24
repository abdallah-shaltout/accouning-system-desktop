// v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md): Branch/CostCenter/Currency/ExchangeRate.
export * from './dimensions';

/**
 * Tax v2 (docs/v2/06-sales-and-pos.md §3, docs/v2/02-accounting-review.md D1/D2). The ZATCA tax
 * categories replace a single store rate:
 * - `S` standard (15% SA) — normal VAT.
 * - `Z` zero-rated (0%) — e.g. qualifying medicines/exports. Zero VAT, but a *different* VAT-return
 *   box than exempt, so it can't be modeled as one "0% tax".
 * - `E` exempt (0%) — needs `exemptionReason` (free text), required by the ZATCA return.
 * - `O` out-of-scope — not part of the VAT system at all (e.g. some government fees).
 *
 * `type`/`isDefault` (v1 fields) are kept so `salesTaxRate()`/`purchaseTaxRate()` in
 * `src/mocks/backend/core.ts` and every reader outside this phase's surface (purchases.ts,
 * PurchaseFormPage.vue, the POS totals legacy path) keep working unchanged — v2 callers
 * (`totals.ts`, the new invoice line shape) read `category`/`direction`/`rate` instead.
 */
export type TaxCategory = 'S' | 'Z' | 'E' | 'O';
export type TaxDirection = 'sales' | 'purchase';

export interface Tax {
  id: string;
  name: string;
  rate: number;
  /** v1: which side of the books this tax posts to. Kept for `core.ts`'s rate lookups. */
  type: 'OUTPUT' | 'INPUT';
  isDefault: boolean;
  active: boolean;
  /** v2: ZATCA tax category — drives the VAT-return box and whether VAT is charged at all. */
  category: TaxCategory;
  /** v2: sales tax (→ vatOutput) or purchase tax (→ vatInput). Mirrors `type` 1:1 (OUTPUT/sales, INPUT/purchase). */
  direction: TaxDirection;
  /** v2: required when `category === 'E'` — the ZATCA exemption reason (free text). */
  exemptionReason?: string;
  /** v2: system role the tax posts to when charged — 'vatOutput' for sales taxes, 'vatInput' for purchase taxes. Informational; `totals.ts`/sales.ts derive the account from `direction` via `accountFor`. */
  accountRole?: 'vatOutput' | 'vatInput';
}

/**
 * Payment methods v2 (docs/v2/09-purchases-payments-expenses.md §2). Each method resolves to its
 * settlement account by *system role* — never a hard-coded account id — so renumbering the CoA
 * never breaks a tender. `branchOverrides` and `feePct` are captured now but stay inert (no
 * per-branch UI, no settlement voucher) until later phases: Phase 9 (branches) and Phase 8
 * (card-settlement vouchers) respectively.
 */
export type PaymentMethodType = 'cash' | 'card' | 'bank_transfer' | 'wallet' | 'credit' | 'store_credit';

export interface PaymentMethod {
  id: string;
  name: string;
  type: PaymentMethodType;
  icon?: string;
  /** System role the method settles to (cash→cash, card→cardClearing, bank_transfer→bank, wallet→walletClearing, credit→receivable). */
  accountRole: 'cash' | 'bank' | 'cardClearing' | 'walletClearing' | 'receivable';
  /** Card/wallet settlement fee %, used later by the Phase 8 card-settlement voucher (posts to `cardFees`). Inert here — only captured. */
  feePct: number;
  /** Whether a reference (last 4 digits, approval code, transfer ref) is expected on the tender. */
  requiresReference?: boolean;
  showInPos: boolean;
  showInPayments: boolean;
  /** Display order (POS tender buttons, the settings list). */
  sortOrder: number;
  /** Inert per-branch account override (docs/v2/09 §2 "account override per branch") — no branch UI until Phase 9. */
  branchOverrides?: { branchId: string; accountId: string }[];
  active: boolean;
  /** System-seeded presets (cash/mada/visa/bank-transfer/stc-pay/credit-sale) can't be deleted, only deactivated. */
  canDelete: boolean;
}

export type PaymentMethodInput = Omit<PaymentMethod, 'id' | 'canDelete'>;

export type PrinterMode = 'a4' | 'thermal';
export type ThermalWidth = 58 | 80;

/**
 * v2 Phase 14 (docs/v2/12-documents-pdf-excel.md §5): native thermal printing
 * settings, additive to the existing `mode`/`thermalWidthMm` mock-phase
 * fields (kept for the browser fallback / test-print preview). Mirrors
 * `src-tauri/src/print/payload.rs`'s `ThermalPrinterConfig` — field names are
 * intentionally the same shape (snake/camel differences aside) so
 * `printService.ts` can pass this through close to verbatim.
 */
export type PrinterConnectionType = 'windows' | 'network';

export interface ThermalPrinterSettings {
  /** Windows queue name from `list_printers`, when `connection === 'windows'`. */
  printerName?: string;
  connection: PrinterConnectionType;
  /** Host/IP, when `connection === 'network'` (always port 9100). */
  host?: string;
  dpi: number;
  cut: boolean;
  openDrawer: boolean;
  copies: number;
}

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
    /** Phase 14: native transport settings for the receipt printer. Optional — absent until the printer settings page is saved once (mirrors `backup?`'s lazy-init pattern). */
    thermal?: ThermalPrinterSettings;
    /** Phase 14: A4 printer is informational-only (system default is used; no printer_name override yet). */
    a4PrinterName?: string;
    /** Phase 14: label printer is a placeholder field — label printing itself is Phase 11b's job. */
    labelPrinterName?: string;
  };
  theme: 'light' | 'dark';
  /**
   * v2 (docs/v2/06-sales-and-pos.md §3, README decision 4): whether entered prices and discounts
   * already include VAT. Default true (Saudi B2C shelf-pricing rule). Every document stores its
   * own copy of this flag at posting time (see `Invoice.pricesIncludeTax`), so changing the store
   * setting later never rewrites historical invoices.
   */
  pricesIncludeTax?: boolean;
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
  /**
   * v2 phase 6 (docs/v2/07-products-and-inventory.md §5, docs/v2/01-personas.md storekeeper):
   * stock-in / write-off / stocktake-loss adjustments whose absolute value is at or above this
   * amount need a manager PIN confirmation before they post. `undefined`/0 = no threshold (always
   * allowed). A simple settings value + PIN dialog, not a full approvals inbox (that's Phase 13b).
   */
  inventoryApprovalThreshold?: number;
  /** v2 phase 6 (§6 "Role matrix editor") — sparse per-role area-access overrides on top of the presets. */
  roleAccessOverrides?: import('@/modules/users/helpers/permissions').RoleAccessOverrides;
  /**
   * v2 phase 10 (docs/v2/11-journal-dashboard-insights.md D1 "Thresholds") — Settings → التوصيات.
   * Sparse: unset fields fall back to `DEFAULT_THRESHOLDS` in `modules/core/services/insightTypes.ts`.
   */
  insightThresholds?: Partial<import('@/modules/core/services/insightTypes').InsightThresholds>;
  /**
   * v2 phase 7 POS settings (docs/v2/06-sales-and-pos.md §1). Kept as flat store-wide toggles
   * (not a per-role permission-matrix entry — no `pos.*`/`sales.*` area exists in
   * `modules/users/helpers/permissions.ts`'s `Area` type, and adding one is a bigger surface than
   * this phase's scope) — `undefined` defaults to the safer choice for each (see each field).
   */
  pos?: {
    /** Cashiers can type a custom price on a line at all. Default true (cashier `maxDiscount` already gates how far below list). */
    overridePrice?: boolean;
    /** Custom price is allowed below the product's cost — otherwise the floor is max(minPrice, cost). Default false. */
    sellBelowCost?: boolean;
    /** A shift must be open to sell from the POS (§5). Default true once the POS is enabled. */
    requireOpenShift?: boolean;
    /** Foreign-currency cash tender in the tender dialog (§1 "Foreign cash"). Default false — no FX-rate settings screen exists before Phase 9. */
    foreignCashEnabled?: boolean;
    foreignCurrency?: string;
    foreignCurrencyRate?: number;
  };
  sales?: {
    /** A return can be recorded without scanning/typing a receipt number (§1 "Return (F7)"). Default false. */
    refundWithoutReceipt?: boolean;
  };
  /**
   * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §4, README decision "invisible until
   * needed"): master switches for the three dimensions. All default OFF/undefined so a fresh company
   * (and every existing screenshot/e2e flow from phases 0-8) looks exactly as before — no switcher,
   * no extra columns, no filters — until the owner turns one on (or onboarding does, per the doc).
   */
  features?: {
    branches?: boolean;
    currencies?: boolean;
    costCenters?: boolean;
  };
  /**
   * v2 phase 5 (docs/v2/05-onboarding.md): the setup wizard's own progress/state. `goLiveDate` is
   * the opening-balance date (locked once the opening entry posts, per §6); `completedStep`/`skipped`
   * drive the dashboard's setup-checklist card; `openingEntryId`/`closingEntryId` are what the
   * "before first use" edit flow reverses+re-posts.
   */
  onboarding?: {
    businessType?: string;
    goLiveDate?: string;
    completedStep?: number;
    /** Step keys the owner skipped (required steps can never appear here). */
    skipped?: string[];
    /** Step keys fully finished — drives the wizard's own progress rail across reloads. */
    done?: string[];
    finishedAt?: string;
    openingEntryId?: string;
    closingEntryId?: string;
    coaTemplate?: 'basic' | 'standard' | 'detailed';
  };
}
