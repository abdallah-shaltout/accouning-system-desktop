/** A labeled phone number — mobile / work / WhatsApp (docs/v2/08-customers-and-suppliers.md §1). */
export interface PartyPhone {
  id: string;
  label: 'mobile' | 'work' | 'whatsapp';
  /** E.164, via AppPhoneInput. */
  number: string;
}

/** A contact person at a company party. */
export interface PartyContact {
  id: string;
  name: string;
  role?: string;
  phone?: string;
  email?: string;
}

/** National address (docs/v2/08 §1 "العنوان الوطني"). */
export interface NationalAddress {
  country?: string;
  city?: string;
  district?: string;
  street?: string;
  buildingNo?: string;
  additionalNo?: string;
  postalCode?: string;
  unitNo?: string;
  /** e.g. "RRRD2929". */
  shortAddress?: string;
}

/**
 * "Balance from an old system" (docs/v2/05-onboarding.md §4). `journalEntryId` is set once posted —
 * its presence is what the form uses to show "مُرحّل" instead of the entry fields, and `locked`
 * (set once a payment has been allocated against it) is what blocks further edits ("corrections via
 * manual journal" from then on).
 */
export interface OpeningBalanceStub {
  amount?: number;
  side?: 'debit' | 'credit';
  asOfDate?: string;
  journalEntryId?: string;
  locked?: boolean;
}

export interface PartyBankInfo {
  bankName?: string;
  iban?: string;
  accountName?: string;
}

interface PartyCommon {
  id: string;
  /** Individual / company (customers only distinguish this today; suppliers are mostly companies). */
  type: 'individual' | 'company';
  name: string;
  nameEn?: string;
  /** Auto "C-0001" / "S-0001". */
  code: string;
  groupId?: string;
  tags?: string[];
  active: boolean;

  /** Legacy single phone, kept for backward compatibility with old data/search; prefer `phones`. */
  phone?: string;
  phones?: PartyPhone[];
  email?: string;
  contacts?: PartyContact[];

  address?: string;
  nationalAddress?: NationalAddress;

  vatNumber?: string;
  crNumber?: string;
  nationalId?: string;

  currency?: string;
  priceListId?: string;
  paymentTermsDays?: number;
  salespersonId?: string;
  branchId?: string;

  bank?: PartyBankInfo;

  openingBalance?: OpeningBalanceStub;

  notes?: string;

  /** "Both roles" (docs/v2/08 §1 "Both roles"): link to the counterpart party record. */
  linkedPartyId?: string;

  /** Computed on read: positive = the customer owes us (customer) / we owe the supplier (supplier). */
  balance: number;
  /** Computed on read: Σ unallocated payment credit sitting on this party (customerAdvances-style). */
  unallocatedCredit?: number;

  createdAt?: string;
  updatedAt?: string;
}

export interface Customer extends PartyCommon {
  creditLimit?: number;
}

export type CustomerInput = Omit<Customer, 'id' | 'balance' | 'code' | 'unallocatedCredit' | 'createdAt' | 'updatedAt'>;

export interface Supplier extends PartyCommon {
  contactPerson?: string;
  /** Default expense account id for suppliers billed to a fixed account (e.g. the electricity company). */
  defaultExpenseAccountId?: string;
}

export type SupplierInput = Omit<Supplier, 'id' | 'balance' | 'code' | 'unallocatedCredit' | 'createdAt' | 'updatedAt'>;

/** Settings → Parties: customer/supplier groups (docs/v2/08 §5). */
export interface PartyGroup {
  id: string;
  kind: 'customer' | 'supplier';
  name: string;
  priceListId?: string;
  paymentTermsDays?: number;
  discountPercent?: number;
}

/** One row of a customer/supplier statement (كشف حساب). */
export interface PartyStatementRow {
  id: string;
  date: string;
  kind: 'invoice' | 'refund' | 'payment' | 'purchaseOrder' | 'purchaseReturn' | 'opening';
  refId: string;
  number: string;
  description: string;
  /** Increases what the party owes us (customer) / what we owe them (supplier). */
  debit: number;
  credit: number;
  balance: number;
}

/** Party activity/history entry (docs/v2/08 §3 "السجل"). */
export interface PartyHistoryEntry {
  id: string;
  partyId: string;
  partyKind: 'customer' | 'supplier';
  date: string;
  message: string;
  userId: string;
}

export type AgingBucketKey = 'current' | '30' | '60' | '90plus';

export interface AgingBucket {
  key: AgingBucketKey;
  label: string;
  total: number;
  documents: { id: string; number: string; date: string; dueDate?: string; outstanding: number }[];
}
