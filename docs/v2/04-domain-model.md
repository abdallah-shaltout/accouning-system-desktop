# 04 — Domain Model v2

This extends `docs/domain_model.md` and replaces it where they conflict. Shapes are the mock DB
tables (`src/mocks/db.ts`) and the service DTOs in `modules/*/types`. Money is in the document
currency unless a field ends in `Base`.

## 1. Shared building blocks

```ts
type Id = string; type ISODate = string; type Money = number;

interface Audit { createdAt: ISODate; createdBy: Id; updatedAt?: ISODate; updatedBy?: Id }

interface DocumentBase extends Audit {
  id: Id; number: string;            // from the NumberSeries of (kind, branch)
  branchId: Id; date: ISODate;
  currency: string; exchangeRate: number;   // base per 1 unit of `currency`; 1 when base
  costCenterId?: Id;                 // default for all lines
  status: string;                    // per document kind
  note?: string; attachmentIds: Id[];
  journalEntryId?: Id;               // set when posted
}

interface NumberSeries { kind: DocKind; branchId: Id | null; prefix: string; next: number;
  padding: number; reset: 'never'|'yearly'|'monthly' }   // e.g. "RYD-INV-2026-00042"

interface Address {                   // Saudi national address, and general
  country: string; city: string; district?: string; street?: string; buildingNo?: string;
  additionalNo?: string; postalCode?: string; unitNo?: string; shortAddress?: string; line?: string }

interface Phone { e164: string; label?: 'mobile'|'work'|'whatsapp' }   // "+966567891234"
```

## 2. Organization

```ts
interface Company { name: string; nameEn?: string; legalType: 'individual'|'company';
  vatNumber?: string; crNumber?: string; address: Address; phones: Phone[]; email?: string;
  website?: string; logoAttachmentId?: Id; country: 'SA'|'EG'|'AE'|string;
  baseCurrency: string; goLiveDate: ISODate; pricesIncludeTax: boolean;
  businessType: 'retail'|'clothing'|'pharmacy'|'supermarket'|'electronics'|'services'|'wholesale' }

interface Branch { id: Id; code: string; name: string; address: Address; phones: Phone[];
  numberingPrefix: string; cashAccountId: Id; defaultBankAccountId?: Id;
  costCenterId: Id;                   // created automatically with the branch
  defaultPriceListId?: Id; receiptHeader?: string; active: boolean }

interface Currency { code: string; nameAr: string; symbol: string; decimals: number; active: boolean }
interface ExchangeRate { currency: string; date: ISODate; rate: number }   // base per 1 unit

interface CostCenter { id: Id; code: string; name: string; parentId?: Id;
  type: 'branch'|'department'|'project'|'other'; branchId?: Id; managerId?: Id;
  budgets: { fiscalYearId: Id; amount: Money }[]; active: boolean }
```

## 3. Tax & payment

```ts
interface Tax { id: Id; name: string; direction: 'OUTPUT'|'INPUT';
  category: 'S'|'Z'|'E'|'O';           // standard / zero-rated / exempt / out of scope (ZATCA codes)
  rate: number; accountId?: Id;         // VAT account; none for E/O
  exemptionReason?: string;             // printed on the invoice for Z/E (required by ZATCA)
  isDefault: boolean; active: boolean }

interface PaymentMethod { id: Id; name: string; icon?: string;
  type: 'cash'|'card'|'bank_transfer'|'wallet'|'cheque'|'credit'|'store_credit'|'other';
  accountId?: Id;                       // cash → the branch cash drawer at runtime; credit → party AR
  branchAccounts?: Record<Id, Id>;      // per-branch override
  feePct?: number; requiresReference: boolean; currency?: string;
  showInPos: boolean; showInPayments: boolean; order: number; active: boolean }
```

## 4. Parties (customers & suppliers)

```ts
interface Party { id: Id; kind: 'customer'|'supplier'; code: string;
  type: 'individual'|'company'; name: string; nameEn?: string;
  phones: Phone[]; email?: string; address: Address;
  vatNumber?: string; crNumber?: string; nationalId?: string;
  contacts: { name: string; role?: string; phone?: Phone; email?: string }[];
  groupId?: Id; tags: string[];
  currency: string;                     // one currency per party (decision 6)
  priceListId?: Id; paymentTermsDays: number; creditLimit?: Money;
  salespersonId?: Id; branchId?: Id;    // "home" branch; null = all branches
  bank?: { name: string; iban: string; accountName?: string };   // mostly suppliers
  defaultExpenseAccountId?: Id;         // suppliers of services/utilities
  opening?: { amount: Money; side: 'DEBIT'|'CREDIT'; date: ISODate; journalEntryId: Id };
  linkedPartyId?: Id;                   // the same business as a supplier/customer (net balance)
  note?: string; attachmentIds: Id[]; active: boolean }
// balance is computed from ledger lines, never stored (review C2)
```

## 5. Catalog & stock

```ts
interface Product { id: Id; type: 'stock'|'service'|'non_stock';
  name: string; nameEn?: string; description?: string; sku: string;
  categoryId?: Id; brandId?: Id; tags: string[]; imageIds: Id[];
  baseUnitId: Id;                       // smallest unit; all stock is in this unit
  units: ProductUnit[];                 // includes the base unit with factor 1
  prices: { priceListId: Id; unitId: Id; price: Money }[];   // price list × unit
  minPrice?: Money;                     // floor for custom price (per base unit)
  saleTaxId: Id; purchaseTaxId: Id;
  accounts?: { revenueId?: Id; cogsId?: Id; purchaseId?: Id };  // override category/settings
  tracking: { batches: boolean; expiry: boolean; serials: false };  // serials: later
  expiryAlertDays?: number; allowNegativeStock: boolean;
  reorder: { branchId: Id; min: number; reorderQty?: number }[];
  preferredSupplierId?: Id; shelfLocation?: string;
  warranty?: { months: number; type: 'manufacturer'|'store' };
  weight?: { value: number; unitId: Id };
  customFields: Record<string, string | number | boolean>;
  parentId?: Id; attributes?: Record<string, string>;   // reserved for variants (07 §7)
  active: boolean;
  // derived / maintained by posting
  stock: { branchId: Id; qty: number }[]; stockValue: Money;   // company-wide value (review A1)
}

interface ProductUnit { unitId: Id; factor: number;   // how many base units in one of this unit
  barcodes: string[]; isDefaultSale: boolean; isDefaultPurchase: boolean }

interface Batch { id: Id; productId: Id; branchId: Id; batchNo: string; expiryDate?: ISODate;
  qty: number; receivedAt: ISODate; purchaseInvoiceId?: Id }

interface StockMovement { id: Id; date: ISODate; productId: Id; branchId: Id; batchId?: Id;
  qtyChange: number;                    // base units
  valueChange: Money;                   // exactly what hit the GL (review A1)
  reason: 'sale'|'credit_note'|'purchase'|'debit_note'|'stock_in'|'write_off'|'stocktake'
        |'transfer_out'|'transfer_in'|'opening';
  refKind: DocKind; refId: Id; refNumber: string; balanceAfter: number }

interface StockTransfer extends DocumentBase { toBranchId: Id;
  status: 'DRAFT'|'SENT'|'RECEIVED'|'REJECTED';
  lines: { productId: Id; unitId: Id; qty: number; baseQty: number; batchId?: Id;
           receivedQty?: number; unitCost: Money }[];
  sentAt?: ISODate; sentBy?: Id; receivedAt?: ISODate; receivedBy?: Id }

interface StockCount extends DocumentBase {        // stocktake v2 (review A5)
  scope: { categoryIds?: Id[]; productIds?: Id[]; shelf?: string };
  status: 'COUNTING'|'REVIEW'|'APPROVED'|'CANCELED';
  lines: { productId: Id; batchId?: Id; systemQtyAtCount: number; countedQty?: number;
           countedAt?: ISODate; countedBy?: Id }[] }

interface StockAdjustment extends DocumentBase {   // stock-in & write-off
  type: 'STOCK_IN'|'WRITE_OFF';
  reason: 'opening'|'owner_contribution'|'gift'|'found'|'damaged'|'expired'|'theft'|'other';
  offsetAccountId?: Id;                // reason = other
  lines: { productId: Id; unitId: Id; qty: number; unitCost: Money; batchId?: Id;
           expiryDate?: ISODate }[] }
```

## 6. Sales

```ts
interface SalesLine { id: Id; productId: Id; name: string;   // snapshot
  unitId: Id; factor: number; qty: number;                     // qty in `unitId`
  listPrice: Money;                    // from the price list (audit for custom price)
  unitPrice: Money;                    // actually charged; inclusive or exclusive per `pricesIncludeTax`
  priceOverrideReason?: string;
  discount?: { kind: 'pct'|'amount'; value: number };
  headerDiscountShare: Money;          // allocated invoice discount (net)
  taxId: Id; taxCategory: 'S'|'Z'|'E'|'O'; taxRate: number;
  net: Money; vat: Money; gross: Money;  // after all discounts
  costValue: Money;                    // COGS posted for this line
  batchAllocations?: { batchId: Id; baseQty: number }[] }

interface Tender { methodId: Id; amount: Money; reference?: string;
  currency?: string; foreignAmount?: Money; rate?: number }   // e.g. a customer pays in USD cash

interface SalesInvoice extends DocumentBase {
  source: 'pos'|'desk';
  invoiceType: 'simplified'|'standard';   // standard when the buyer has a VAT number (B2B)
  customerId?: Id; buyerSnapshot?: Pick<Party,'name'|'vatNumber'|'address'|'crNumber'>;
  salespersonId?: Id; shiftId?: Id; priceListId: Id; dueDate?: ISODate;
  pricesIncludeTax: boolean; lines: SalesLine[];
  headerDiscount?: { kind: 'pct'|'amount'; value: number };
  totals: { net: Money; discount: Money; vat: Money; gross: Money;
            vatByCategory: { category: string; rate: number; net: Money; vat: Money }[];
            vatBase?: Money };          // VAT in SAR when the currency isn't SAR (ZATCA)
  tenders: Tender[]; changeGiven: Money;
  status: 'DRAFT'|'POSTED'|'VOID';     // void = only a draft; posted documents are corrected by credit note
  zatca?: { uuid: string; icv?: number; pih?: string; hash?: string };   // reserved for Phase 2
  terms?: string; templateId?: Id }

interface Quotation extends Omit<SalesInvoice,'tenders'|'changeGiven'|'shiftId'|'journalEntryId'> {
  validUntil: ISODate; status: 'DRAFT'|'SENT'|'ACCEPTED'|'REJECTED'|'EXPIRED'|'CONVERTED';
  convertedInvoiceId?: Id }

interface CreditNote extends DocumentBase { invoiceId: Id; reason: string;
  lines: { invoiceLineId: Id; qty: number; net: Money; vat: Money; gross: Money; costValue: Money }[];
  totals: SalesInvoice['totals']; settlement: { toReceivable: Money; refunds: Tender[]; toCredit: Money } }
```

## 7. Purchases, payments, expenses

```ts
interface PurchaseInvoice extends DocumentBase { supplierId: Id;
  supplierInvoiceNo?: string; supplierInvoiceDate?: ISODate; dueDate?: ISODate;
  status: 'DRAFT'|'ORDERED'|'RECEIVED'|'CANCELED';   // RECEIVED = posted (stock + AP)
  pricesIncludeTax: boolean;
  lines: { productId: Id; unitId: Id; factor: number; qty: number; receivedQty?: number;
           unitPrice: Money; discount?: {kind:'pct'|'amount'; value:number}; taxId: Id;
           net: Money; vat: Money; landedCost: Money; batchNo?: string; expiryDate?: ISODate }[];
  headerDiscount?: {kind:'pct'|'amount'; value:number};
  landedCosts: { description: string; amount: Money; accountId?: Id;
                 allocateBy: 'value'|'qty'; supplierId?: Id }[];
  totals: SalesInvoice['totals']; receivedBy?: Id; receivedAt?: ISODate }

interface DebitNote extends DocumentBase { purchaseInvoiceId: Id; reason: string; /* mirrors CreditNote */ }

interface Payment extends DocumentBase { direction: 'IN'|'OUT';
  partyKind: 'customer'|'supplier'; partyId: Id; methodId: Id; amount: Money;
  reference?: string; allocations: Allocation[]; unallocated: Money }

interface Allocation { id: Id; paymentId?: Id; creditNoteId?: Id;
  targetKind: 'invoice'|'purchase'|'opening'|'debit_note'; targetId: Id;
  amount: Money; date: ISODate; reversedAt?: ISODate }

interface Expense extends DocumentBase { categoryId: Id;   // category → expense account
  amount: Money; taxId?: Id; vat: Money; supplierId?: Id; supplierInvoiceNo?: string;
  paidBy: { methodId: Id } | { onCredit: true };
  recurringTemplateId?: Id; status: 'DRAFT'|'POSTED' }

interface Voucher extends DocumentBase {        // general income / expense / transfer / card settlement
  type: 'receipt'|'disbursement'|'transfer'|'card_settlement'|'owner';
  fromAccountId?: Id; toAccountId?: Id; amount: Money; lines?: JournalLineInput[]; fees?: Money }

interface Shift { id: Id; branchId: Id; userId: Id; cashAccountId: Id; openedAt: ISODate;
  openingFloat: Money; closedAt?: ISODate;
  movements: { kind: 'sale'|'refund'|'pay_in'|'pay_out'|'drop'; amount: Money; refId?: Id; note?: string }[];
  expectedCash?: Money; countedCash?: Money; denominations?: Record<string, number>;
  variance?: Money; status: 'OPEN'|'CLOSED' }
```

## 8. Accounting

```ts
interface JournalEntry extends Audit { id: Id; number: string; date: ISODate; branchId?: Id;
  type: 'SYSTEM'|'MANUAL'|'OPENING'|'CLOSING'|'REVERSAL'|'RECURRING'|'VAT_SETTLEMENT'|'FX_REVAL';
  status: 'DRAFT'|'POSTED';
  description: string; reference?: string;
  sourceRef?: { kind: DocKind; id: Id; number: string };
  lines: JournalLine[]; totalDebit: Money; totalCredit: Money;   // base currency
  attachmentIds: Id[]; templateId?: Id;
  reversal?: { ofId?: Id; byId?: Id; reason: string; date: ISODate } }

interface JournalLine { id: Id; accountId: Id; debit: Money; credit: Money;   // base currency
  description?: string; partyKind?: 'customer'|'supplier'; partyId?: Id;
  branchId?: Id; costCenterId?: Id; currency?: string; amountFc?: Money; rate?: number }

interface JournalTemplate { id: Id; name: string; lines: Omit<JournalLine,'id'>[];
  recurrence?: { every: 'month'|'quarter'|'year'; day: number; nextDate: ISODate; autoPost: boolean } }

interface FiscalYear { id: Id; name: string; start: ISODate; end: ISODate;
  status: 'OPEN'|'CLOSED'; closingEntryId?: Id }
// settings.accounting.lockDate: ISODate | null
```

## 9. Platform

```ts
interface Attachment { id: Id; ownerKind: string; ownerId: Id; name: string; mime: string;
  size: number; blobKey: string;        // IndexedDB blob store key (mock), file path (real)
  thumbnailKey?: string; createdAt: ISODate; createdBy: Id }

interface PrintTemplate { id: Id; docKind: DocKind | 'label' | 'statement';
  name: string; engine: 'typst'; base: 'classic'|'modern'|'compact'|'thermal'|'label';
  options: TemplateOptions;             // see 12-documents-pdf-excel.md §3
  source?: string;                      // advanced: custom Typst source
  paper: { size: 'A4'|'A5'|'80mm'|'58mm'|`${number}x${number}mm`; margins: [number,number,number,number] };
  isDefault: boolean; branchId?: Id }

interface Insight { id: string; ruleId: string; severity: 'critical'|'warning'|'info'|'positive';
  audience: Role[]; title: string; detail: string; metric?: { label: string; value: string };
  action?: { label: string; to?: string; command?: string };
  entityRefs: { kind: string; id: Id }[]; createdAt: ISODate }
interface InsightState { userId: Id; insightKey: string; dismissedAt?: ISODate; snoozedUntil?: ISODate }

interface UserPreferences { theme: 'light'|'dark'|'system'; numerals: 'latn'|'arab';
  fontFamily: 'cairo'|'ibm-plex-arabic'|'tajawal'|'noto-naskh'; fontScale: 0.9|1|1.1|1.2|1.3;
  density: 'comfortable'|'compact'; dateFormat: 'dd/mm/yyyy'|'yyyy-mm-dd';
  showHijri: boolean; firstDayOfWeek: 0|1|6; tablePageSize: 25|50|100; zebraRows: boolean;
  reduceMotion: boolean; sidebarCollapsed: boolean; homeBranchId?: Id; posFavorites: Id[] }

interface SetupState { steps: { key: SetupStep; status: 'pending'|'done'|'skipped'; data?: unknown }[];
  completedAt?: ISODate; mode: 'fresh'|'demo' }

interface BackupManifest { app: string; appVersion: string; schemaVersion: number;
  createdAt: ISODate; company: string; counts: Record<string, number>;
  checksum: string; encrypted: boolean; kind: 'manual'|'auto'|'pre-restore' }
```
